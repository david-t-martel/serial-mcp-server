# Git Hooks Quick Start Guide

## 🚀 One-Minute Setup

```bash
# Install hooks
make hooks-install

# Verify installation
bash scripts/verify-hooks.sh
```

Done! Your git hooks are now active.

---

## 📋 What Just Happened?

Git will now automatically run quality checks:

### On Every Commit
- Code formatting check
- Clippy linting
- Unit tests
- Documentation build
- Dependency audit

### On Every Push
- Full test suite
- Release build verification

### On Commit Messages
- Enforces conventional commit format
- Example: `feat(mcp): add new feature`

---

## ✅ Quick Test

```bash
# Make a trivial change
echo "# Test" >> .test

# Try to commit (hooks will run)
git add .test
git commit -m "test: verify hooks work"

# You should see:
# 🔍 Running pre-commit checks...
# 📝 Checking formatting...
# 📎 Running clippy...
# 🧪 Running unit tests...
# ✅ All pre-commit checks passed!
```

---

## 📝 Commit Message Format

**Required format:**
```
<type>(<scope>): <description>
```

**Common types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance

**Examples:**
```bash
git commit -m "feat(mcp): add list_ports_extended tool"
git commit -m "fix(serial): handle timeout correctly"
git commit -m "docs: update README"
```

---

## 🛠️ Daily Workflow

```bash
# 1. Make changes
vim src/main.rs

# 2. Format code
make fmt

# 3. Test locally
make precommit

# 4. Commit
git commit -m "feat(api): add new endpoint"
# ← Hooks run automatically

# 5. Push
git push
# ← More comprehensive checks run
```

---

## 🔧 Common Commands

```bash
# Development
make fmt              # Auto-format code
make precommit        # Run all pre-commit checks
make test             # Run tests

# Hooks
make hooks-install    # Install hooks
make hooks-test       # Test hooks
make hooks-uninstall  # Remove hooks

# Verification
bash scripts/verify-hooks.sh   # Check installation
```

---

## 🚨 Emergency Bypass

**Only use in emergencies:**

```bash
# Skip pre-commit hook
git commit --no-verify

# Skip pre-push hook
git push --no-verify
```

**⚠️ Warning:** Only bypass hooks for:
- Emergency hotfixes
- WIP commits on feature branches
- Known hook issues being fixed

---

## ❌ Troubleshooting

### Hooks not running?
```bash
git config core.hooksPath  # Should show: .githooks
make hooks-install         # Re-install
```

### Permission denied?
```bash
chmod +x .githooks/*
```

### Tools not found?
```bash
bash scripts/setup-dev.sh  # Full setup
```

---

## 📚 More Information

- **Full documentation**: `cat .githooks/README.md`
- **Development guide**: `cat DEVELOPMENT.md`
- **Complete summary**: `cat HOOKS_SETUP_SUMMARY.md`
- **Make targets**: `make help`

---

## ✨ Benefits

✅ Catch issues before committing
✅ Maintain code quality automatically
✅ Consistent commit messages
✅ Faster CI/CD (issues caught locally)
✅ Better code reviews
✅ Professional development workflow

---

**You're all set! Happy coding! 🎉**
