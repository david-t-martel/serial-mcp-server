# Contributing to Serial MCP Server

Thank you for your interest in contributing to the Serial MCP Server project. This document provides guidelines for contributing code, reporting issues, and participating in development.

## Table of Contents

- [Development Setup](#development-setup)
- [Build Commands](#build-commands)
- [Code Style](#code-style)
- [Testing Requirements](#testing-requirements)
- [Pull Request Process](#pull-request-process)
- [Code of Conduct](#code-of-conduct)
- [License](#license)

## Development Setup

### Prerequisites

1. **Rust Toolchain**: Install the latest stable Rust via [rustup](https://rustup.rs/):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Required Components**:
   ```bash
   rustup component add rustfmt clippy
   ```

3. **Optional Tools** (recommended for pre-commit checks):
   ```bash
   cargo install cargo-audit cargo-deny
   ```

4. **Platform Requirements**:
   - Windows: Visual Studio Build Tools with C++ workload
   - Linux: `libudev-dev` package for serial port enumeration
   - macOS: Xcode Command Line Tools

### Clone and Build

```bash
git clone <repository-url>
cd rust-comm
make build
```

### Database Initialization (Optional)

Session persistence uses SQLite. The schema is applied automatically on first use, but you can pre-initialize:

```bash
make db-init
# Or with custom path:
SESSION_DB_URL=sqlite://data/sessions.db make db-init
```

## Build Commands

The project uses a Makefile for consistent developer workflows. All targets encode the correct feature flags and quality gates.

| Command | Description |
|---------|-------------|
| `make build` | Debug build with all features |
| `make release` | Optimized release build |
| `make check` | Fast type checking without codegen |
| `make clippy` | Lint with warnings as errors |
| `make fmt` | Format all source files |
| `make fmt-check` | Verify formatting without changes |
| `make test` | Run all tests (unit + integration) |
| `make bench` | Run Criterion benchmarks |
| `make audit` | Security audit (requires cargo-audit) |
| `make deny` | Dependency license/advisory check (requires cargo-deny) |
| `make precommit` | Full pre-commit validation suite |
| `make clean` | Remove build artifacts |
| `make help` | List all available targets |

### Direct Cargo Usage

While Make targets are the authoritative workflows, you can invoke cargo directly:

```bash
# Build with all features
cargo build --all-features

# Run MCP server
cargo run --features mcp

# Generate documentation
cargo doc --no-deps --open
```

## Code Style

### Formatting

All code must be formatted with `rustfmt`. The project uses default rustfmt settings:

```bash
make fmt        # Apply formatting
make fmt-check  # Verify without changes
```

### Linting

Clippy compliance is mandatory. All warnings are treated as errors:

```bash
make clippy
# Equivalent to: cargo clippy --all-features -- -D warnings
```

### Guidelines

1. **No Panics in Runtime Code**: Use `Result` types and proper error handling. Reserve `unwrap()` and `expect()` for tests only.

2. **Explicit Error Types**: Use the project's error types from `src/error.rs` for consistent error handling.

3. **Documentation**: Add doc comments (`///`) for public items. Include examples where helpful.

4. **Async Patterns**: Use `async/await` consistently. Avoid blocking operations in async contexts.

5. **Naming Conventions**:
   - Types: `PascalCase`
   - Functions/methods: `snake_case`
   - Constants: `SCREAMING_SNAKE_CASE`
   - MCP tools: lowercase with underscores (e.g., `list_ports_extended`)

6. **Feature Flags**: Guard optional functionality behind feature flags. Default features are `mcp` and `rest-api`.

## Testing Requirements

### Running Tests

```bash
make test
# Or directly:
cargo test --all-features
```

### Test Categories

1. **Unit Tests**: Located alongside source code in `src/` modules using `#[cfg(test)]` blocks.

2. **Integration Tests**: Located in `tests/` directory:
   - `smoke_stdio.rs` - Basic MCP transport validation
   - `initialize_framing.rs` - Protocol framing tests
   - `session_append_stress.rs` - Session persistence stress tests

3. **Benchmarks**: Located in `benches/` using Criterion:
   ```bash
   make bench
   ```

### Writing Tests

- Test both success and error paths
- Use descriptive test names that explain the scenario
- Mock serial ports when possible to avoid hardware dependencies
- For integration tests requiring real hardware, use `#[ignore]` and document requirements

### Coverage Expectations

While no strict coverage threshold is enforced, aim for:
- Core MCP tool handlers: comprehensive coverage
- Error paths: explicit test cases
- Edge cases: timeout handling, partial reads, malformed input

## Pull Request Process

### Before Submitting

1. **Run Pre-commit Checks**:
   ```bash
   make precommit
   ```
   This runs: `fmt-check`, `clippy`, `test`, and `deny`.

2. **Update Documentation**: If adding features, update README.md and relevant docs.

3. **Add Changelog Entry**: Document changes in CHANGELOG.md under `[Unreleased]`.

### Submission Guidelines

1. **Branch Naming**: Use descriptive branch names:
   - `feat/add-binary-read-tool`
   - `fix/timeout-handling`
   - `docs/improve-configuration`

2. **Commit Messages**: Follow conventional commits:
   - `feat: add reconfigure_port tool`
   - `fix: handle timeout edge case in read`
   - `docs: update MCP tool descriptions`
   - `refactor: extract common port validation`

3. **Pull Request Description**:
   - Summarize the change and motivation
   - Reference related issues
   - List any breaking changes
   - Include testing notes

### Review Process

1. All PRs require at least one approving review
2. CI checks must pass (format, clippy, tests)
3. Breaking changes require discussion before merge
4. Squash commits before merging to maintain clean history

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). We are committed to providing a welcoming and inclusive environment for all contributors.

Key points:
- Be respectful and inclusive
- Focus on constructive feedback
- Assume good intentions
- Report unacceptable behavior to project maintainers

## License

This project is dual-licensed under:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

You may choose either license. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project shall be dual licensed as above, without any additional terms or conditions.

---

Questions? Open an issue or reach out to the maintainers. We appreciate your contributions!
