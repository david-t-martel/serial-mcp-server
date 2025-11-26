# rust-comm Justfile - Modern task runner
# Install: cargo install just
# Usage: just <recipe>

# Default recipe - show available commands
default:
    @just --list

# === Build Targets ===

# Build release binary with default features
build:
    cargo build --release --features "mcp,rest-api,auto-negotiation"

# Build debug binary
build-debug:
    cargo build --features "mcp,rest-api,auto-negotiation"

# Build TUI binary
build-tui:
    cargo build --release --features "tui"

# Build with scripting support
build-tui-full:
    cargo build --release --features "tui,scripting"

# Build small binary (optimized for size)
build-small:
    cargo build --profile release-small --features "mcp,rest-api"

# Build with all features
build-all-features:
    cargo build --release --all-features

# === Cross-Platform Builds ===

# Build for Windows (native)
build-windows:
    cargo build --release --target x86_64-pc-windows-msvc --features "mcp,rest-api,tui,auto-negotiation"

# Build for Linux x64 (requires cross)
build-linux-x64:
    cross build --release --target x86_64-unknown-linux-gnu --features "mcp,rest-api,tui,auto-negotiation"

# Build for Linux ARM64 (requires cross)
build-linux-arm64:
    cross build --release --target aarch64-unknown-linux-gnu --features "mcp,rest-api,tui,auto-negotiation"

# Build for macOS x64
build-macos-x64:
    cargo build --release --target x86_64-apple-darwin --features "mcp,rest-api,tui,auto-negotiation"

# Build for macOS ARM64 (Apple Silicon)
build-macos-arm64:
    cargo build --release --target aarch64-apple-darwin --features "mcp,rest-api,tui,auto-negotiation"

# Build for all platforms (requires cross)
build-all-platforms: build-windows build-linux-x64 build-linux-arm64

# === Testing ===

# Run all unit tests
test:
    cargo test --all-features

# Run tests with verbose output
test-verbose:
    cargo test --all-features -- --nocapture

# Run hardware tests with auto-discovery (Windows)
test-hardware-win config="./config.toml":
    powershell -ExecutionPolicy Bypass ./scripts/run-hardware-tests.ps1 -ConfigPath "{{config}}"

# Run hardware tests with auto-discovery (Linux/macOS)
test-hardware config="./config.toml":
    ./scripts/run-hardware-tests.sh "{{config}}"

# Run hardware tests with specific port
test-hardware-port port baud="115200":
    $env:TEST_PORT="{{port}}"; $env:TEST_BAUD="{{baud}}"; cargo test --release --features auto-negotiation -- --ignored --test-threads=1 --nocapture

# Run tests with coverage
coverage:
    cargo llvm-cov --all-features --html --open

# Run tests with coverage (CI)
coverage-ci:
    cargo llvm-cov --all-features --lcov --output-path lcov.info

# Run benchmarks
bench:
    cargo bench

# === Development ===

# Run MCP server (stdio mode)
run:
    cargo run --release --features "mcp,rest-api"

# Run HTTP server on specified port
run-server port="3000":
    cargo run --release --features "mcp,rest-api" -- --server --port {{port}}

# Run TUI application
tui:
    cargo run --release --bin serial-tui --features "tui"

# Run TUI with scripting
tui-full:
    cargo run --release --bin serial-tui --features "tui,scripting"

# Watch for changes and rebuild
watch:
    cargo watch -x check -x test -x "run --features mcp"

# Watch and run TUI
watch-tui:
    cargo watch -x "run --bin serial-tui --features tui"

# === Utilities ===

# Discover available serial ports
discover:
    cargo run --release --features auto-negotiation -- discover 2>/dev/null || echo "Run with MCP to discover ports"

# List ports (simple)
list-ports:
    cargo run --example port_usage 2>/dev/null || echo "Example not available"

# Create config from example
config-init:
    @cp config.toml.example config.toml 2>/dev/null && echo "Created config.toml" || echo "config.toml already exists"

# Show config locations
config-info:
    @echo "Config file resolution order:"
    @echo "  1. RUST_COMM_CONFIG environment variable"
    @echo "  2. ./config.toml (current directory)"
    @echo "  3. ~/.config/rust-comm/config.toml (Linux/macOS)"
    @echo "  4. %APPDATA%\\rust-comm\\config.toml (Windows)"
    @echo "  5. Built-in defaults"

# === Code Quality ===

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all --check

# Run clippy lints
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all lints
lint: fmt-check clippy

# Fix lint issues automatically
fix:
    cargo fmt --all
    cargo clippy --fix --all-targets --all-features --allow-dirty

# Check for outdated dependencies
outdated:
    cargo outdated

# Audit dependencies for security issues
audit:
    cargo audit

# === Documentation ===

# Build documentation
doc:
    cargo doc --all-features --no-deps

# Build and open documentation
doc-open:
    cargo doc --all-features --no-deps --open

# === Cleanup ===

# Clean build artifacts
clean:
    cargo clean

# Clean and rebuild
rebuild: clean build

# Remove generated files
clean-all: clean
    rm -rf lcov.info
    rm -rf target/
    rm -f sessions.db

# === Release ===

# Prepare release (run all checks)
release-check: lint test doc
    @echo "All checks passed!"

# Create release tag
release-tag version:
    git tag -a v{{version}} -m "Release v{{version}}"
    git push origin v{{version}}

# Show version
version:
    @cargo pkgid | cut -d# -f2
