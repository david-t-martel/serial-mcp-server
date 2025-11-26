# Development environment setup script for rust-comm (Windows PowerShell)
# Installs required tools and configures git hooks

$ErrorActionPreference = "Stop"

Write-Host "🛠️  Setting up development environment for rust-comm..." -ForegroundColor Cyan
Write-Host ""

# Check if cargo is installed
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ cargo not found. Please install Rust from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

Write-Host "📦 Installing required cargo tools..." -ForegroundColor Cyan
Write-Host ""

# Install cargo-deny for dependency auditing
if (-not (Get-Command cargo-deny -ErrorAction SilentlyContinue)) {
    Write-Host "Installing cargo-deny..."
    cargo install cargo-deny
} else {
    Write-Host "✓ cargo-deny already installed" -ForegroundColor Green
}

# Install cargo-audit for security advisories
if (-not (Get-Command cargo-audit -ErrorAction SilentlyContinue)) {
    Write-Host "Installing cargo-audit..."
    cargo install cargo-audit
} else {
    Write-Host "✓ cargo-audit already installed" -ForegroundColor Green
}

# Install cargo-llvm-cov for code coverage
if (-not (Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue)) {
    Write-Host "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
} else {
    Write-Host "✓ cargo-llvm-cov already installed" -ForegroundColor Green
}

# Install cargo-watch for development (optional but useful)
if (-not (Get-Command cargo-watch -ErrorAction SilentlyContinue)) {
    Write-Host "Installing cargo-watch (optional)..."
    cargo install cargo-watch
} else {
    Write-Host "✓ cargo-watch already installed" -ForegroundColor Green
}

# Install cargo-expand for macro debugging (optional)
if (-not (Get-Command cargo-expand -ErrorAction SilentlyContinue)) {
    Write-Host "Installing cargo-expand (optional)..."
    cargo install cargo-expand
} else {
    Write-Host "✓ cargo-expand already installed" -ForegroundColor Green
}

Write-Host ""
Write-Host "🔗 Setting up git hooks..." -ForegroundColor Cyan

# Configure git to use custom hooks directory
git config core.hooksPath .githooks

Write-Host "✓ Git hooks configured" -ForegroundColor Green
Write-Host ""

Write-Host "✅ Development environment setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Available make targets:"
Write-Host "  make build           - Build in debug mode"
Write-Host "  make release         - Build in release mode"
Write-Host "  make test            - Run all tests"
Write-Host "  make test-unit       - Run unit tests only"
Write-Host "  make clippy          - Run clippy lints"
Write-Host "  make fmt             - Format code"
Write-Host "  make precommit       - Run pre-commit checks"
Write-Host "  make coverage        - Generate coverage report"
Write-Host "  make deny            - Check dependencies"
Write-Host "  make audit           - Security audit"
Write-Host ""
Write-Host "Git hooks active:"
Write-Host "  pre-commit           - Format, clippy, tests, deny"
Write-Host "  commit-msg           - Enforce conventional commits"
Write-Host "  pre-push             - Full test suite + release build"
Write-Host ""
Write-Host "Note: Git hooks work best in Git Bash or WSL on Windows"
Write-Host ""
