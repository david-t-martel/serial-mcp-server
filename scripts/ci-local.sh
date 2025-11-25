#!/bin/bash
# Local CI/CD verification script
# Run this before pushing to ensure CI will pass

set -e  # Exit on error

echo "================================"
echo "Running Local CI/CD Checks"
echo "================================"
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track failures
FAILURES=0

# Helper function to run checks
run_check() {
    local name=$1
    shift
    echo -e "${YELLOW}Running: $name${NC}"
    if "$@"; then
        echo -e "${GREEN}✓ $name passed${NC}"
        echo ""
    else
        echo -e "${RED}✗ $name failed${NC}"
        echo ""
        FAILURES=$((FAILURES + 1))
    fi
}

# 1. Format check
run_check "Code Formatting" cargo fmt --all --check

# 2. Clippy
run_check "Clippy Lints" cargo clippy --all-targets --all-features -- -D warnings

# 3. Tests
run_check "Unit Tests" cargo test --all-features --verbose

# 4. Doc tests
run_check "Documentation Tests" cargo test --doc --all-features

# 5. Security audit (if tools installed)
if command -v cargo-deny &> /dev/null; then
    run_check "Cargo Deny" cargo deny check
else
    echo -e "${YELLOW}⚠ cargo-deny not installed, skipping${NC}"
    echo "  Install with: cargo install cargo-deny"
    echo ""
fi

if command -v cargo-audit &> /dev/null; then
    run_check "Cargo Audit" cargo audit
else
    echo -e "${YELLOW}⚠ cargo-audit not installed, skipping${NC}"
    echo "  Install with: cargo install cargo-audit"
    echo ""
fi

# 6. Build release
run_check "Release Build" cargo build --release --all-features

# Summary
echo "================================"
if [ $FAILURES -eq 0 ]; then
    echo -e "${GREEN}All checks passed!${NC}"
    echo "You're ready to push your changes."
    exit 0
else
    echo -e "${RED}$FAILURES check(s) failed${NC}"
    echo "Please fix the issues before pushing."
    exit 1
fi
