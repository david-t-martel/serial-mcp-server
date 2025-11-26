//! Basic smoke tests for stdio command processing logic.
use serde_json::Value;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn spawn_stdio() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_serial_mcp_agent"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("MCP_DEBUG_BOOT", "1")
        .spawn()
        .expect("failed to start binary")
}

#[test]
fn initialize_round_trip() {
    let mut child = spawn_stdio();
    let stdin = child.stdin.as_mut().expect("stdin");

    // Take stdout/stderr so we can read them on background threads without blocking the main timeout loop.
    let mut child_stdout = child.stdout.take().expect("stdout");
    let mut child_stderr = child.stderr.take().expect("stderr");
    let out_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let err_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let out_buf_clone = Arc::clone(&out_buf);
    let err_buf_clone = Arc::clone(&err_buf);

    // Reader thread for stdout
    let stdout_handle = thread::spawn(move || {
        let mut local = [0u8; 512];
        loop {
            match child_stdout.read(&mut local) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    if let Ok(mut b) = out_buf_clone.lock() {
                        b.extend_from_slice(&local[..n]);
                    }
                }
                Err(_) => break,
            }
        }
    });
    // Reader thread for stderr
    let stderr_handle = thread::spawn(move || {
        let mut local = [0u8; 512];
        loop {
            match child_stderr.read(&mut local) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut b) = err_buf_clone.lock() {
                        b.extend_from_slice(&local[..n]);
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Give the async runtime a brief moment to initialize its read loop
    thread::sleep(Duration::from_millis(120));

    // Send MCP initialize request using Content-Length framed protocol used by rust-mcp-sdk
    // NOTE: Content-Length must be the exact number of bytes in the JSON body ONLY (no trailing newline)
    let init_body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#;
    let frame = format!(
        "Content-Length: {}\r\nContent-Type: application/json\r\n\r\n{}\n",
        init_body.len(),
        init_body
    );
    stdin.write_all(frame.as_bytes()).unwrap(); // trailing newline not counted in length (outside body)
    stdin.flush().unwrap();

    // Poll buffers until we parse a response or timeout.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut parsed = None;
    while Instant::now() < deadline {
        {
            let raw_locked = out_buf.lock().unwrap();
            for msg in parse_messages(&raw_locked) {
                let is_init = msg.get("id").and_then(|i| i.as_i64()) == Some(1)
                    && msg.get("result").is_some();
                if is_init {
                    parsed = Some(msg);
                    break;
                }
            }
            if parsed.is_some() {
                break;
            }
        }
        thread::sleep(Duration::from_millis(25));
    }

    let _ = child.kill();
    let _ = child.wait();
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();
    let raw_final = out_buf.lock().unwrap().clone();
    let err_final = err_buf.lock().unwrap().clone();
    let msg = String::from_utf8_lossy(&raw_final).to_string();
    let stderr_msg = String::from_utf8_lossy(&err_final);
    let v = parsed.as_ref().unwrap_or_else(|| {
        panic!("No framed response parsed within timeout. Raw: {msg}\nStderr: {stderr_msg}")
    });
    // Basic assertions on initialize response structure
    assert_eq!(
        v.get("id").and_then(|i| i.as_i64()),
        Some(1),
        "response id mismatch: {v:?}"
    );
    let result = v.get("result").expect("missing result");
    assert!(
        result.get("serverInfo").is_some(),
        "missing serverInfo field: {v:?}"
    );
}

// Attempt to parse a single Content-Length framed JSON message from the accumulated buffer.
fn parse_messages(buf: &[u8]) -> Vec<Value> {
    let mut msgs = Vec::new();
    let mut idx = 0usize;
    while idx < buf.len() {
        // Skip leading whitespace/newlines between frames.
        while idx < buf.len() && buf[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if idx >= buf.len() {
            break;
        }
        // Attempt framed parse (Content-Length)
        if buf[idx..].starts_with(b"Content-Length:") {
            // Safe UTF8 for header region (only ASCII expected)
            if let Ok(s) = std::str::from_utf8(&buf[idx..]) {
                if let Some(term_pos) = s.find("\r\n\r\n").or_else(|| s.find("\n\n")) {
                    let header = &s[..term_pos];
                    let mut content_length: Option<usize> = None;
                    for line in header.lines() {
                        let parts: Vec<_> = line.splitn(2, ':').collect();
                        if parts.len() == 2 && parts[0].eq_ignore_ascii_case("Content-Length") {
                            content_length = parts[1].trim().parse().ok();
                        }
                    }
                    if let Some(len) = content_length {
                        let header_bytes = if s.as_bytes()[term_pos..].starts_with(b"\r\n\r\n") {
                            term_pos + 4
                        } else {
                            term_pos + 2
                        };
                        let abs_body_start = idx + header_bytes;
                        let abs_body_end = abs_body_start + len;
                        if abs_body_end <= buf.len() {
                            if let Ok(val) =
                                serde_json::from_slice(&buf[abs_body_start..abs_body_end])
                            {
                                msgs.push(val);
                                idx = abs_body_end;
                                continue;
                            }
                        } else {
                            // Incomplete body; stop further attempts
                            break;
                        }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        // Fallback: try raw JSON object (newline delimited)
        if buf[idx] == b'{' {
            let mut end = idx + 1;
            while end <= buf.len() {
                if let Ok(val) = serde_json::from_slice(&buf[idx..end]) {
                    msgs.push(val);
                    idx = end;
                    break;
                }
                end += 1;
            }
            if end > buf.len() {
                break;
            }
            continue;
        }
        // Unknown leading bytes; stop to avoid infinite loop
        break;
    }
    msgs
}
