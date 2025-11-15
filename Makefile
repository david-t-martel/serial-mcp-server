# Makefile for Serial MCP Server
# Provides ergonomic wrappers around cargo + tooling

CARGO ?= cargo
FEATURES ?= --all-features
BIN := serial_mcp_agent
TARGET_DIR := targets

.PHONY: all build release check clippy fmt test audit deny bench run metrics clean precommit help db-init

all: build

help:
	@echo "Common targets:"
	@echo "  make build        - debug build"
	@echo "  make release      - optimized build"
	@echo "  make test         - run all tests"
	@echo "  make check        - cargo check"
	@echo "  make clippy       - clippy (deny warnings)"
	@echo "  make fmt          - format"
	@echo "  make audit        - cargo audit (if installed)"
	@echo "  make deny         - cargo deny check"
	@echo "  make bench        - run benchmarks"
	@echo "  make run          - run in stdio MCP mode"
	@echo "  make precommit    - run pre-commit hook tasks"

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
