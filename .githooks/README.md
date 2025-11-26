# Git Hooks for rust-comm

This directory contains custom git hooks for the rust-comm serial MCP server project.

## Installation

### Quick Setup
```bash
make hooks-install
```

### Manual Setup
```bash
chmod +x .githooks/*
git config core.hooksPath .githooks
```

### Full Development Environment Setup
```bash
# On Linux/macOS/WSL
bash scripts/setup-dev.sh

# On Windows PowerShell
.\scripts\setup-dev.ps1
```

## Available Hooks

### pre-commit
Runs before each commit to ensure code quality:
- **Format Check**: Ensures code is properly formatted with `rustfmt`
- **Clippy**: Runs linter with warnings as errors
- **Unit Tests**: Runs library tests (`cargo test --lib`)
- **Documentation**: Ensures docs build without errors
- **Cargo Deny**: Checks dependencies, licenses, and advisories (if installed)
- **Cargo Audit**: Checks for security vulnerabilities (if installed)

**Bypass** (not recommended):
```bash
git commit --no-verify
```

### commit-msg
Enforces conventional commit message format:
```
<type>(<scope>): <description>

Types:
  feat:     New feature
  fix:      Bug fix
  docs:     Documentation changes
  style:    Code style (formatting, etc.)
  refactor: Code refactoring
  perf:     Performance improvement
  test:     Test changes
  chore:    Maintenance
  ci:       CI/CD changes
  build:    Build system changes
  revert:   Revert previous commit
```

**Examples**:
```bash
git commit -m "feat(mcp): add list_ports_extended tool"
git commit -m "fix(serial): handle connection timeout"
git commit -m "docs: update README installation steps"
```

### pre-push
Runs before pushing to remote (more comprehensive):
- **All Tests**: Unit + integration tests
- **Release Build**: Ensures release mode compiles
- **TODO/FIXME Check**: Warns about pending work
- **Debug Print Check**: Warns about println! usage

**Bypass** (not recommended):
```bash
git push --no-verify
```

## Testing Hooks

Test the pre-commit hook without making a commit:
```bash
make hooks-test
# or
.githooks/pre-commit
```

## Uninstalling Hooks

```bash
make hooks-uninstall
```

## Required Tools

### Essential (auto-installed by setup-dev.sh):
- **cargo-deny**: Dependency auditing and license checks
- **cargo-audit**: Security vulnerability scanning
- **cargo-llvm-cov**: Code coverage reports

### Optional but Recommended:
- **cargo-watch**: Auto-rebuild on file changes
- **cargo-expand**: Macro expansion debugging

### Install Manually:
```bash
cargo install cargo-deny cargo-audit cargo-llvm-cov
cargo install cargo-watch cargo-expand  # optional
```

## Integration with Make

The hooks integrate seamlessly with the Makefile:

```bash
make precommit       # Run same checks as pre-commit hook
make test            # Run tests
make fmt             # Format code
make clippy          # Run linter
make deny            # Check dependencies
make audit           # Security audit
```

## CI/CD Integration

The same checks run in CI/CD pipelines. Local hooks ensure you catch issues before pushing.

## Troubleshooting

### Hooks Not Running
```bash
# Check hooks path configuration
git config core.hooksPath

# Should output: .githooks
# If not, run: make hooks-install
```

### Permission Denied
```bash
chmod +x .githooks/*
```

### Cargo Tools Missing
```bash
bash scripts/setup-dev.sh
```

## Performance Tips

Hooks are designed to be fast:
- **pre-commit**: Runs only unit tests (~seconds)
- **pre-push**: Runs full test suite (~minutes)

If hooks are too slow:
1. Use `git commit --no-verify` sparingly
2. Fix issues before committing to reduce iterations
3. Run `make precommit` manually before committing

## Customization

Edit hook scripts in `.githooks/` to customize behavior. After editing:
```bash
chmod +x .githooks/pre-commit  # ensure executable
```

## Best Practices

1. **Run `make fmt` before committing** to auto-fix formatting
2. **Use conventional commit format** for better changelogs
3. **Keep commits focused** - one logical change per commit
4. **Test locally** before pushing to save CI time
5. **Don't bypass hooks** unless absolutely necessary

## Windows Compatibility

Hooks are bash scripts and work best in:
- Git Bash (recommended)
- WSL (Windows Subsystem for Linux)
- Cygwin/MSYS2

On native Windows PowerShell, use:
```powershell
make precommit  # Instead of running hooks directly
```
