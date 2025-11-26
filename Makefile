# Makefile for Serial MCP Server
# Provides ergonomic wrappers around cargo + tooling

CARGO ?= cargo
FEATURES ?= --all-features
BIN := serial_mcp_agent
TARGET_DIR := targets

.PHONY: all build release check clippy fmt test test-unit test-integration test-all coverage coverage-ci audit deny bench run metrics clean precommit help db-init hooks-install hooks-uninstall hooks-test setup-dev

all: build

help:
	@echo "Common targets:"
	@echo "  make build           - debug build"
	@echo "  make release         - optimized build"
	@echo "  make test            - run all tests"
	@echo "  make test-unit       - run unit tests only"
	@echo "  make test-integration - run integration tests only"
	@echo "  make coverage        - generate coverage report (HTML)"
	@echo "  make coverage-ci     - generate coverage for CI (LCOV)"
	@echo "  make check           - cargo check"
	@echo "  make clippy          - clippy (deny warnings)"
	@echo "  make fmt             - format"
	@echo "  make audit           - cargo audit (if installed)"
	@echo "  make deny            - cargo deny check"
	@echo "  make bench           - run benchmarks"
	@echo "  make run             - run in stdio MCP mode"
	@echo "  make precommit       - run pre-commit hook tasks"
	@echo ""
	@echo "Git hooks:"
	@echo "  make hooks-install   - install git hooks"
	@echo "  make hooks-uninstall - remove git hooks"
	@echo "  make hooks-test      - test git hooks"
	@echo "  make setup-dev       - full dev environment setup"

build:
	$(CARGO) build $(FEATURES)

release:
	$(CARGO) build --release $(FEATURES)

check:
	$(CARGO) check $(FEATURES)

clippy:
	$(CARGO) clippy $(FEATURES) -- -D warnings

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

test:
	$(CARGO) test $(FEATURES)

# Run unit tests only (library tests)
test-unit:
	$(CARGO) test --lib $(FEATURES)

# Run integration tests only
test-integration:
	$(CARGO) test --test '*' $(FEATURES)

# Run all tests including doc tests
test-all: test-unit test-integration
	$(CARGO) test --doc $(FEATURES)

# Generate HTML coverage report (requires cargo-llvm-cov)
coverage:
	@if command -v cargo-llvm-cov >/dev/null 2>&1; then \
		cargo llvm-cov $(FEATURES) --workspace --html --open; \
	else \
		echo "cargo-llvm-cov not installed. Run: cargo install cargo-llvm-cov"; \
	fi

# Generate LCOV coverage for CI
coverage-ci:
	@if command -v cargo-llvm-cov >/dev/null 2>&1; then \
		cargo llvm-cov $(FEATURES) --workspace --lcov --output-path lcov.info; \
	else \
		echo "cargo-llvm-cov not installed"; exit 1; \
	fi

bench:
	$(CARGO) bench

audit:
	@if command -v cargo-audit >/dev/null 2>&1; then cargo audit; else echo "cargo-audit not installed"; fi

deny:
	@if command -v cargo-deny >/dev/null 2>&1; then cargo deny check; else echo "cargo-deny not installed"; fi

run:
	$(CARGO) run --features mcp -- $(RUN_ARGS)

# Ensure (create/migrate) the session database at default path or override with DB_URL env variable.
db-init:
	SESSION_DB_URL?=sqlite://sessions.db
	$(CARGO) run --bin init_db --quiet -- $$SESSION_DB_URL || true

metrics: run

clean:
	$(CARGO) clean
	@rm -rf $(TARGET_DIR)

precommit: fmt-check clippy test deny
	@echo "Pre-commit checks passed."

# Git hooks management
hooks-install:
	@echo "🔗 Installing git hooks..."
	@chmod +x .githooks/pre-commit .githooks/commit-msg .githooks/pre-push
	@git config core.hooksPath .githooks
	@echo "✅ Git hooks installed!"
	@echo ""
	@echo "Active hooks:"
	@echo "  pre-commit  - Format, clippy, tests, deny"
	@echo "  commit-msg  - Enforce conventional commits"
	@echo "  pre-push    - Full test suite + release build"

hooks-uninstall:
	@echo "🔓 Removing custom hooks path..."
	@git config --unset core.hooksPath || true
	@echo "✅ Git hooks uninstalled!"

hooks-test:
	@echo "🧪 Testing pre-commit hook..."
	@.githooks/pre-commit

# Full development environment setup
setup-dev:
	@echo "🛠️  Running development environment setup..."
	@bash scripts/setup-dev.sh
