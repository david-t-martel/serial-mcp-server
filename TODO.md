# TODO / Roadmap

## Near-Term

- [ ] Strict framing: remove raw fallback in tests once initialize reliably framed.
- [ ] Add hardware loopback self-test (example binary + optional integration test gated by env) for multi-baud validation.
- [ ] Enrich `list_ports` with extended metadata (VID/PID, manufacturer, description) behind new `port_metadata` field.
- [ ] Add `describe_port` tool returning enriched metadata (future multi-port foundation).
- [ ] Integrate session message rate into `metrics` or add `composite_metrics` tool.
- [ ] Pagination tool: `list_messages_range (session_id, start_after_id, limit)`.
- [ ] Normalize markdown (lint fixes in README / ARCHITECTURE) or adopt markdownlint config.
- [ ] Add unit tests for:
  - Terminator append/trim logic.
  - Idle auto-close (with mocked Instant or configurable clock abstraction).
  - Metrics accumulation after write/read sequences.
  - Session store filtering & feature index.
- [ ] Add integration test invoking MCP tools over stdio (JSON-RPC) to cover open/write/read/metrics/close.
- [ ] Provide a fake serial backend (trait abstraction) to simulate device responses deterministically.
- [ ] Introduce CHANGELOG.md (semantic version entries; already bumped to 3.1.0).
- [ ] Add cargo deny / audit tasks to CI (deny already configured via deny.toml).
- [ ] Implement backlog-driven GitHub Issues template mapping to these tasks.

## Observability Enhancements

- [x] Expose `timeout_streak` in metrics when non-zero (implemented always present).
- [ ] Add error counters (open failures, read timeouts total, parse errors).
- [ ] Provide `status` embedding a concise metrics snapshot (combine or add fast path).

## Session & Analytics

- [ ] Tokenize features into a junction table for precise querying.
- [ ] Add full-text search (FTS5) for message content.
- [ ] Provide session close tool (sets `closed_at`).
- [ ] Add export variant filtering by feature tag.

## Protocol / Tools

- [x] `reconfigure_port` to change selected parameters without manual close/reopen.
- [ ] Binary-safe read/write (base64 or hex mode flag).
- [ ] Streaming / subscribe tool for push notifications (lines or bytes).
- [ ] Multi-port support (map of port_name -> state) with explicit handle tokens.
- [ ] `describe_port` (detailed metadata) after enrichment.

## Performance

- [ ] Dedicated async task for serial IO; reduce mutex hold time.
- [ ] Optional buffered reader for line assembly.
- [ ] Benchmark: throughput (writes/sec), latency distribution (reads) under synthetic load.

## Reliability / Robustness

- [ ] Graceful shutdown signal handling (Ctrl+C) ensuring session DB flush.
- [ ] Configurable max read length (avoid oversized allocations).
- [ ] Configurable retry/backoff policy surfaces for open failures.

## Tooling / DX

- [ ] Provide `make` targets (build, test, lint, fmt, bench, audit, release) (partial; some exist, verify completeness).
- [ ] Git pre-commit hook running fmt, clippy, tests (fast subset), markdown lint, sqlx prepare.
- [ ] Add `sqlx migrate` setup if migrations become necessary (currently auto via SessionStore::new).
- [ ] Parameterize sccache settings via env file `.env`.

## Security

- [ ] Capability allowlist for which MCP tools are enabled via config file.
- [ ] Rate limiting for high-frequency read/write misuse.

## Documentation

- [x] PROTOCOL.md (initial draft).
- [ ] TESTING.md with explicit instructions and sample commands (include loopback harness usage).
- [ ] Architecture diagram (mermaid) for README.
- [ ] Public API stability notes (what is considered stable vs experimental).
- [ ] CHANGELOG.md population & version gate for breaking changes.

## Long-Term / Stretch

- [ ] Web dashboard for live metrics (WASM or Tauri companion).
- [ ] gRPC or WebSocket transport (if MCP standard evolves).
- [ ] Pluggable encoding/decoding pipeline (CBOR / MsgPack).
- [ ] Multi-session concurrent port streaming analytics.

---
Updated on: 2025-10-03
