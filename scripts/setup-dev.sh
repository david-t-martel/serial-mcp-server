#!/bin/bash
# Development environment setup script for rust-comm
# Installs required tools and configures git hooks

set -e

echo "🛠️  Setting up development environment for rust-comm..."
echo ""

# Color codes
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "📦 Installing required cargo tools..."
echo ""

# Install cargo-deny for dependency auditing
if ! command -v cargo-deny &> /dev/null; then
    echo "Installing cargo-deny..."
    cargo install cargo-deny
else
    echo -e "${GREEN}✓${NC} cargo-deny already installed"
fi

# Install cargo-audit for security advisories
if ! command -v cargo-audit &> /dev/null; then
    echo "Installing cargo-audit..."
    cargo install cargo-audit
else
    echo -e "${GREEN}✓${NC} cargo-audit already installed"
fi

# Install cargo-llvm-cov for code coverage
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
else
    echo -e "${GREEN}✓${NC} cargo-llvm-cov already installed"
fi

# Install cargo-watch for development (optional but useful)
if ! command -v cargo-watch &> /dev/null; then
    echo "Installing cargo-watch (optional)..."
    cargo install cargo-watch
else
    echo -e "${GREEN}✓${NC} cargo-watch already installed"
fi

# Install cargo-expand for macro debugging (optional)
if ! command -v cargo-expand &> /dev/null; then
    echo "Installing cargo-expand (optional)..."
    cargo install cargo-expand
else
    echo -e "${GREEN}✓${NC} cargo-expand already installed"
fi

echo ""
echo "🔗 Setting up git hooks..."

# Make hooks executable
chmod +x .githooks/pre-commit
chmod +x .githooks/commit-msg
chmod +x .githooks/pre-push

# Configure git to use custom hooks directory
git config core.hooksPath .githooks

echo -e "${GREEN}✓${NC} Git hooks installed and configured"
echo ""

# Run initial checks
echo "🔍 Running initial pre-commit checks..."
echo ""

if .githooks/pre-commit; then
    echo ""
    echo -e "${GREEN}✅ Development environment setup complete!${NC}"
    echo ""
    echo "Available make targets:"
    echo "  make build           - Build in debug mode"
    echo "  make release         - Build in release mode"
    echo "  make test            - Run all tests"
    echo "  make test-unit       - Run unit tests only"
    echo "  make clippy          - Run clippy lints"
    echo "  make fmt             - Format code"
    echo "  make precommit       - Run pre-commit checks"
    echo "  make coverage        - Generate coverage report"
    echo "  make deny            - Check dependencies"
    echo "  make audit           - Security audit"
    echo ""
    echo "Git hooks active:"
    echo "  pre-commit           - Format, clippy, tests, deny"
    echo "  commit-msg           - Enforce conventional commits"
    echo "  pre-push             - Full test suite + release build"
    echo ""
else
    echo ""
    echo -e "${YELLOW}⚠  Initial checks failed. Run 'make precommit' to see details.${NC}"
    echo ""
fi
