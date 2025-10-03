//! Basic smoke tests for stdio command processing logic.
use std::process::{Command, Stdio};
use std::io::{Write, Read};

fn spawn_stdio() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_serial_mcp_agent"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start binary")
}

#[test]
fn help_command_returns_description() {
    let mut child = spawn_stdio();
    let stdin = child.stdin.as_mut().expect("stdin available");
    stdin.write_all(b"{\"command\": \"help\"}\n").unwrap();

    let mut out = String::new();
    child.stdout.as_mut().unwrap().read_to_string(&mut out).unwrap();

    assert!(out.contains("MCP Interface for Rust Serial Port Server"));
}
