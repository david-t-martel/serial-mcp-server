# MCP Serial Server Protocol & Persistence Schema

> Version: 0.1 (2025-10-03) – Draft documenting current implementation state. Will evolve as framing strictness and multi‑port support are added.

## 1. Transport Layer

The server uses **MCP (Machine Control Protocol)** over STDIO via `rust-mcp-sdk`.
All outbound messages MUST be framed with HTTP‑style headers:

```text
Content-Length: <bytes>\r\n
Content-Type: application/json\r\n
\r\n
<JSON payload>
```

Current State: Most responses are framed by the SDK. An optional early debug frame can be emitted when the environment variable `MCP_DEBUG_BOOT` is set. A known gap exists around reliably observing the *initialize* response under stress; strict framing tests are planned.

Planned Tightening:

- Eliminate any reliance on raw (unframed) fallback parsing in tests.
- Add a startup heartbeat frame (minimal `{}` JSON) if initialize is delayed beyond a short threshold.

## 2. Initialization Flow

Client sequence:

1. Launch process (captures stdin/stdout pipes).
2. Send MCP `initialize` request per spec.
3. Read framed `initialize` result (server details + capabilities).
4. Call `tools/list` to enumerate supported tools.

Server responds with capabilities advertising the tool surface. No authentication is currently implemented.

## 3. Tool Invocation Contract

Each tool invocation is a JSON-RPC request via MCP method namespace (e.g. `tools/call`). Parameters are passed via `arguments` map (string → JSON value). The server returns:

```json
{
  "content": [{ "type": "text", "text": "<human summary>" }],
  "structured_content": { /* machine-friendly fields */ }
}
```

Errors use `CallToolError` (`invalid_arguments`, `unknown_tool`, or message-bearing error) surfaced in the MCP error channel.

## 4. Serial Port State Machine

State Enum (single-port current design):

- `Closed`
- `Open { port, config, last_activity, timeout_streak, bytes_read_total, bytes_written_total, idle_close_count, open_started }`

Metrics reset whenever a port is (re)opened.

### Auto-Close Behavior

If `idle_disconnect_ms` is configured and no successful read/write occurs within that interval, a `read` invocation triggers an auto-close:

```json
{
  "event": "auto_close",
  "reason": "idle_timeout",
  "idle_ms": <threshold>,
  "idle_close_count": <n>
}
```

### Timeout Streak

A `read` that returns 0 bytes due to timeout increments `timeout_streak`. Any successful read resets it to 0. Exposed via `metrics`.

## 5. Serial Configuration Model

```rust
struct PortConfig {
  port_name: String,
  baud_rate: u32,
  timeout_ms: u64,
  data_bits: String,   // five|six|seven|eight
  parity: String,      // none|odd|even
  stop_bits: String,   // one|two
  flow_control: String,// none|hardware|software
  terminator: Option<String>,
  idle_disconnect_ms: Option<u64>,
}
```

### Terminator Semantics

- On `write`: appended if not already present.
- On `read`: exactly one trailing instance trimmed (if present) prior to returning `data`.

## 6. Tools (Current Set)

| Tool | Purpose | Notes |
|------|---------|-------|
| `list_ports` | Enumerate system serial ports | Port objects may include additional OS-specific metadata (future). |
| `open_port` | Open port with provided settings | Fails if already open. |
| `write` | Write UTF-8 data | Adds terminator if configured. |
| `read` | Read up to 1024 bytes | Non-blocking beyond timeout; trims terminator. |
| `close` | Idempotent close | Succeeds if already closed. |
| `status` | Return current port state | Embeds serialized `PortState`. |
| `metrics` | Counters & timing | Now includes `timeout_streak`. |
| `reconfigure_port` | Open or reopen with new config | Resets metrics; port name optional if already open. |
| `create_session` | Start persistent session | Returns session id (UUID). |
| `append_message` | Append timeline entry | Extended metadata supported. |
| `list_messages` | Sequential listing | Ascending by autoincrement id. |
| `export_session` | Full session dump | Includes messages array. |
| `filter_messages` | Filtered subset | Role / feature substring / direction. |
| `feature_index` | Feature tag aggregation | Map of token → count. |
| `session_stats` | Lightweight stats | count, last id, rate (messages/min). |

