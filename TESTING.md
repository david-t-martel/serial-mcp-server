# Testing Strategy

This document describes the testing approach for the Serial MCP Server.

## Goals
- Ensure serial port lifecycle correctness (open -> write -> read -> close).
- Verify terminator logic, idle timeout auto-close, and metrics accumulation.
- Validate session persistence, filtering, and feature index aggregation.
- Provide fast feedback (unit tests) plus higher‑level integration (MCP JSON-RPC over stdio).

## Test Layers
1. Unit Tests (pure logic / small scope)
   - Terminator append & trim (helper abstraction around write/read simulation).
   - Idle timeout decision function (introduce a small internal helper or feature gate to inject clock if necessary).
   - Feature index tokenization (split & count) using in-memory DB.
2. Session Store Tests (database)
   - Use `sqlx::SqlitePool` against a temporary file or `sqlite::memory:` connection.
   - Validate migrations idempotency and CRUD flows.
3. Integration Tests (MCP stdio)
   - Spawn the compiled binary with default features.
   - Perform `initialize` + `list_tools` + tool calls (`open_port` may fail w/o a real device; guard behind env var for CI).
   - Use a feature flag or env (`TEST_FAKE_SERIAL=1`) to swap a mock port implementation (future enhancement) for deterministic IO.
4. Smoke Tests
   - Basic process start/stop (already present in `tests/smoke_stdio.rs`).

## Handling Real Serial Devices
Real serial hardware is not guaranteed in CI. Strategies:
- Skip device-dependent tests if `SERIAL_TEST_PORT` env var is unset.
- Allow specifying a loopback virtual serial pair (e.g., `com0com` on Windows) for full duplex tests.
- Future: Implement a `MockSerialPort` that mimics `serialport::SerialPort` trait (gated behind `--cfg test_mock_serial`).

## Metrics Validation
Because metrics counters live inside `PortState`, unit tests can create a synthetic `PortState::Open` variant with a dummy `Box<dyn SerialPort>` wrapper that records writes/reads. Provide an internal test-only module implementing a minimal stub (just capturing buffers in memory) to simulate reads/writes.

## Suggested Helper: Mock Port (future)
```rust
pub struct MockPort { pub read_buf: Vec<u8>, pub written: Vec<u8>, pub timeout: std::time::Duration }
impl serialport::SerialPort for MockPort { /* implement needed trait methods returning Ok/Err appropriately */ }
```
This enables deterministic read responses and error injection (timeouts, partial reads).

## Running Tests
```bash
cargo test --all-features
```
To run only fast unit tests (skip benches):
```bash
cargo test --lib
```
To run a specific test:
```bash
cargo test terminator_append -- --nocapture
```

## Coverage (Optional)
Integrate `cargo llvm-cov` locally for coverage:
```bash
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --html
```

## Lint & Formatting Gates
Pre-commit and CI will run:
- `cargo fmt -- --check`
- `cargo clippy --all-features -- -D warnings`
- `cargo deny check`
- `cargo audit` (optional offline advisory DB)
- Markdown lint (`markdownlint-cli2`) for docs

## Test Data Management
Session tests use ephemeral DB: set env `SESSION_DB_URL=sqlite::memory:` or `sqlite://test_sessions.db?mode=rwc` and remove file after test.

## Future Enhancements
- Property tests (using `proptest`) for argument parsing robustness.
- Time abstraction (trait) to simulate idle time passage deterministically.
- Benchmark assertions (upper bounds on write/read latency) with Criterion statistical thresholds.

## CI Matrix Idea
| Job | Features | Purpose |
|-----|----------|---------|
| lint | all | Style & static analysis |
| unit | mcp | Fast logic tests |
| integration | mcp | Binary spawn & MCP protocol |
| minimal | default-no-rest | Build sanity w/o REST |
| bench (nightly) | mcp | Performance trend tracking |

## Edge Cases Checklist
- Re-open after idle auto-close.
- Write with existing terminator (no duplicate append).
- Read returning 0 bytes (timeout) does not reset last_activity.
- Filtering messages with no matches returns empty array gracefully.
- Feature index with repeated & mixed delimiter tokens.

## Smoke Example (stdout abridged)
See `tests/smoke_stdio.rs` for baseline spawn + help interrogation; extend with JSON-RPC interactions as mocks are added.
