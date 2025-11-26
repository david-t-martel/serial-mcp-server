Rust Serial Port MCP Server (v3.1 – Observability Edition)
=========================================================

This application provides a production-grade, highly reliable Machine Control Protocol (MCP) server for interacting with system serial ports. It is engineered for stability, explicit configurability, diagnosability, and deep observability—ideal for mission‑critical automation and integration with LLM / autonomous agents.

Core Capabilities (v3.1)
------------------------

 
* Panic-Free Runtime: All `unwrap()` calls removed in favor of structured, recoverable error flows.
* Structured MCP Tooling: Exposes a first‑class MCP tool set (`list_ports`, `open_port`, `write`, `read`, `close`, `status`, `metrics`).
* Rich Serial Configuration: Full control of baud rate, timeout, data bits, parity, stop bits, flow control, optional write terminator, and idle auto‑disconnect.
* Deterministic & Idempotent: Repeated `close` on an already closed port returns success; safe to retry after transient failures.
* Cross‑Platform: Works on Windows (`COM3` etc.) and Unix‑like systems (`/dev/ttyUSB0`, `/dev/ttyS0`, `/dev/ttyACM0`, etc.).
* Modular Architecture: Clear separation of concerns (`state`, `mcp`, `session`, `error`, optional legacy `stdio`). See `ARCHITECTURE.md` for details.
* Session Analytics: Persistent session logging with feature tagging, directional metadata, latency capture, filtering, and feature index aggregation.
* Metrics & Health: Real‑time cumulative counters (bytes read / written, open duration, idle auto‑close count) via the `metrics` tool.

Feature Flags & Interfaces
---------------------------

The project supports multiple interfaces for different use cases. All interfaces are equally valid and fully supported:

* **MCP** (default): LLM agent integration via Model Context Protocol
* **REST API** (opt-in): Web clients, HTTP-based automation, test frameworks  
* **stdio** (opt-in): Simple scripting, legacy integrations, command-line tools
* **WebSocket** (opt-in): Real-time streaming for monitoring applications

Build examples:
```bash
# MCP-only (default, minimal binary)
cargo build --release

# With REST API for web clients/testing
cargo build --release --features rest-api

# With stdio for scripting
cargo build --release --features legacy-stdio

# Full feature set
cargo build --release --all-features
```

Build & Workflow
----------------