## 7. Persistence Schema (SQLite)

Autocreated via `SessionStore::new` migrations.

### Tables

```sql
CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL,
  port_name TEXT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  closed_at TEXT NULL
);

CREATE TABLE messages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  role TEXT NOT NULL,
  direction TEXT NULL,
  content TEXT NOT NULL,
  features TEXT NULL,
  latency_ms INTEGER NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### Ordering & Determinism

Message ordering uses the monotonically increasing `id`. The `append_message` tool returns `(message_id, created_at)` allowing exact reconstruction and verification (tests assert ordering under rapid insertion).

## 8. Device Discovery (Forward-Looking)

Current `list_ports` exposes minimal port_name. Planned enrichment:

- Vendor / Product IDs (VID/PID) where available.
- Serial number, manufacturer, description.
- Transport type inference (USB-UART, PCI, Bluetooth SPP, etc.).
- Capability flags (e.g., supports high baud > 1M, RS485 direction control, etc.).

Non‑FTDI Support Strategy:

- Abstract detection logic: gather all ports; attempt OS-specific metadata extraction (Windows via `SetupAPI`, Linux via `/sys/class/tty/*/device/..` udev properties, macOS via IOKit). Populate a unified `DiscoveredPort` struct.
- Provide future `describe_port` tool returning extended metadata.
- Heuristic classification (match common driver names: `FTDI`, `Silicon Labs`, `Prolific`, `CH34x`, `CP21xx`).

## 9. Reconfiguration Semantics

`reconfigure_port` will:

- Determine target port name (argument or currently open one).
- Open a new port handle with provided settings (dropping previous handle).
- Reset all metrics counters and `timeout_streak`.
- Return structured reflection of new config.

## 10. Session Analytics & Feature Index

`feature_index` tokenizes the raw `features` string field by splitting on whitespace and commas. Future iteration will introduce a `message_features` linking table for normalized many‑to‑many relations enabling precise counts and advanced queries (e.g., co‑occurrence).

## 11. Hardware Loopback Validation Procedure

A dedicated self-test will:

1. Discover candidate port pair (auto or via `LOOPBACK_PORT_A` / `LOOPBACK_PORT_B`).
2. For a suite of baud rates `[9600, 19200, 38400, 57600, 115200, 230400]`:
   - Reconfigure port.
   - Send framed test lines with sequence, timestamp, CRC32.
   - Measure round-trip or one-way latency (depending on cross-loop wiring).
3. Track success ratio, latency distribution, throughput.
4. Emit JSON summary for machine ingestion + human table.
5. Optionally reverse direction.

## 12. Planned Extensions

- Multi-port: Map of `port_name -> PortState` with handles; tools gain `port_name` parameter.
- Binary Mode: `write_binary` / `read_binary` with base64 or raw framing + length prefix.
- Streaming Subscriptions: Server → client push when new data arrives without polling.
- Structured Error Codes: Replace generic messages with enumerated codes (`ERR_PORT_BUSY`, `ERR_DB_WRITE`, etc.).

## 13. Compatibility & Stability

Stability tiers:

- Stable: Basic serial tools (`list_ports`, `open_port`, `write`, `read`, `close`, `status`, `metrics`).
- Beta: Session analytics tools.
- Experimental: `reconfigure_port` (new), planned discovery enrichment.

Breaking changes will be noted in `CHANGELOG.md` and versioned semantically.

## 14. Security Considerations

Current implementation has no auth. Planned:

- Capability allowlist configuration file.
- Optional signing of tool requests.
- Rate limiting per tool (particularly `read`).

## 15. Reference Client Notes

Clients should:

- Buffer partial reads to reconstruct semantic messages.
- Backoff on consecutive timeouts (exponential strategy) or adjust timeout_ms.
- Use `metrics` deltas for health dashboards.
- Persist session ids externally for later export / analytics.

---
Generated: 2025-10-03
