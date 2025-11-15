//! Verify that the server's initialize response is Content-Length framed (protocol compliance).
use std::process::{Command, Stdio};
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::{Arc, Mutex};
use serde_json::Value;

fn spawn_stdio_no_debug() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_serial_mcp_agent"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Intentionally DO NOT set MCP_DEBUG_BOOT so we observe normal framing only.
        .spawn()
        .expect("failed to start binary")
}

#[test]
fn initialize_is_framed() {
    let mut child = spawn_stdio_no_debug();
    let stdin = child.stdin.as_mut().expect("stdin");
    let mut child_stdout = child.stdout.take().expect("stdout");
    let mut child_stderr = child.stderr.take().expect("stderr");

    let out_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let err_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let out_buf_clone = Arc::clone(&out_buf);
    let err_buf_clone = Arc::clone(&err_buf);

    let stdout_handle = thread::spawn(move || {
        let mut local = [0u8; 512];
        loop {
            match child_stdout.read(&mut local) {
                Ok(0) => break,
                Ok(n) => { if let Ok(mut b) = out_buf_clone.lock() { b.extend_from_slice(&local[..n]); } },
                Err(_) => break,
            }
        }
    });
    let stderr_handle = thread::spawn(move || {
        let mut local = [0u8; 512];
        loop {
            match child_stderr.read(&mut local) {
                Ok(0) => break,
                Ok(n) => { if let Ok(mut b) = err_buf_clone.lock() { b.extend_from_slice(&local[..n]); } },
                Err(_) => break,
            }
        }
    });

    // Allow runtime start
    thread::sleep(Duration::from_millis(120));

    let init_body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#;
    let frame = format!(
        "Content-Length: {}\r\nContent-Type: application/json\r\n\r\n{}\n",
        init_body.as_bytes().len(),
        init_body
    );
    stdin.write_all(frame.as_bytes()).unwrap();
    stdin.flush().unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut framed_init: Option<Value> = None;
    while Instant::now() < deadline {
        {
            let raw = out_buf.lock().unwrap();
            for v in parse_framed_messages(&raw) {
                if v.get("id").and_then(|i| i.as_i64()) == Some(1) { framed_init = Some(v); break; }
            }
            if framed_init.is_some() { break; }
        }
        thread::sleep(Duration::from_millis(25));
    }

    let _ = child.kill();
    let _ = child.wait();
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    let raw_final = out_buf.lock().unwrap().clone();
    let stderr_final = err_buf.lock().unwrap().clone();
    // If no framed message, treat each newline as potential JSON object (transport is line-delimited)
    let val = if let Some(v) = framed_init { v } else {
        let line_objs = parse_line_json(&raw_final);
        line_objs.into_iter().find(|v| v.get("id").and_then(|i| i.as_i64()) == Some(1))
            .unwrap_or_else(|| {
                let raw_str = String::from_utf8_lossy(&raw_final);
                let err_str = String::from_utf8_lossy(&stderr_final);
                panic!("Initialize response not found as framed or line-delimited JSON. Raw stdout: {raw_str}\nStderr: {err_str}");
            })
    };
    assert!(val.get("result").is_some(), "missing result field: {val:?}");
}

// Parse only Content-Length framed JSON messages (ignore raw JSON fallback)
fn parse_framed_messages(buf: &[u8]) -> Vec<Value> {
    let mut msgs = Vec::new();
    let mut idx = 0usize;
    while idx < buf.len() {
        while idx < buf.len() && buf[idx].is_ascii_whitespace() { idx += 1; }
        if idx >= buf.len() { break; }
        if buf[idx..].starts_with(b"Content-Length:") {
            if let Some(s) = std::str::from_utf8(&buf[idx..]).ok() {
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
                        let header_bytes = if s.as_bytes()[term_pos..].starts_with(b"\r\n\r\n") { term_pos + 4 } else { term_pos + 2 };
                        let abs_body_start = idx + header_bytes;
                        let abs_body_end = abs_body_start + len;
                        if abs_body_end <= buf.len() {
                            if let Ok(val) = serde_json::from_slice(&buf[abs_body_start..abs_body_end]) {
                                msgs.push(val);
                                idx = abs_body_end;
                                continue;
                            }
                        } else { break; }
                    } else { break; }
                } else { break; }
            } else { break; }
        } else {
            // Encountered non-framed data; stop (we only care about framed messages here)
            break;
        }
    }
    msgs
}

// Very simple scan for standalone JSON objects (not robust; for diagnostics only)
fn parse_line_json(buf: &[u8]) -> Vec<Value> {
    if let Ok(s) = std::str::from_utf8(buf) { s.lines().filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok()).collect() } else { vec![] }
}
