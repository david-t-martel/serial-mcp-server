# Local CI/CD verification script for Windows
# Run this before pushing to ensure CI will pass

$ErrorActionPreference = "Stop"

Write-Host "================================" -ForegroundColor Cyan
Write-Host "Running Local CI/CD Checks" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""

$Failures = 0

# Helper function to run checks
function Run-Check {
    param(
        [string]$Name,
        [scriptblock]$Command
    )

    Write-Host "Running: $Name" -ForegroundColor Yellow
    try {
        & $Command
        Write-Host "✓ $Name passed" -ForegroundColor Green
        Write-Host ""
    } catch {
        Write-Host "✗ $Name failed" -ForegroundColor Red
        Write-Host $_.Exception.Message -ForegroundColor Red
        Write-Host ""
        $script:Failures++
    }
}

# 1. Format check
Run-Check "Code Formatting" {
    cargo fmt --all --check
    if ($LASTEXITCODE -ne 0) { throw "Format check failed" }
}

# 2. Clippy
Run-Check "Clippy Lints" {
    cargo clippy --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "Clippy failed" }
}

# 3. Tests
Run-Check "Unit Tests" {
    cargo test --all-features --verbose
    if ($LASTEXITCODE -ne 0) { throw "Tests failed" }
}

# 4. Doc tests
Run-Check "Documentation Tests" {
    cargo test --doc --all-features
    if ($LASTEXITCODE -ne 0) { throw "Doc tests failed" }
}

# 5. Security audit (if tools installed)
if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
    Run-Check "Cargo Deny" {
        cargo deny check
        if ($LASTEXITCODE -ne 0) { throw "Cargo deny failed" }
    }
} else {
    Write-Host "⚠ cargo-deny not installed, skipping" -ForegroundColor Yellow
    Write-Host "  Install with: cargo install cargo-deny"
    Write-Host ""
}

if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
    Run-Check "Cargo Audit" {
        cargo audit
        if ($LASTEXITCODE -ne 0) { throw "Cargo audit failed" }
    }
} else {
    Write-Host "⚠ cargo-audit not installed, skipping" -ForegroundColor Yellow
    Write-Host "  Install with: cargo install cargo-audit"
    Write-Host ""
}

# 6. Build release
Run-Check "Release Build" {
    cargo build --release --all-features
    if ($LASTEXITCODE -ne 0) { throw "Release build failed" }
}

# Summary
Write-Host "================================" -ForegroundColor Cyan
if ($Failures -eq 0) {
    Write-Host "All checks passed!" -ForegroundColor Green
    Write-Host "You're ready to push your changes."
    exit 0
} else {
    Write-Host "$Failures check(s) failed" -ForegroundColor Red
    Write-Host "Please fix the issues before pushing."
    exit 1
}