Prerequisites: Latest stable Rust (install via <https://rustup.rs/>).

Use the provided Makefile for consistent developer flows (it encodes features & quality gates):

```bash
make help        # list targets
make build       # debug build
make release     # optimized build
make test        # run all tests (unit + integration)
make clippy      # lint (denies warnings)
make precommit   # fmt-check + clippy + tests + deny (if installed)
make db-init     # create / migrate session DB (optional, auto on first use)
```

You can still invoke cargo directly, but Make targets are the authoritative workflows.

Binary output: `target/release/serial_mcp_agent`.

Run (MCP / Stdio Transport)
---------------------------

 
When built with the `mcp` feature (default) the process speaks MCP over stdio:

```bash
./target/release/serial_mcp_agent
```

An MCP client (LLM host / orchestrator) should then perform a standard MCP initialize + `list_tools` flow.

Available MCP Tools
-------------------

Serial / Port Control:

1. `list_ports`      → Enumerate available system serial ports.
2. `open_port`       → Open a port with full configuration.
3. `write`           → Write UTF‑8 text to the open port (auto‑appends configured terminator if missing).
4. `read`            → Read up to 1024 bytes (non‑blocking beyond configured timeout; trims configured terminator if present).
5. `close`           → Close the port (idempotent).
6. `status`          → Return structured state, including current configuration if open.
7. `metrics`         → Return cumulative IO counters & timing.

Session Persistence & Analytics:

1. `create_session`      → Create a persistent session log (returns session id).
2. `append_message`      → Append a message with extended metadata.
3. `list_sessions`       → List all sessions with filtering (open/closed) and optional limit.
4. `close_session`       → Close a session by marking it as closed.
5. `list_messages`       → List messages (ascending; optional limit).
6. `list_messages_range` → List messages with cursor-based pagination (after_message_id).
7. `export_session`      → Export full session JSON (metadata + ordered messages).
8. `filter_messages`     → Filter messages by role / feature substring / direction.
9. `feature_index`       → Aggregate feature tag counts.
10. `session_stats`      → Session statistics (message count, timestamps).

Serial Configuration (open_port)
--------------------------------

 
| Field                | Type   | Default    | Allowed / Notes                                                                                           |
| -------------------- | ------ | ---------- | --------------------------------------------------------------------------------------------------------- |
| `port_name`          | string | (required) | System device identifier (e.g. `COM4`, `/dev/ttyUSB0`).                                                   |
| `baud_rate`          | u32    | (required) | Common values: 9600, 19200, 38400, 57600, 115200, etc.                                                    |
| `timeout_ms`         | u64    | 1000       | Read timeout in milliseconds.                                                                             |
| `data_bits`          | enum   | `eight`    | One of: `five`, `six`, `seven`, `eight` (numeric aliases `5..8`).                                         |
| `parity`             | enum   | `none`     | One of: `none`, `odd`, `even`.                                                                            |
| `stop_bits`          | enum   | `one`      | One of: `one`, `two`.                                                                                     |
| `flow_control`       | enum   | `none`     | One of: `none`, `hardware` (RTS/CTS), `software` (XON/XOFF).                                              |
| `terminator`         | string | (none)     | Optional line terminator appended on `write` (if absent) and trimmed on `read` (e.g. "\n", "\r", "\r\n"). |
| `idle_disconnect_ms` | u64    | (none)     | Milliseconds of inactivity (no successful read/write) after which the port is auto‑closed.                |

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
  "flow_control": "none",
  "terminator": "\n",
  "idle_disconnect_ms": 60000
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

`read` returns up to 1024 bytes. A timeout with no data yields 0 bytes and `"read 0 bytes"` (not an error). If a `terminator` is configured it is trimmed from the right edge of the returned text (single instance). Partial frames are expected—agents should internally buffer until a complete semantic unit (e.g. line) is assembled.

Idle Auto‑Disconnect
--------------------

If `idle_disconnect_ms` is set the port is automatically closed on a `read` call once the elapsed wall time since the last successful read or write exceeds that threshold. The `read` response will indicate closure (text content includes `closed (idle timeout)`). An agent may immediately reopen using the previous configuration if further communication is required.

Structured auto‑close event payload (in `read` structured_content):
```json
{
  "event": "auto_close",
  "reason": "idle_timeout",
  "idle_ms": 60000,
  "idle_close_count": 1
}
```

Metrics & Observability
-----------------------

The `metrics` tool returns cumulative counters & timing for the current port state.

Example structured_content fields:

* `state` – `Open` or `Closed`
* `bytes_read_total` – total bytes read since last open
* `bytes_written_total` – total bytes written since last open
* `idle_close_count` – number of idle auto‑closures in current open session
* `open_duration_ms` – milliseconds since port opened
* `last_activity_ms` – milliseconds since last successful read/write

Usage Tips:

* Poll before/after operation bursts to compute per‑burst deltas.
* Rising `last_activity_ms` combined with stable `bytes_read_total` indicates device silence.
* Increase polling cadence or raise `idle_disconnect_ms` if `idle_close_count` increments unexpectedly.

Session Persistence & Analytics Tools
-------------------------------------
Extended message schema (v3.1 additions):

* `direction` (optional) – e.g. `tx`, `rx`, `agent`.
* `features` (optional) – space or comma separated feature tokens.
* `latency_ms` (optional) – associated latency measurement.

Common Pattern:

1. `create_session` at start of workflow.
2. After each read → `append_message(role=device, direction=rx, content=...)`.
3. After each write → optionally `append_message(role=tool, direction=tx, content=command)`.
4. Use `features` to tag semantic meaning (e.g. `temp voltage ack`).
5. Query `feature_index` + `filter_messages` for targeted analysis.

Closing & Reconfiguration
-------------------------

Reconfiguring currently requires `close` then a new `open_port` with updated parameters (a dedicated reconfigure tool may be added later).

Legacy Interfaces
-----------------

The legacy (non-MCP) stdio command surface is deprecated and only built when the `mcp` feature is disabled. It intentionally returns deprecation errors for prior commands. Prefer MCP for all integrations.

Database Initialization
-----------------------

The session database schema is applied automatically on first use. To pre-create (e.g., packaging/CI):

```bash
make db-init
SESSION_DB_URL=sqlite://data/sessions.db make db-init
```

If the configured on-disk database cannot be opened (e.g. read-only filesystem), the server logs a warning and falls back to an in-memory SQLite instance so functionality remains available (persistence disabled for that run).

Reliability & Error Semantics
-----------------------------

Production code avoids `unwrap()` / `expect()` so recoverable failures never abort the process. Error handling strategy:

* MCP tool failures return structured `CallToolError` variants (invalid arguments / unknown tool / message).
* Serial port conflicts (already open / not open) are surfaced as user-correctable tool errors.
* Session DB initialization failure triggers a logged warning and in-memory fallback.
* Idle timeouts emit a structured auto-close event instead of a bare error.
* Integration tests may still use `expect()` for clarity; runtime paths do not.

Development & Contribution
--------------------------

* Run debug build: `cargo build`
* Format / lint (optional): `cargo fmt` / `cargo clippy`
* Bench scaffolding: `cargo bench` (basic Criterion harness included)
* Custom session DB path: set `SESSION_DB_URL=sqlite://sessions.db` (defaults to in-project `sqlite://sessions.db` if unspecified)
* Generate docs: `cargo doc --no-deps --open`

Changelog (v3.1)
----------------

* Added tools: `metrics`, `filter_messages`, `feature_index`.
* Extended `append_message` with `direction`, `features`, `latency_ms`.
* Added cumulative IO counters and timing to `PortState`.
* Structured idle auto‑close event output in `read`.
* Documentation updated for observability & analytics features.

See `ARCHITECTURE.md` for an internal systems overview and `llms.txt` for succinct agent guidance.

License
-------

Dual-licensed under MIT or Apache-2.0.
