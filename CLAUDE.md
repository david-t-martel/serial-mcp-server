# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Serial MCP Server (v3.1) - A production-grade Machine Control Protocol (MCP) server for serial port communication. Designed for LLM/autonomous agent integration with panic-free runtime, structured error handling, and session analytics.

## Build & Development Commands

Use the Makefile for all workflows:

```bash
make build       # Debug build with all features
make release     # Optimized release build
make test        # Run all tests (unit + integration)
make clippy      # Lint with -D warnings
make precommit   # Full pre-commit: fmt-check + clippy + test + deny
make run         # Run in stdio MCP mode
make db-init     # Initialize/migrate session database
```

Run a specific test:
```bash
cargo test terminator_append -- --nocapture
cargo test --lib  # Unit tests only (skip benches)
```

Coverage (optional):
```bash
cargo llvm-cov --all-features --html
```

Binary output: `target/release/serial_mcp_agent`

## Architecture

### Module Structure

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point; CLI parsing, feature gates, launches MCP or HTTP server |
| `mcp.rs` | MCP server handler implementing `rust-mcp-sdk`; all tool definitions and dispatch |
| `state.rs` | Shared `PortState` enum (Closed/Open) + `PortConfig` with serial settings |
| `session.rs` | SQLite-backed session persistence using `sqlx`; messages, filtering, analytics |
| `error.rs` | Unified `AppError` enum for REST/legacy surfaces |
| `stdio.rs` | Deprecated legacy stdio interface (only built without `mcp` feature) |
| `rest_api.rs` | REST API placeholder (feature-gated, minimal) |

### Data Flow

1. MCP client connects via stdio transport (newline-delimited JSON-RPC 2.0)
2. Client calls `list_tools` then invokes tools via `tools/call` method
3. `ServerHandler::handle_call_tool_request` dispatches to `*_impl` methods
4. Tool methods mutate/read `PortState` inside `Arc<Mutex<...>>`
5. Responses return `TextContent` + optional `structured_content` map

### Port State Model

Single port managed at a time. `PortState::Open` contains:
- Serial port handle
- Immutable `PortConfig` snapshot
- Activity tracking: `last_activity`, `timeout_streak`
- Metrics: `bytes_read_total`, `bytes_written_total`, `idle_close_count`, `open_started`

### Session Persistence

Two SQLite tables (`sessions`, `messages`) with:
- UUID-based session IDs
- Extended message metadata: `direction`, `features`, `latency_ms`
- Indexes on `session_id`, `role`, `features` for filtering
- Falls back to in-memory SQLite if on-disk DB unavailable

## Feature Flags

```toml
[features]
default = ["mcp", "rest-api"]
mcp = ["rust-mcp-sdk"]      # Official MCP SDK integration (recommended)
rest-api = ["axum"]          # HTTP surface placeholder
```

Build without MCP for deprecated legacy stdio: `cargo build --no-default-features`

## MCP Tools

**Serial Control:** `list_ports`, `list_ports_extended`, `open_port`, `write`, `read`, `close`, `status`, `metrics`, `reconfigure_port`

**Session Analytics:** `create_session`, `append_message`, `list_messages`, `export_session`, `filter_messages`, `feature_index`, `session_stats`

## Protocol Notes

- Transport: Newline-delimited JSON-RPC 2.0 over stdio (no `Content-Length` framing)
- Tool invocation method: `tools/call` (not legacy `callTool`)
- Heartbeat: Server emits `{"jsonrpc":"2.0","method":"_heartbeat","params":{}}` on startup (disable with `MCP_DISABLE_HEARTBEAT=1`)
- Idle auto-close: Configured via `idle_disconnect_ms`; triggers on `read` when threshold exceeded

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `SESSION_DB_URL` | SQLite database URL (default: `sqlite://sessions.db`) |
| `MCP_DISABLE_HEARTBEAT` | Suppress startup heartbeat notification |
| `MCP_DEBUG_BOOT` | Emit debug boot marker on startup |
| `RUST_LOG` | Tracing filter (default: `info`) |
| `SERIAL_TEST_PORT` | Specify real serial port for integration tests |

## Testing Strategy

- **Unit tests:** Terminator logic, idle timeout decisions, feature index tokenization
- **Session tests:** CRUD flows against in-memory SQLite (`sqlite::memory:?cache=shared`)
- **Integration tests:** Spawn binary, MCP initialize + tool calls
- **Smoke tests:** `tests/smoke_stdio.rs` - basic process start/stop

Real serial hardware not guaranteed in CI. Tests skip device-dependent operations if `SERIAL_TEST_PORT` unset.

## Error Handling

- MCP tools return `CallToolError` variants (`from_message`, `invalid_arguments`, `unknown_tool`)
- No `unwrap()`/`expect()` in runtime paths (integration tests may use for clarity)
- Serial conflicts surfaced as user-correctable tool errors
- Session DB failures trigger logged warning + in-memory fallback

## Concurrency

Single `Mutex<PortState>` guards all port operations. Serial I/O is short and blocking. For high-latency streaming, future work may spawn dedicated IO tasks with mpsc channels.
