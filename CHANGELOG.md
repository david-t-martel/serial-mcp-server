# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [3.1.0] - 2025-01-XX

### Added

- **Metrics Tool**: New `metrics` tool returning cumulative IO counters and timing (`bytes_read_total`, `bytes_written_total`, `idle_close_count`, `open_duration_ms`, `last_activity_ms`).
- **Session Analytics Tools**:
  - `filter_messages`: Filter session messages by role, feature substring, or direction with optional limit.
  - `feature_index`: Aggregate feature tag counts across session messages for analysis.
  - `session_stats`: Lightweight stats endpoint returning message count, last ID, and rate metrics.
- **Extended Port Discovery**: `list_ports_extended` tool providing enriched USB/Bluetooth/PCI metadata including VID/PID, manufacturer, product name, and serial number.
- **Port Reconfiguration**: `reconfigure_port` tool to reopen a serial port with new configuration, resetting runtime metrics without requiring explicit close/open sequence.
- **Extended Message Metadata**: Session messages now support `direction` (tx/rx/agent), `features` (space/comma-separated tags), and `latency_ms` fields for enhanced observability.
- **Cumulative IO Counters**: `PortState` now tracks `bytes_read_total`, `bytes_written_total`, and timing metrics across the port lifecycle.
- **Structured Idle Auto-Close Event**: When idle timeout triggers port closure, `read` returns structured content with `event: auto_close`, `reason: idle_timeout`, `idle_ms`, and `idle_close_count`.
- **Session Persistence with SQLite**: Full session storage backend using SQLite with automatic schema migration and in-memory fallback for read-only environments.

### Changed

- Enhanced `append_message` schema to accept optional `direction`, `features`, and `latency_ms` parameters.
- Improved observability across all MCP tools with consistent structured content responses.

## [3.0.0] - 2025-XX-XX

### Added

- **MCP Protocol Implementation**: Full Machine Control Protocol server using `rust-mcp-sdk` with stdio transport.
- **Core Serial Tools**:
  - `list_ports`: Enumerate available system serial ports.
  - `open_port`: Open and configure serial port with full parameter control (baud rate, data bits, parity, stop bits, flow control, terminator, idle timeout).
  - `write`: Write UTF-8 data to open port with automatic terminator handling.
  - `read`: Non-blocking read up to 1024 bytes with configurable timeout.
  - `close`: Idempotent port closure.
  - `status`: Structured port state and configuration reporting.
- **Session Management Tools**:
  - `create_session`: Initialize persistent session for device interaction logging.
  - `append_message`: Add timestamped messages to session timeline.
  - `list_messages`: Retrieve session messages in ascending order.
  - `export_session`: Export complete session with metadata as JSON.
- **Panic-Free Runtime**: All `unwrap()` calls removed from production code paths in favor of structured, recoverable error handling.
- **Cross-Platform Support**: Windows (COM ports) and Unix-like systems (/dev/ttyUSB, /dev/ttyACM, etc.).
- **Idle Auto-Disconnect**: Configurable `idle_disconnect_ms` for automatic port closure after inactivity.
- **Feature Flags**: `mcp` (default) for MCP SDK, `rest-api` for optional HTTP surface.

### Changed

- Complete architecture redesign with modular separation (`state`, `mcp`, `session`, `error`, `stdio`).

### Deprecated

- Legacy stdio command interface (non-MCP) returns deprecation errors when `mcp` feature is disabled.

---

[Unreleased]: https://github.com/OWNER/REPO/compare/v3.1.0...HEAD
[3.1.0]: https://github.com/OWNER/REPO/compare/v3.0.0...v3.1.0
[3.0.0]: https://github.com/OWNER/REPO/releases/tag/v3.0.0
