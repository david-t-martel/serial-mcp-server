Rust Serial Port MCP Server (v3.0 - Robust Edition)
This application provides a production-grade, highly reliable Machine Control Protocol (MCP) server for interacting with system serial ports. It is engineered for stability and diagnosability, making it an ideal tool for mission-critical automation and integration with LLM agents.

Key Features of v3.0
Panic-Free Runtime: All unwrap() calls have been eliminated in favor of a comprehensive error handling system. The server is designed to run indefinitely without crashing.

Structured, Diagnosable Errors: Failures no longer produce generic messages. Instead, they return a structured JSON object ({"status": "error", "error": {"type": "...", "message": "..."}}) that allows an automated agent to understand the specific type of error and react accordingly.

Enhanced Code Modularity: The codebase is now organized into logical modules (error, state, mcp, stdio), making it significantly easier to maintain, test, and extend.

Cross-Platform Guidance: Documentation and examples now explicitly guide the user on platform-specific serial port naming conventions (e.g., COMx for Windows, /dev/tty... for Unix-like systems).

Idempotent Operations: Certain operations, like closing an already-closed port, are now treated as successful no-ops rather than errors, leading to more predictable state management for agents.

Building the Application
Ensure you have the Rust toolchain installed. You can get it from rustup.rs.

Build the project in release mode for optimal performance:

cargo build --release

The executable will be located at ./target/release/serial_mcp_agent.

Running the Application
HTTP Server Mode
For network-based agents, this mode provides a RESTful API.

# Start the server on the default port 3000
./target/release/serial_mcp_agent --server

Stdio Mode
For local agents, this mode provides a JSON-RPC style interface over stdin and stdout.

./target/release/serial_mcp_agent

Example Error Response
If you attempt to read from a port that is not open, you will receive a clear, structured error instead of a crash:

HTTP Response (409 Conflict):

{
  "status": "error",
  "error": {
    "type": "PortNotOpen",
    "message": "Operation requires an open serial port, but the port is closed."
  }
}

Stdio Response:

{
  "status": "error",
  "error": {
    "code": 409,
    "message": "Operation requires an open serial port, but the port is closed.",
    "type": "PortNotOpen"
  }
}

This level of detail is crucial for an agent to self-correct. For example, upon receiving a PortNotOpen error, it can automatically try listing available ports and then opening one before retrying the original command.