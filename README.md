Rust Serial Port MCP Server (v3.0 – Robust Edition)
=================================================

This application provides a production-grade, highly reliable Machine Control Protocol (MCP) server for interacting with system serial ports. It is engineered for stability, explicit configurability, and diagnosability—ideal for mission‑critical automation and integration with LLM / autonomous agents.

Core Capabilities (v3.0)
------------------------

 
* Panic-Free Runtime: All `unwrap()` calls removed in favor of structured, recoverable error flows.
* Structured MCP Tooling: Exposes a first‑class MCP tool set (`list_ports`, `open_port`, `write`, `read`, `close`, `status`).
* Rich Serial Configuration: Full control of baud rate, timeout, data bits, parity, stop bits, and flow control.
* Deterministic & Idempotent: Repeated `close` on an already closed port returns success; safe to retry after transient failures.
* Cross‑Platform: Works on Windows (`COM3` etc.) and Unix‑like systems (`/dev/ttyUSB0`, `/dev/ttyS0`, `/dev/ttyACM0`, etc.).
* Modular Architecture: Clear separation of concerns (`state`, `mcp`, `error`, optional legacy `stdio`). See `ARCHITECTURE.md` for details.

Feature Flags
-------------

 
* `mcp` (default): Enables the official `rust-mcp-sdk` server (recommended).
* `rest-api` (default): Placeholder for future HTTP surface (currently minimal / deprecated path).
* Build without MCP (not recommended) to expose a deprecated legacy stdio command interface.

Build
-----

 
Prerequisites: Latest stable Rust (install via <https://rustup.rs/>).

Optimized release build:

    cargo build --release

Binary output: `target/release/serial_mcp_agent`.

Run (MCP / Stdio Transport)
---------------------------

 
When built with the `mcp` feature (default) the process speaks MCP over stdio:

    ./target/release/serial_mcp_agent

An MCP client (LLM host / orchestrator) should then perform a standard MCP initialize + `list_tools` flow.

Available MCP Tools
-------------------

 
1. `list_ports`  → Enumerate available system serial ports.
2. `open_port`   → Open a port with full configuration.
3. `write`       → Write UTF‑8 text data (raw bytes of UTF‑8 string) to the open port.
4. `read`        → Read up to 1024 bytes (non‑blocking beyond configured timeout).
5. `close`       → Close the port (idempotent).
6. `status`      → Return structured state, including current configuration if open.

Serial Configuration (open_port)
--------------------------------

 
| Field          | Type   | Default    | Allowed / Notes                                                            |
| -------------- | ------ | ---------- | -------------------------------------------------------------------------- |
| `port_name`    | string | (required) | System device identifier (e.g. `COM4`, `/dev/ttyUSB0`).                    |
| `baud_rate`    | u32    | (required) | Common values: 9600, 19200, 38400, 57600, 115200, etc.                     |
| `timeout_ms`   | u64    | 1000       | Read timeout in milliseconds.                                              |
| `data_bits`    | enum   | `eight`    | One of: `five`, `six`, `seven`, `eight` (numeric aliases `5..8` accepted). |
| `parity`       | enum   | `none`     | One of: `none`, `odd`, `even`.                                             |
| `stop_bits`    | enum   | `one`      | One of: `one`, `two`.                                                      |
| `flow_control` | enum   | `none`     | One of: `none`, `hardware` (RTS/CTS), `software` (XON/XOFF).               |

Example MCP Call (open_port)
----------------------------

 
Pseudo JSON-RPC payload (client perspective):

```json
{
  "method": "tools/call",
  "params": {
    "name": "open_port",
    "arguments": {
      "port_name": "COM3",
      "baud_rate": 115200,
      "timeout_ms": 500,
      "data_bits": "eight",
      "parity": "none",
      "stop_bits": "one",
      "flow_control": "none"
    }
  }
}
```

Successful Response (abridged):
 
```json
{
  "content": [{"type": "text", "text": "opened"}],
  "structured_content": {}
}
```

Status Example
--------------
```json
{
  "method": "tools/call",
  "params": { "name": "status" }
}
```
Response (if open):
```json
{
  "content": [{"type": "text", "text": "status"}],
  "structured_content": {
    Status Example
    --------------
      "details": {
 
        "config": {
          "port_name": "COM3",
          "baud_rate": 115200,
          "timeout_ms": 500,
          "data_bits": "eight",
          "parity": "none",
          "stop_bits": "one",
          "flow_control": "none"
        }
      }
    }
  }
}
```

Error Semantics
---------------
Representative MCP tool errors use `CallToolError` forms (`invalid_arguments`, `unknown_tool`, or message). Agents should:
* Retry after transient I/O errors (e.g., permission denied due to another process locking the port—wait then retry `open_port`).
* On `invalid_arguments`, correct the offending field(s) before retrying.
* If `Port already open` appears when calling `open_port`, either `close` first or proceed with operations.

Reading Behavior
----------------
`read` returns up to 1024 bytes. A timeout with no data yields 0 bytes and `"read 0 bytes"` (not an error). Partial frames are possible—agents should buffer if they require line or packet boundaries.

Closing & Reconfiguration
-------------------------
Reconfiguring currently requires `close` then a new `open_port` with updated parameters (a dedicated reconfigure tool may be added later).

Legacy Interfaces
-----------------
The legacy (non-MCP) stdio command surface is deprecated and only built when the `mcp` feature is disabled. It intentionally returns deprecation errors for prior commands. Prefer MCP for all integrations.

Development & Contribution
--------------------------
* Run debug build: `cargo build`
* Format / lint (optional): `cargo fmt` / `cargo clippy`
* Bench scaffolding: `cargo bench` (basic Criterion harness included)

See `ARCHITECTURE.md` for an internal systems overview and `llms.txt` for succinct agent guidance.

License
-------
Dual-licensed under MIT or Apache-2.0.
