//! Basic smoke tests for stdio command processing logic.
use std::process::{Command, Stdio};
use std::io::{Write, Read};
use std::time::{Duration, Instant};
use std::thread;

fn spawn_stdio() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_serial_mcp_agent"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start binary")
}

#[test]
fn help_command_returns_description() {
    let mut child = spawn_stdio();
    let stdin = child.stdin.as_mut().expect("stdin available");
    stdin.write_all(b"{\"command\": \"help\"}\n").unwrap();
    stdin.flush().ok();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut collected = String::new();
    let mut buf = [0u8; 512];
    loop {
        if Instant::now() > deadline { break; }
        match child.stdout.as_mut().unwrap().read(&mut buf) {
            Ok(0) => { thread::sleep(Duration::from_millis(50)); },
            Ok(n) => {
                collected.push_str(&String::from_utf8_lossy(&buf[..n]));
                if collected.contains("MCP Interface for Rust Serial Port Server") { break; }
            }
            Err(_) => { thread::sleep(Duration::from_millis(25)); }
        }
    }

    // Clean up child process so test exits promptly.
    let _ = child.kill();
    let _ = child.wait();

    assert!(collected.contains("MCP Interface for Rust Serial Port Server"), "help output missing. Got: {}", collected);
}
