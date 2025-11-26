# Development Guide

## Quick Start

### Initial Setup
```bash
# Install development tools and configure git hooks
make setup-dev

# Or manually install tools
cargo install cargo-deny cargo-audit cargo-llvm-cov
make hooks-install
```

### Daily Development Workflow
```bash
# 1. Make changes to code

# 2. Format code
make fmt

# 3. Run checks locally
make precommit

# 4. Commit with conventional format
git commit -m "feat(mcp): add new tool"

# 5. Push (triggers full test suite)
git push
```

## Build Commands

### Basic Builds
```bash
make build          # Debug build
make release        # Release build (optimized)
make check          # Quick syntax check
```

### Testing
```bash
make test           # All tests
make test-unit      # Unit tests only (fast)
make test-integration  # Integration tests
make test-all       # Including doc tests
```

### Code Quality
```bash
make fmt            # Auto-format code
make fmt-check      # Check formatting (CI)
make clippy         # Lint with clippy
make precommit      # All pre-commit checks
```

### Coverage
```bash
make coverage       # HTML report (opens browser)
make coverage-ci    # LCOV for CI/CD
```

### Security & Dependencies
```bash
make deny           # Check deps, licenses, advisories
make audit          # Security vulnerabilities
```

### Other
```bash
make bench          # Run benchmarks
make run            # Run in MCP stdio mode
make clean          # Clean build artifacts
make help           # Show all targets
```

## Git Hooks

### Installation
```bash
make hooks-install    # Enable hooks
make hooks-uninstall  # Disable hooks
make hooks-test       # Test pre-commit hook
```

### Hook Behavior

**pre-commit** (runs on `git commit`):
- Format check (`cargo fmt --check`)
- Clippy lints (`cargo clippy -- -D warnings`)
- Unit tests (`cargo test --lib`)
- Doc build check
- Dependency audit (if tools installed)

**commit-msg** (runs on `git commit`):
- Enforces conventional commit format
- Examples:
  ```
  feat(mcp): add new discovery tool
  fix(serial): handle timeout correctly
  docs: update README
  ```

**pre-push** (runs on `git push`):
- Full test suite
- Release build verification
- TODO/FIXME warnings
- Debug print detection

### Bypassing Hooks (NOT RECOMMENDED)
```bash
git commit --no-verify   # Skip pre-commit
git push --no-verify     # Skip pre-push
```

## Commit Message Format

### Structure
```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types
- **feat**: New feature
- **fix**: Bug fix
- **docs**: Documentation only
- **style**: Formatting, missing semicolons, etc.
- **refactor**: Code restructuring
- **perf**: Performance improvement
- **test**: Adding tests
- **chore**: Maintenance tasks
- **ci**: CI/CD changes
- **build**: Build system changes
- **revert**: Revert previous commit

### Scopes (examples)
- `mcp`: MCP server functionality
- `serial`: Serial port handling
- `api`: REST API
- `db`: Database operations
- `cli`: Command-line interface
- `tools`: MCP tools

### Examples
```bash
# New feature
git commit -m "feat(mcp): add list_ports_extended with USB metadata"

# Bug fix
git commit -m "fix(serial): prevent timeout when reading large buffers"

# Documentation
git commit -m "docs: add troubleshooting section to README"

# Performance
git commit -m "perf(serial): optimize read buffer allocation"

# Multiple line
git commit -m "feat(tools): add session persistence

- Add SQLite database for session storage
- Implement session CRUD operations
- Add migration support"
```

## Development Tools

### Required
```bash
# Installed by setup-dev.sh
cargo install cargo-deny        # Dependency auditing
cargo install cargo-audit       # Security scanning
cargo install cargo-llvm-cov    # Coverage reports
```

### Recommended
```bash
cargo install cargo-watch       # Auto-rebuild
cargo install cargo-expand      # Macro debugging
cargo install cargo-edit        # Manage dependencies
cargo install cargo-outdated    # Check for updates
```

### Usage Examples
```bash
# Auto-rebuild on changes
cargo watch -x check -x test

# Expand macros
cargo expand

# Add dependency
cargo add tokio --features full

# Check outdated deps
cargo outdated
```

## Project Structure

```
rust-comm/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library root
│   ├── mcp/              # MCP server implementation
│   ├── serial/           # Serial port handling
│   ├── tools/            # MCP tools
│   └── db/               # Database operations
├── tests/                # Integration tests
├── benches/              # Benchmarks
├── .githooks/            # Custom git hooks
├── scripts/              # Development scripts
├── .cargo/
│   └── config.toml       # Cargo configuration
├── Cargo.toml            # Dependencies
├── rustfmt.toml          # Format config
├── clippy.toml           # Clippy config
├── deny.toml             # Dependency audit config
└── Makefile              # Build automation
```

## Configuration Files

### rustfmt.toml
- Line width: 100 characters
- Tab spaces: 4
- Import organization: Group by std/external/crate
- Comment wrapping enabled

### clippy.toml
- Custom lint configuration
- See file for specific rules

### deny.toml
- Allowed licenses: MIT, Apache-2.0, etc.
- Advisory database for security
- Multiple version detection

### .cargo/config.toml
- Uses `sccache` for faster builds
- Custom target directory: `targets/`
- Release profile: LTO + size optimization
- Windows debug symbols enabled

## Continuous Integration

### Local CI Simulation
```bash
# Run exactly what CI runs
make fmt-check && make clippy && make test-all && make deny
```

### Coverage Requirements
- Aim for >80% code coverage
- View with: `make coverage`
- CI generates LCOV: `make coverage-ci`

## Troubleshooting

### Hooks Not Running
```bash
git config core.hooksPath  # Should show: .githooks
make hooks-install         # Re-install if needed
```

### Build Errors
```bash
make clean          # Clean build artifacts
cargo clean         # Full clean
cargo update        # Update dependencies
```

### Test Failures
```bash
# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --nocapture

# Show test output even on success
cargo test -- --show-output
```

### Slow Builds
```bash
# Check sccache stats
sccache --show-stats

# Ensure sccache is working
echo $RUSTC_WRAPPER  # Should show: sccache

# Clear sccache if needed
sccache --stop-server
```

## Performance Tips

1. **Incremental builds**: Enabled by default in `.cargo/config.toml`
2. **Shared cache**: Uses `sccache` across projects
3. **Parallel compilation**: `-j` flag (automatic)
4. **Release builds**: Use `make release` for deployment

## Best Practices

### Before Committing
1. Run `make fmt` to auto-format
2. Run `make precommit` to verify
3. Write clear commit message
4. Keep commits focused and atomic

### Before Pushing
1. Ensure all tests pass
2. Update documentation if needed
3. Run `make release` to verify
4. Review CHANGELOG impact

### Code Quality
1. Follow Rust API guidelines
2. Write doc comments for public APIs
3. Add tests for new functionality
4. Use clippy suggestions
5. Keep functions small and focused

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [MCP Protocol](https://modelcontextprotocol.io/)
