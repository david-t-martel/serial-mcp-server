# Build script for ARM64 Linux cross-compilation (Windows)
# Usage: .\build-arm64.ps1 [-BuildType release|debug|small]

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet('release', 'debug', 'small')]
    [string]$BuildType = 'release'
)

$ErrorActionPreference = "Stop"

$TARGET = "aarch64-unknown-linux-gnu"

Write-Host "Building rust-comm for ARM64 Linux..." -ForegroundColor Cyan
Write-Host "Target: $TARGET" -ForegroundColor Gray
Write-Host "Build type: $BuildType" -ForegroundColor Gray
Write-Host ""

# Check if cross is installed
try {
    $null = Get-Command cross -ErrorAction Stop
} catch {
    Write-Host "Error: 'cross' is not installed." -ForegroundColor Red
    Write-Host "Install with: cargo install cross --git https://github.com/cross-rs/cross" -ForegroundColor Yellow
    exit 1
}

# Check if Docker is running
try {
    $null = docker ps 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Docker not running"
    }
} catch {
    Write-Host "Error: Docker is not running." -ForegroundColor Red
    Write-Host "Please start Docker Desktop and try again." -ForegroundColor Yellow
    exit 1
}

# Build based on type
switch ($BuildType) {
    'release' {
        Write-Host "Building release binary..." -ForegroundColor Green
        cross build --target $TARGET --release
        $BinaryPath = "target\$TARGET\release\serial_mcp_agent"
    }
    'debug' {
        Write-Host "Building debug binary..." -ForegroundColor Green
        cross build --target $TARGET
        $BinaryPath = "target\$TARGET\debug\serial_mcp_agent"
    }
    'small' {
        Write-Host "Building size-optimized binary..." -ForegroundColor Green
        cross build --target $TARGET --profile release-small
        $BinaryPath = "target\$TARGET\release-small\serial_mcp_agent"
    }
}

Write-Host ""
Write-Host "Build complete!" -ForegroundColor Green
Write-Host "Binary location: $BinaryPath" -ForegroundColor Cyan
Write-Host ""

# Show binary info
if (Test-Path $BinaryPath) {
    $size = (Get-Item $BinaryPath).Length
    $sizeInMB = [math]::Round($size / 1MB, 2)
    Write-Host "Binary size: $sizeInMB MB" -ForegroundColor Gray
    Write-Host ""
    Write-Host "To verify architecture on Linux/WSL:" -ForegroundColor Yellow
    Write-Host "  file $BinaryPath" -ForegroundColor White
    Write-Host "  Expected: ELF 64-bit LSB pie executable, ARM aarch64" -ForegroundColor Gray
} else {
    Write-Host "Warning: Binary not found at expected location" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  1. Transfer to ARM64 device: scp $BinaryPath user@device:~/" -ForegroundColor White
Write-Host "  2. Run on device: ./serial_mcp_agent --help" -ForegroundColor White
Write-Host "  3. Verify architecture: file serial_mcp_agent" -ForegroundColor White
Write-Host ""
Write-Host "Or use WSL to verify:" -ForegroundColor Yellow
Write-Host "  wsl file $BinaryPath" -ForegroundColor White
