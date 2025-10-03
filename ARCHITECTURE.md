# Architecture Overview

This document explains the internal structure of the Serial MCP Agent.

## Modules

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point; parses CLI / feature gates and launches MCP runtime. |
| `mcp.rs` | Implements MCP server handler and tool definitions using `rust-mcp-sdk`. |
| `state.rs` | Shared application state & serial configuration enums. |
| `error.rs` | Unified error enumeration (legacy / REST oriented still present). |
| `stdio.rs` | Deprecated legacy stdio command interface (compiled only without `mcp`). |

## Data Flow (MCP)

1. MCP client connects over stdio transport.
2. Client calls `list_tools` then invokes tool methods via `tools/call` RPC.
3. `ServerHandler::handle_call_tool_request` dispatches to internal `*_impl` methods.
4. Methods mutate / read `PortState` inside an `Arc<Mutex<...>>`.
5. Responses return textual `TextContent` plus optional `structured_content` map.

## Port State Model

`PortState` enum:

* `Closed` – no active port.
* `Open { port, config }` – active serialport handle plus immutable configuration snapshot.

Only one port is managed at a time (intentionally simple). A future multi‑port mode could promote state to a map keyed by `port_name` with reference counts or session tokens.

## Configuration

`PortConfig` + enums provide explicit, serde + JSON schema derivable representation:

* `baud_rate`, `timeout_ms`, `data_bits`, `parity`, `stop_bits`, `flow_control`.
Defaults are applied via `serde(default=...)` helpers.

## Error Handling

MCP surface: uses `CallToolError` constructors (`from_message`, `invalid_arguments`, `unknown_tool`).
Legacy surfaces (REST / stdio) rely on `AppError` for typed classification.

## Concurrency Considerations

A single mutex guards `PortState`. Serial operations are short and blocking (write/read). For high‑latency or streaming use cases, a future refactor could:

* Spawn a dedicated serial IO task with an mpsc channel.
* Provide streaming notifications via MCP if protocol evolves.

## Validation & Idempotency

* `open_port` rejects if already open.
* `close` always succeeds (idempotent) returning either `closed` or `already closed` message.
* `read` timeout returns 0 bytes instead of failing.

## Extensibility Hooks

Potential next steps:

* `reconfigure_port` tool: apply changes without full close/open cycle (requires serialport crate support or custom logic).
* Binary read/write (base64 payload).
* Line oriented reading with delimiter detection.
* Multi‑port support with explicit port handles.
* Metrics / health tool returning counters (bytes read/written, open duration).

## Feature Flags

* `mcp` – official MCP integration (default).
* `rest-api` – placeholder for HTTP surface (currently minimal; may be removed or redesigned).
* Without `mcp` – legacy stdio stub signals deprecation.

## Testing Strategy (Planned)

* Unit tests: configuration parsing and enum defaults.
* Integration tests: open → write → read (loopback / virtual serial) → status → close.
* Property tests (optional): invalid argument rejection for each enum.

## Benchmarks

Criterion harness (`benches/basic.rs`) reserved for future throughput measurements (e.g., write loop latency, read polling cost).


## Security / Safety Notes

* No arbitrary file or process access – limited to serial ports recognized by `serialport` crate.
* Caller must ensure appropriate OS permissions for device access.
* No background threads beyond those used by Tokio + MCP runtime.

## Rationale Summary

The design favors clarity and deterministic behavior over maximal throughput. Extensibility surfaces (tool definitions, config, state) are intentionally explicit to optimize for autonomous agent reliability.
