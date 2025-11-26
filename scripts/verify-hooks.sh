#!/bin/bash
# Verification script for git hooks installation
# Run this to ensure hooks are properly configured

set -e

echo "🔍 Verifying git hooks installation..."
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ERRORS=0

# 1. Check git hooks path
echo "1. Checking git hooks path configuration..."
HOOKS_PATH=$(git config core.hooksPath)
if [ "$HOOKS_PATH" = ".githooks" ]; then
    echo -e "${GREEN}✓${NC} Git hooks path is configured: .githooks"
else
    echo -e "${RED}✗${NC} Git hooks path not configured correctly"
    echo "   Expected: .githooks"
    echo "   Got: $HOOKS_PATH"
    echo "   Run: make hooks-install"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# 2. Check hook files exist
echo "2. Checking hook files exist..."
for hook in pre-commit commit-msg pre-push; do
    if [ -f ".githooks/$hook" ]; then
        echo -e "${GREEN}✓${NC} .githooks/$hook exists"
    else
        echo -e "${RED}✗${NC} .githooks/$hook missing"
        ERRORS=$((ERRORS + 1))
    fi
done
echo ""

# 3. Check hooks are executable
echo "3. Checking hooks are executable..."
for hook in pre-commit commit-msg pre-push; do
    if [ -x ".githooks/$hook" ]; then
        echo -e "${GREEN}✓${NC} .githooks/$hook is executable"
    else
        echo -e "${RED}✗${NC} .githooks/$hook not executable"
        echo "   Run: chmod +x .githooks/$hook"
        ERRORS=$((ERRORS + 1))
    fi
done
echo ""

# 4. Check for required tools
echo "4. Checking for required tools..."

if command -v cargo &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo installed"
else
    echo -e "${RED}✗${NC} cargo not found"
    ERRORS=$((ERRORS + 1))
fi

if command -v cargo-deny &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-deny installed"
else
    echo -e "${YELLOW}⚠${NC}  cargo-deny not installed (optional)"
    echo "   Install: cargo install cargo-deny"
fi

if command -v cargo-audit &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-audit installed"
else
    echo -e "${YELLOW}⚠${NC}  cargo-audit not installed (optional)"
    echo "   Install: cargo install cargo-audit"
fi

if command -v cargo-llvm-cov &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-llvm-cov installed"
else
    echo -e "${YELLOW}⚠${NC}  cargo-llvm-cov not installed (optional)"
    echo "   Install: cargo install cargo-llvm-cov"
fi
echo ""

# 5. Test hook syntax
echo "5. Testing hook script syntax..."
for hook in pre-commit commit-msg pre-push; do
    if bash -n ".githooks/$hook" 2>/dev/null; then
        echo -e "${GREEN}✓${NC} .githooks/$hook syntax valid"
    else
        echo -e "${RED}✗${NC} .githooks/$hook has syntax errors"
        ERRORS=$((ERRORS + 1))
    fi
done
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✅ All checks passed!${NC}"
    echo ""
    echo "Git hooks are properly installed and configured."
    echo ""
    echo "Try a test commit to verify hooks work:"
    echo "  git add -A"
    echo "  git commit -m \"test: verify hooks\""
    echo ""
else
    echo -e "${RED}❌ Found $ERRORS error(s)${NC}"
    echo ""
    echo "Fix the errors above, then run this script again."
    echo "Quick fix: make hooks-install"
    echo ""
    exit 1
fi
