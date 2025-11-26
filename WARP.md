# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Serial MCP Server (v3.2) - A production-grade Machine Control Protocol (MCP) server for serial port communication, designed for LLM/autonomous agent integration with panic-free runtime, structured error handling, and session analytics.

**Key Features:**
- MCP tool interface for serial port control (list, open, read, write, close, status, metrics)
- Session persistence and analytics with SQLite
- Cross-platform support (Windows, Linux, macOS)
- Optional TUI for interactive serial console management
- Async serial support with auto-negotiation
- WebSocket streaming and OpenAPI documentation

## Common Development Commands

### Building

Use `just` (recommended) or `make` for consistent workflows:

```bash
# Build release binary with MCP features
just build

# Build debug binary
just build-debug

# Build TUI with scripting support
just build-tui-full

# Build optimized for size
just build-small
```

Or with make:
```bash
make build       # Debug build
make release     # Optimized release build
```

### Testing

```bash
# Run all tests
just test
# or
make test

# Run unit tests only
just test --lib
# or
make test-unit

# Run with verbose output
just test-verbose

# Run tests with coverage (requires cargo-llvm-cov)
just coverage
# or
make coverage

# Run hardware tests (requires actual serial devices)
# Windows:
just test-hardware-win config="./config.toml"
# Linux/macOS:
just test-hardware config="./config.toml"

# Run hardware tests with specific port
just test-hardware-port COM3 115200
```

### Code Quality

```bash
# Format code
just fmt
# or
make fmt

# Check formatting without modifying
just fmt-check

# Run clippy lints (denies warnings)
just clippy
# or
make clippy

# Run all lints (format + clippy)
just lint

# Auto-fix lint issues
just fix

# Run pre-commit checks (fmt-check + clippy + test + deny)
just release-check
# or
make precommit
```

### Running

```bash
# Run MCP server (stdio mode)
just run
# or
make run

# Run HTTP server on port 3000
just run-server 3000

# Run TUI application
just tui

# Run TUI with scripting
just tui-full

# Discover available serial ports
just discover
```

### Git Hooks

```bash
# Install git hooks (recommended for contributors)
make hooks-install

# Uninstall git hooks
make hooks-uninstall

# Test pre-commit hook
make hooks-test
```

### Development Setup

```bash
# Full development environment setup
make setup-dev

# Initialize session database
make db-init

# Create config from example
just config-init
```

## Architecture Overview

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
| `tui/` | Terminal UI for interactive serial console management |
| `negotiation/` | Auto-negotiation for serial port configuration |
| `port/` | Serial port abstractions and utilities |
| `service/` | Service layer for MCP/REST integration |

### Data Flow (MCP Mode)

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

SQLite backend with two tables:
- `sessions`: UUID-based session IDs, device info, timestamps
- `messages`: Messages with extended metadata (direction, features, latency_ms)

Falls back to in-memory SQLite if on-disk DB unavailable.

## Interface Options

The project supports multiple interfaces for different use cases:

- **MCP**: LLM agent integration via Model Context Protocol (default)
- **REST API**: Web clients, HTTP-based automation, test frameworks
- **stdio**: Simple scripting, legacy integrations, command-line tools
- **WebSocket**: Real-time streaming for monitoring applications

All interfaces share the same service layer and are equally supported. Feature-gating allows minimal builds tailored to specific needs.

## Feature Flags

```toml
[features]
default = ["mcp"]                   # Default: MCP-only for minimal builds
mcp = ["rust-mcp-sdk"]              # Official MCP SDK for LLM integration
rest-api = ["axum"]                 # HTTP API (opt-in for web clients/testing)
legacy-stdio = []                   # Legacy stdio interface (opt-in for scripting)
async-serial = ["tokio-serial"]     # Async serial support
auto-negotiation = ["async-serial"] # Auto-negotiation
websocket = ["rest-api", "tokio-stream"]
openapi = ["utoipa", "utoipa-swagger-ui"]
tui = ["ratatui", "crossterm"]      # Terminal UI
scripting = ["tui", "rhai"]         # Scripting in TUI
hot-reload = ["notify"]             # Hot-reload for TUI
hardware-tests = []                 # Real serial device tests
```

**Build Examples:**
```bash
# MCP-only (default, minimal binary)
cargo build --release

# With REST API for web clients
cargo build --release --features rest-api

# With stdio for scripting
cargo build --release --features legacy-stdio

# Full feature set
cargo build --release --all-features
```

## MCP Tools

### Serial Control
- `list_ports` - Enumerate available system serial ports
- `list_ports_extended` - Extended port information
- `open_port` - Open port with full configuration
- `write` - Write UTF-8 text (auto-appends terminator if configured)
- `read` - Read up to 1024 bytes (trims terminator if present)
- `close` - Close port (idempotent)
- `status` - Return structured state and configuration
- `metrics` - Return cumulative IO counters & timing
- `reconfigure_port` - Reconfigure open port

### Session Analytics
- `create_session` - Create persistent session log
- `append_message` - Append message with extended metadata
- `list_sessions` - List all sessions with filtering (open/closed) and optional limit
- `close_session` - Close a session by marking it as closed
- `list_messages` - List messages (ascending order)
- `list_messages_range` - List messages with cursor-based pagination (after_message_id)
- `export_session` - Export full session JSON
- `filter_messages` - Filter by role/feature/direction
- `feature_index` - Aggregate feature tag counts
- `session_stats` - Session statistics

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `SESSION_DB_URL` | SQLite database URL (default: `sqlite://sessions.db`) |
| `MCP_DISABLE_HEARTBEAT` | Suppress startup heartbeat notification |
| `MCP_DEBUG_BOOT` | Emit debug boot marker on startup |
| `RUST_LOG` | Tracing filter (default: `info`) |
| `SERIAL_TEST_PORT` | Specify real serial port for integration tests |
| `RUST_COMM_CONFIG` | Path to config file (overrides default resolution) |

