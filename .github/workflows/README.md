# GitHub Actions CI/CD Workflows

This directory contains automated workflows for the serial_mcp_agent project.

## Workflows

### ci.yml - Main CI/CD Pipeline

Comprehensive build, test, and release automation pipeline.

#### Triggers
- **Push to main**: Runs on every commit to main branch
- **Pull Requests**: Runs on all PRs targeting main
- **Manual Dispatch**: Can be triggered manually from GitHub Actions tab

#### Jobs

##### 1. Lint (Fast Feedback)
- **Runtime**: ~2-3 minutes
- **Checks**:
  - Code formatting with `rustfmt`
  - Linting with `clippy` (all warnings treated as errors)
- **Runs on**: Ubuntu Latest

##### 2. Test (Cross-Platform)
- **Runtime**: ~5-8 minutes per platform
- **Platforms**: Ubuntu, Windows, macOS
- **Tests**:
  - Unit tests with `cargo test`
  - Documentation tests
  - All features enabled
- **Strategy**: Fail-fast disabled (continues testing other platforms if one fails)

##### 3. Security Audit
- **Runtime**: ~3-5 minutes
- **Tools**:
  - `cargo-deny`: License compliance, banned crates, advisories
  - `cargo-audit`: Known vulnerability scanning via RustSec database
- **Runs on**: Ubuntu Latest
- **Blocking**: Fails CI on vulnerabilities

##### 4. Build Release Binaries
- **Runtime**: ~5-10 minutes per target
- **Targets**:
  - Linux x86_64 (`x86_64-unknown-linux-gnu`)
  - Windows x86_64 (`x86_64-pc-windows-msvc`)
  - macOS x86_64 (`x86_64-apple-darwin`)
  - macOS ARM64 (`aarch64-apple-darwin` - Apple Silicon)
- **Features**: All features enabled
- **Optimizations**:
  - Release profile with optimizations
  - Binaries stripped on Linux/macOS (smaller size)
- **Artifacts**: Uploaded as GitHub Actions artifacts (90-day retention)

##### 5. Code Coverage (Optional)
- **Runtime**: ~5-7 minutes
- **Tool**: `cargo-llvm-cov`
- **Output**: LCOV format for Codecov integration
- **Integration**: Uploads to Codecov (requires `CODECOV_TOKEN` secret)

##### 6. Release (Tag-Triggered)
- **Trigger**: Only on Git tags matching `v*` (e.g., `v3.1.0`)
- **Actions**:
  - Downloads all platform binaries
  - Creates GitHub Release
  - Attaches binaries as release assets
  - Auto-generates release notes

## Performance Optimizations

### Caching Strategy
- **Tool**: `Swatinem/rust-cache@v2`
- **Caches**:
  - Cargo registry index
  - Cargo registry cache
  - Target directory artifacts
- **Shared Keys**: Job-specific keys to maximize cache hits
- **Benefits**: 2-5x faster builds after first run

### Build Optimizations
```yaml
CARGO_INCREMENTAL: 0          # Faster clean builds
CARGO_REGISTRIES_CRATES_IO_PROTOCOL: sparse  # Faster dependency downloads
```

## Usage Examples

### Running Workflows Locally

#### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install tools
cargo install cargo-deny cargo-audit cargo-llvm-cov
rustup component add rustfmt clippy llvm-tools-preview
```

#### Lint
```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
```

#### Test
```bash
cargo test --all-features --verbose
cargo test --doc --all-features
```

#### Security Audit
```bash
cargo deny check
cargo audit
```

#### Build Release
```bash
# Current platform
cargo build --release --all-features

# Specific target
cargo build --release --all-features --target x86_64-unknown-linux-gnu

# Strip binary (Linux/macOS)
strip target/release/serial_mcp_agent
```

#### Code Coverage
```bash
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
```

### Creating a Release

1. **Update version** in `Cargo.toml`:
   ```toml
   version = "3.2.0"
   ```

2. **Commit changes**:
   ```bash
   git add Cargo.toml
   git commit -m "chore: bump version to 3.2.0"
   ```

3. **Create and push tag**:
   ```bash
   git tag v3.2.0
   git push origin main --tags
   ```

4. **Workflow automatically**:
   - Builds binaries for all platforms
   - Creates GitHub Release
   - Attaches binaries as assets

### Manual Workflow Dispatch

1. Go to **Actions** tab in GitHub
2. Select **CI/CD Pipeline** workflow
3. Click **Run workflow**
4. Select branch
5. Click **Run workflow** button

## Troubleshooting

### Cache Issues
If builds fail due to corrupted cache:
1. Go to **Actions** > **Caches**
2. Delete problematic caches
3. Re-run workflow

### Security Audit Failures

#### cargo-deny failures
Check `deny.toml` configuration:
- Add exceptions for known issues
- Update allowed licenses if needed
- Review banned crates

#### cargo-audit failures
Update dependencies with security patches:
```bash
cargo update
cargo audit
```

### Platform-Specific Build Failures

#### Windows
- Check MSVC toolchain installation
- Verify Windows-specific dependencies

#### macOS
- Cross-compilation for ARM64 on x86_64 runners requires setup
- May need Xcode command-line tools

#### Linux
- Check for missing system dependencies
- Verify libc compatibility

## Required Secrets

### Optional (for full features)

#### CODECOV_TOKEN
For code coverage reporting:
1. Sign up at [codecov.io](https://codecov.io)
2. Add repository
3. Copy upload token
4. Add as GitHub secret: `CODECOV_TOKEN`

## Best Practices

### Before Pushing
1. Run local tests: `cargo test --all-features`
2. Check formatting: `cargo fmt --all --check`
3. Run clippy: `cargo clippy --all-targets --all-features`
4. Audit dependencies: `cargo deny check`

### Pull Request Checklist
- [ ] All tests pass
- [ ] Code is formatted
- [ ] No clippy warnings
- [ ] Security audit passes
- [ ] Documentation updated

### Release Checklist
- [ ] Version bumped in Cargo.toml
- [ ] CHANGELOG.md updated
- [ ] Tests pass on all platforms
- [ ] Security audit clean
- [ ] Tag follows semver (vMAJOR.MINOR.PATCH)

## Monitoring

### Workflow Status Badge
Add to README.md:
```markdown
![CI/CD Pipeline](https://github.com/YOUR_USERNAME/rust-comm/workflows/CI%2FCD%20Pipeline/badge.svg)
```

### Code Coverage Badge (if Codecov enabled)
```markdown
[![codecov](https://codecov.io/gh/YOUR_USERNAME/rust-comm/branch/main/graph/badge.svg)](https://codecov.io/gh/YOUR_USERNAME/rust-comm)
```

## Performance Metrics

Typical run times (with warm cache):
- **Lint**: 2-3 minutes
- **Test (per platform)**: 3-5 minutes
- **Security**: 3-4 minutes
- **Build (per platform)**: 4-6 minutes
- **Total CI time**: ~15-20 minutes

First run (cold cache): ~25-35 minutes
