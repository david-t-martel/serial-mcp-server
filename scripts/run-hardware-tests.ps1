<#
.SYNOPSIS
    Run hardware tests for rust-comm using config.toml and auto-discovery.

.DESCRIPTION
    This script reads test configuration from config.toml and runs hardware tests
    against discovered or configured serial ports.

.PARAMETER ConfigPath
    Path to the configuration file. Defaults to ./config.toml

.PARAMETER AutoDiscover
    Force auto-discovery even if a port is specified in config.

.PARAMETER Verbose
    Show verbose test output with --nocapture.

.PARAMETER Port
    Override the test port (bypasses config and discovery).

.PARAMETER Baud
    Override the test baud rate.

.EXAMPLE
    .\run-hardware-tests.ps1

.EXAMPLE
    .\run-hardware-tests.ps1 -ConfigPath .\my-config.toml -Verbose

.EXAMPLE
    .\run-hardware-tests.ps1 -Port COM15 -Baud 9600
#>

param(
    [string]$ConfigPath = ".\config.toml",
    [switch]$AutoDiscover,
    [switch]$VerboseOutput,
    [string]$Port,
    [int]$Baud
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  rust-comm Hardware Test Runner" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Helper function to parse TOML (basic implementation)
function Get-TomlValue {
    param([string]$Content, [string]$Section, [string]$Key)

    $inSection = $false
    foreach ($line in $Content -split "`n") {
        $line = $line.Trim()

        if ($line -match "^\[$Section\]") {
            $inSection = $true
            continue
        }
        elseif ($line -match "^\[" -and $inSection) {
            break
        }

        if ($inSection -and $line -match "^$Key\s*=\s*(.+)$") {
            $value = $Matches[1].Trim()
            # Remove quotes and comments
            $value = $value -replace '^"(.*)"$', '$1'
            $value = $value -replace "^'(.*)'$", '$1'
            $value = $value -replace '\s*#.*$', ''
            return $value.Trim()
        }
    }
    return $null
}

# Load configuration if file exists
$config = @{}
if (Test-Path $ConfigPath) {
    Write-Host "Loading config from: $ConfigPath" -ForegroundColor Gray
    $configContent = Get-Content $ConfigPath -Raw

    $config.Port = Get-TomlValue -Content $configContent -Section "testing" -Key "port"
    $config.Baud = Get-TomlValue -Content $configContent -Section "testing" -Key "baud"
    $config.TimeoutMs = Get-TomlValue -Content $configContent -Section "testing" -Key "timeout_ms"
    $config.LoopbackEnabled = Get-TomlValue -Content $configContent -Section "testing" -Key "loopback_enabled"
    $config.DiscoveryEnabled = Get-TomlValue -Content $configContent -Section "testing.discovery" -Key "enabled"
}
else {
    Write-Host "Config file not found, using defaults and auto-discovery" -ForegroundColor Yellow
    $config.DiscoveryEnabled = "true"
}

# Command-line overrides
if ($Port) { $config.Port = $Port }
if ($Baud) { $config.Baud = $Baud }

# Determine test port
$testPort = $null

if ($config.Port -and -not $AutoDiscover) {
    $testPort = $config.Port
    Write-Host "Using configured port: $testPort" -ForegroundColor Green
}
elseif ($config.DiscoveryEnabled -eq "true" -or $AutoDiscover -or -not $config.Port) {
    Write-Host "Auto-discovering test ports..." -ForegroundColor Cyan

    # Get available ports using PowerShell
    try {
        $ports = [System.IO.Ports.SerialPort]::GetPortNames() | Sort-Object

        if ($ports.Count -eq 0) {
            Write-Error "No serial ports discovered"
            exit 1
        }

        Write-Host "Available ports:" -ForegroundColor Gray
        foreach ($p in $ports) {
            Write-Host "  - $p" -ForegroundColor Gray
        }

        # Filter out system ports (COM1, COM2)
        $validPorts = $ports | Where-Object { $_ -notmatch "^COM[12]$" }

        if ($validPorts.Count -eq 0) {
            Write-Host "No non-system ports available, using first port" -ForegroundColor Yellow
            $testPort = $ports[0]
        }
        else {
            $testPort = $validPorts[0]
        }

        Write-Host "Selected port: $testPort" -ForegroundColor Green
    }
    catch {
        Write-Error "Failed to discover ports: $_"
        exit 1
    }
}

if (-not $testPort) {
    Write-Error "No test port available"
    exit 1
}

# Set test baud rate
$testBaud = if ($config.Baud) { $config.Baud } else { "115200" }
$testTimeout = if ($config.TimeoutMs) { $config.TimeoutMs } else { "2000" }
$loopbackEnabled = if ($config.LoopbackEnabled -eq "true") { "true" } else { "false" }

# Set environment variables
$env:TEST_PORT = $testPort
$env:TEST_BAUD = $testBaud
$env:TEST_TIMEOUT = $testTimeout
$env:LOOPBACK_ENABLED = $loopbackEnabled

Write-Host ""
Write-Host "Test Configuration:" -ForegroundColor Cyan
Write-Host "  Port:     $env:TEST_PORT" -ForegroundColor White
Write-Host "  Baud:     $env:TEST_BAUD" -ForegroundColor White
Write-Host "  Timeout:  $env:TEST_TIMEOUT ms" -ForegroundColor White
Write-Host "  Loopback: $env:LOOPBACK_ENABLED" -ForegroundColor White
Write-Host ""

# Build test arguments
$testArgs = @(
    "test",
    "--release",
    "--features", "auto-negotiation",
    "--",
    "--ignored",
    "--test-threads=1"
)

if ($VerboseOutput) {
    $testArgs += "--nocapture"
}

# Run tests
Write-Host "Running hardware tests..." -ForegroundColor Cyan
Write-Host "Command: cargo $($testArgs -join ' ')" -ForegroundColor Gray
Write-Host ""

try {
    & cargo @testArgs
    $exitCode = $LASTEXITCODE
}
catch {
    Write-Error "Test execution failed: $_"
    exit 1
}

Write-Host ""
if ($exitCode -eq 0) {
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "  Hardware tests PASSED!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
}
else {
    Write-Host "========================================" -ForegroundColor Red
    Write-Host "  Hardware tests FAILED (exit code: $exitCode)" -ForegroundColor Red
    Write-Host "========================================" -ForegroundColor Red
}

exit $exitCode