## Testing Strategy

### Test Layers

1. **Unit Tests** - Pure logic, terminator handling, idle timeout decisions
   ```bash
   cargo test --lib
   ```

2. **Session Store Tests** - Database CRUD with in-memory SQLite
   ```bash
   cargo test session::
   ```

3. **Integration Tests** - MCP stdio protocol with binary spawn
   ```bash
   cargo test --test '*'
   ```

4. **Hardware Tests** - Real serial device tests (optional)
   ```bash
   # Set TEST_PORT environment variable first
   cargo test --features hardware-tests -- --ignored
   ```

### Hardware Testing

Real serial hardware not guaranteed in CI. Tests skip device operations if `SERIAL_TEST_PORT` unset. Use loopback virtual serial pairs for full-duplex tests.

### Coverage

```bash
# Generate HTML coverage report
cargo llvm-cov --all-features --html --open

# Generate LCOV for CI
cargo llvm-cov --all-features --lcov --output-path lcov.info
```

## Error Handling

- MCP tools return `CallToolError` variants (`invalid_arguments`, `unknown_tool`, `from_message`)
- No `unwrap()`/`expect()` in runtime paths (panic-free)
- Serial conflicts surfaced as user-correctable tool errors
- Session DB failures trigger logged warning + in-memory fallback

**Common Error Scenarios:**
- `Port already open` - Call `close` first or proceed with operations
- `invalid_arguments` - Correct the offending field(s) before retrying
- Permission denied - Wait briefly, retry with same params
- Timeout with 0 bytes - Not an error, optionally retry

## Protocol Notes

- **Transport:** Newline-delimited JSON-RPC 2.0 over stdio
- **Tool invocation:** Use `tools/call` method (not legacy `callTool`)
- **Heartbeat:** Server emits startup heartbeat (disable with `MCP_DISABLE_HEARTBEAT=1`)
- **Idle auto-close:** Configured via `idle_disconnect_ms`; triggers on `read` when threshold exceeded

## Configuration

Configuration file resolution order:
1. `RUST_COMM_CONFIG` environment variable
2. `./config.toml` (current directory)
3. `~/.config/rust-comm/config.toml` (Linux/macOS)
4. `%APPDATA%\rust-comm\config.toml` (Windows)
5. Built-in defaults

See `config.toml.example` for full configuration schema.

## Cross-Platform Builds

```bash
# Windows (native)
just build-windows

# Linux x64 (requires cross)
just build-linux-x64

# Linux ARM64 (requires cross)
just build-linux-arm64

# macOS x64
just build-macos-x64

# macOS ARM64 (Apple Silicon)
just build-macos-arm64

# All platforms
just build-all-platforms
```

## Documentation

```bash
# Build documentation
just doc

# Build and open documentation
just doc-open
```

## Release Process

```bash
# Run all checks
just release-check

# Create release tag
just release-tag <version>

# Show current version
just version
```

## Important Conventions

### Error Handling
- All runtime code must handle errors gracefully (no `unwrap()`/`expect()`)
- Use `CallToolError` for MCP tool failures
- Return structured error responses with actionable guidance

### Port State Management
- Only one port managed at a time (intentional design choice)
- `close` is idempotent - always succeeds
- `read` timeout returns 0 bytes (not an error)
- Auto-close triggers on `read` when `idle_disconnect_ms` exceeded

### Session Persistence
- Use extended metadata: `direction`, `features`, `latency_ms`
- Feature tags are space/comma separated tokens
- `feature_index` aggregates across all session messages

### Code Style
- Format with `cargo fmt` before committing
- All clippy warnings must be resolved (`-D warnings`)
- Add tests for new functionality
- Update relevant documentation

## Development Tips

### Running a Single Test
```bash
cargo test test_name -- --nocapture
```

### Debugging MCP Communication
```bash
# Enable debug logging
RUST_LOG=debug cargo run --features mcp

# Emit debug boot marker
MCP_DEBUG_BOOT=1 cargo run --features mcp
```

### Testing with Mock Serial Ports

On Windows, use `com0com` for virtual serial port pairs.
On Linux, use `socat` to create pseudo-terminals:
```bash
socat -d -d pty,raw,echo=0 pty,raw,echo=0
```

### Hot-Reload Development
```bash
# Watch for changes and rebuild
just watch

# Watch and run TUI
just watch-tui
```

## CI/CD Integration

The project uses GitHub Actions for CI. Workflows check:
- Code formatting (`cargo fmt --check`)
- Clippy lints (`cargo clippy -- -D warnings`)
- All tests (`cargo test --all-features`)
- Dependency audits (`cargo audit`, `cargo deny check`)

Pre-commit hooks automatically run these checks locally when installed via `make hooks-install`.

## Additional Resources

- **ARCHITECTURE.md** - Internal systems overview
- **TESTING.md** - Comprehensive testing strategy
- **PROTOCOL.md** - Serial communication protocol details
- **DEVELOPMENT.md** - Extended development guide
- **llms.txt** - Succinct agent guidance for LLM integration
- **CONTRIBUTING.md** - Contribution guidelines
- **CHANGELOG.md** - Version history

## Support

For issues and feature requests, see the GitHub repository issue tracker.
