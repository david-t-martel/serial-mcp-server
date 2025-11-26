//! Stress test rapid append_message calls over the MCP stdio interface to ensure ordering.
use serde_json::Value;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn spawn() -> std::process::Child {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_serial_mcp_agent"));
    // Disable heartbeat for this stress test to simplify early framing expectations
    cmd.env("MCP_DISABLE_HEARTBEAT", "1");
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.spawn().expect("spawn serial_mcp_agent")
}

// Basic framed write helper
fn write_frame(stdin: &mut std::process::ChildStdin, body: &str) {
    let frame = format!(
        "Content-Length: {}\r\nContent-Type: application/json\r\n\r\n{}\n",
        body.len(),
        body
    );
    stdin.write_all(frame.as_bytes()).unwrap();
    stdin.flush().unwrap();
}

#[test]
fn stress_append_ordering() {
    let mut child = spawn();
    let stdin = child.stdin.as_mut().expect("stdin");
    let mut out = child.stdout.take().expect("stdout");
    let mut err = child.stderr.take().expect("stderr");
    let out_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let err_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let out_clone = out_buf.clone();
    let err_clone = err_buf.clone();
    let th_out = thread::spawn(move || {
        let mut local = [0u8; 2048];
        loop {
            match out.read(&mut local) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut b) = out_clone.lock() {
                        b.extend_from_slice(&local[..n]);
                    }
                }
                Err(_) => break,
            }
        }
    });
    let th_err = thread::spawn(move || {
        let mut local = [0u8; 1024];
        loop {
            match err.read(&mut local) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut b) = err_clone.lock() {
                        b.extend_from_slice(&local[..n]);
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Initialize
    thread::sleep(Duration::from_millis(120));
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#;
    write_frame(stdin, init);

    // Wait for initialize response
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if has_response(&out_buf, 1) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    if !has_response(&out_buf, 1) {
        // Dump buffers for diagnostics
        let out_snapshot = out_buf.lock().unwrap().clone();
        let err_snapshot = err_buf.lock().unwrap().clone();
        eprintln!("STDOUT RAW:\n{}", String::from_utf8_lossy(&out_snapshot));
        eprintln!("STDERR RAW:\n{}", String::from_utf8_lossy(&err_snapshot));
        panic!("did not receive initialize response");
    }

    // Create session
    // Use current MCP tools invocation method name 'tools/call' (was previously 'callTool').
    let create_body = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_session","arguments":{"device_id":"stress"}}}"#;
    write_frame(stdin, create_body);
    // Use a fresh deadline for create_session (don't reuse the initialize window or we may timeout early)
    let deadline_create = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline_create {
        if has_response(&out_buf, 2) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let session_id = extract_session_id(&out_buf).unwrap_or_else(|| {
        let out_snapshot = out_buf.lock().unwrap().clone();
        let err_snapshot = err_buf.lock().unwrap().clone();
        eprintln!(
            "FAILED to extract session id. Raw STDOUT:\n{}",
            String::from_utf8_lossy(&out_snapshot)
        );
        eprintln!("Raw STDERR:\n{}", String::from_utf8_lossy(&err_snapshot));
        panic!("session id");
    });

    // Rapid appends
    let total = 50u32; // keep modest for CI speed
    for i in 0..total {
        let body = format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"method\":\"tools/call\",\"params\":{{\"name\":\"append_message\",\"arguments\":{{\"session_id\":\"{}\",\"role\":\"test\",\"content\":\"msg {}\"}}}}}}", 1000 + i, session_id, i);
        write_frame(stdin, &body);
    }
    let deadline2 = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline2 {
        if count_responses(&out_buf, 1000, 1000 + total as i64) >= total as usize {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(
        count_responses(&out_buf, 1000, 1000 + total as i64) >= total as usize,
        "did not receive all append responses"
    );

    // Fetch messages
    let list_body = format!("{{\"jsonrpc\":\"2.0\",\"id\":5000,\"method\":\"tools/call\",\"params\":{{\"name\":\"list_messages\",\"arguments\":{{\"session_id\":\"{}\",\"limit\":1000}}}}}}", session_id);
    write_frame(stdin, &list_body);
    let deadline3 = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline3 {
        if has_response(&out_buf, 5000) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let msgs = extract_messages(&out_buf).expect("messages");
    assert!(
        msgs.len() as u32 >= total,
        "expected at least {total} messages, got {}",
        msgs.len()
    );
    // Ascending by id
    let mut last_id = -1i64;
    for m in msgs.iter() {
        if let Some(id) = m.get("id").and_then(|v| v.as_i64()) {
            assert!(
                id > last_id,
                "message ids not strictly increasing: {} <= {}",
                id,
                last_id
            );
            last_id = id;
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    let _ = th_out.join();
    let _ = th_err.join();
}

fn has_response(buf: &Arc<Mutex<Vec<u8>>>, id: i64) -> bool {
    parse_messages(&buf.lock().unwrap())
        .into_iter()
        .any(|v| v.get("id").and_then(|i| i.as_i64()) == Some(id))
}
fn count_responses(buf: &Arc<Mutex<Vec<u8>>>, start: i64, end: i64) -> usize {
    parse_messages(&buf.lock().unwrap())
        .into_iter()
        .filter(|v| {
            if let Some(id) = v.get("id").and_then(|i| i.as_i64()) {
                id >= start && id < end
            } else {
                false
            }
        })
        .count()
}

fn extract_session_id(buf: &Arc<Mutex<Vec<u8>>>) -> Option<String> {
    for v in parse_messages(&buf.lock().unwrap()) {
        if v.get("id").and_then(|i| i.as_i64()) == Some(2) {
            if let Some(result) = v.get("result") {
                if let Some(export) = result.get("session") {
                    if let Some(id) = export.get("id").and_then(|s| s.as_str()) {
                        return Some(id.to_string());
                    }
                }
                if let Some(structured) = result.get("structuredContent") {
                    if let Some(sess) = structured.get("session") {
                        if let Some(id) = sess.get("id").and_then(|s| s.as_str()) {
                            return Some(id.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_messages(buf: &Arc<Mutex<Vec<u8>>>) -> Option<Vec<Value>> {
    for v in parse_messages(&buf.lock().unwrap()) {
        if v.get("id").and_then(|i| i.as_i64()) == Some(5000) {
            if let Some(result) = v.get("result") {
                if let Some(structured) = result.get("structuredContent") {
                    if let Some(msgs) = structured.get("messages").and_then(|m| m.as_array()) {
                        return Some(msgs.clone());
                    }
                }
            }
        }
    }
    None
}

fn parse_messages(raw: &[u8]) -> Vec<Value> {
    // Support BOTH protocols:
    // 1. Content-Length framed bodies (original expectation in earlier tests)
    // 2. Line-delimited JSON objects (current rust-mcp-transport stdio behavior)
    let mut msgs = Vec::new();
    // First, attempt framed extraction (can coexist with newline messages)
    let mut framed_cursor = 0usize;
    while framed_cursor < raw.len() {
        if let Some(pos) = memchr::memmem::find(&raw[framed_cursor..], b"Content-Length:") {
            let idx = framed_cursor + pos;
            if let Ok(s) = std::str::from_utf8(&raw[idx..]) {
                if let Some(term) = s.find("\r\n\r\n").or_else(|| s.find("\n\n")) {
                    let header_str = &s[..term];
                    let mut length: Option<usize> = None;
                    for line in header_str.lines() {
                        let parts: Vec<_> = line.splitn(2, ':').collect();
                        if parts.len() == 2 && parts[0].eq_ignore_ascii_case("Content-Length") {
                            length = parts[1].trim().parse().ok();
                        }
                    }
                    if let Some(len) = length {
                        let header_bytes = if s.as_bytes()[term..].starts_with(b"\r\n\r\n") {
                            term + 4
                        } else {
                            term + 2
                        };
                        let start = idx + header_bytes;
                        let end = start + len;
                        if end <= raw.len() {
                            if let Ok(val) = serde_json::from_slice(&raw[start..end]) {
                                msgs.push(val);
                            }
                            framed_cursor = end;
                            continue;
                        } else {
                            break;
                        }
                    }
                }
            }
            framed_cursor = idx + 1; // advance to avoid infinite loop on malformed data
        } else {
            break;
        }
    }
    // Next, parse newline-delimited JSON objects line by line
    if let Ok(all) = std::str::from_utf8(raw) {
        for line in all.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if t.starts_with('{') {
                if let Ok(val) = serde_json::from_str::<Value>(t) {
                    msgs.push(val);
                }
            }
        }
    }
    msgs
}
