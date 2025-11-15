# Architecture Overview

This document explains the internal structure of the Serial MCP Agent.

## Modules

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point; parses CLI / feature gates and launches MCP runtime. |
| `mcp.rs` | Implements MCP server handler and tool definitions using `rust-mcp-sdk` (serial + session tools). |
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
* `Open { port, config, last_activity, timeout_streak }` – active serialport handle plus immutable configuration snapshot, plus activity metadata for idle disconnect.

Only one port is managed at a time (intentionally simple). A future multi‑port mode could promote state to a map keyed by `port_name` with reference counts or session tokens.

## Configuration

`PortConfig` + enums provide explicit, serde + JSON schema derivable representation:

* `baud_rate`, `timeout_ms`, `data_bits`, `parity`, `stop_bits`, `flow_control`.
* `terminator` (optional string) appended on write if absent; trimmed from end of read.
* `Open { port, config, last_activity, timeout_streak, bytes_read_total, bytes_written_total, idle_close_count, open_started }` – active serialport handle plus immutable configuration snapshot, activity metadata, and cumulative IO metrics.
* `idle_disconnect_ms` (optional u64) triggers auto close when exceeded without successful IO.
Defaults are applied via `serde(default=...)` helpers.

## Error Handling

* `idle_disconnect_ms` (optional u64) triggers auto close when exceeded without successful IO (enforced during a `read` call).
MCP surface: uses `CallToolError` constructors (`from_message`, `invalid_arguments`, `unknown_tool`).
Legacy surfaces (REST / stdio) rely on `AppError` for typed classification.

## Concurrency Considerations
`last_activity` updated on successful read (bytes > 0) and on successful write. Idle determination ignores empty timeouts. `timeout_streak` reserved for future heuristics. Metrics fields accumulate per open lifecycle and reset when a new `open_port` occurs.

Structured idle auto‑close event (emitted via `read` structured_content):
```json
{
	"event": "auto_close",
	"reason": "idle_timeout",
	"idle_ms": <configured>,
	"idle_close_count": <count>
}
```

Metrics Tool (`metrics`): returns `state`, `bytes_read_total`, `bytes_written_total`, `idle_close_count`, `open_duration_ms`, `last_activity_ms` for observability & health tracking.
A single mutex guards `PortState`. Serial operations are short and blocking (write/read). For high‑latency or streaming use cases, a future refactor could:

* Spawn a dedicated serial IO task with an mpsc channel.
* Provide streaming notifications via MCP if protocol evolves.

## Validation & Idempotency

* `messages` (`id` UUID, `session_id` FK, `role`, `direction` nullable, `features` nullable, `latency_ms` nullable, `content`, `created_at`)
* `open_port` rejects if already open.
* `close` always succeeds (idempotent) returning either `closed` or `already closed` message.
* `read` timeout returns 0 bytes instead of failing and updates inactivity tracking.
* Auto‑close: On a `read` invocation, if `idle_disconnect_ms` is configured and `now - last_activity` exceeds it, the port transitions to `Closed` and a closure message is returned.

## Session Persistence Layer

* `filter_messages` – server-side filter by role, direction, feature substring.
* `feature_index` – aggregate token counts (splitting `features` on whitespace / commas) for topical summary.
Implemented in `session.rs` using SQLite (via `sqlx`). Two tables:

* `sessions` (`id` UUID primary key, `device_id`, `port_name`, `created_at`, `closed_at` nullable)
* `messages` (`id` UUID, `session_id` FK, `role`, `content`, `created_at`)

MCP Tools:

* `create_session` – inserts a row and returns metadata.
* `append_message` – adds a message (role taxonomy unopinionated: `user`, `device`, `agent`, etc.).
* `list_messages` – ordered ascending; optional limit.
* Advanced metrics (EWMA throughput, timeout streak exposure, error counters).
* `export_session` – joins + returns full payload for archival.

The handler holds a single `SessionStore` instance injected from `main.rs` to avoid multiple SQLite connections & duplicated schema initialization.

## Activity Tracking & Idle Disconnect

`last_activity` updated on successful read with bytes > 0 and on successful write. Idle determination intentionally ignores empty timeouts to prevent spurious extension unless real data flows. `timeout_streak` can be used for future heuristics (e.g., escalating polling backoff or health metrics).

## Extensibility Hooks

Potential next steps:

* `reconfigure_port` tool: apply changes without full close/open cycle (requires serialport crate support or custom logic).
* Binary read/write (base64 payload).
* Line oriented reading with delimiter detection.
* Multi‑port support with explicit port handles.
* Metrics / health tool returning counters (bytes read/written, open duration, idle closures).

## Feature Flags

* `mcp` – official MCP integration (default).
* `rest-api` – placeholder for HTTP surface (currently minimal; may be removed or redesigned).
* Without `mcp` – legacy stdio stub signals deprecation.

## Testing Strategy (Planned)

* Unit tests: configuration parsing, terminator append/trim logic, idle disconnect threshold evaluation.
* Integration tests: open → write (with terminator) → read (verify trimmed) → idle timeout auto close (simulate time advance / configurable clock abstraction if added) → reopen.
* Session tests: create_session → append_message → list_messages (ordering) → export_session (structure integrity).
* Property tests (optional): invalid argument rejection for each enum, fuzz random terminator strings.

## Benchmarks

Criterion harness (`benches/basic.rs`) reserved for future throughput measurements (e.g., write loop latency, read polling cost).


## Security / Safety Notes

* No arbitrary file or process access – limited to serial ports recognized by `serialport` crate.
* Caller must ensure appropriate OS permissions for device access.
* No background threads beyond those used by Tokio + MCP runtime.

## Rationale Summary

The design favors clarity and deterministic behavior over maximal throughput. Extensibility surfaces (tool definitions, config, state) are intentionally explicit to optimize for autonomous agent reliability.
