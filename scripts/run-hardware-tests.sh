#!/usr/bin/env bash
#
# run-hardware-tests.sh - Run hardware tests for rust-comm
#
# This script reads test configuration from config.toml and runs hardware tests
# against discovered or configured serial ports.
#
# Usage:
#   ./scripts/run-hardware-tests.sh [config_path]
#   AUTO_DISCOVER=true ./scripts/run-hardware-tests.sh
#   VERBOSE=true ./scripts/run-hardware-tests.sh
#   TEST_PORT=COM15 TEST_BAUD=9600 ./scripts/run-hardware-tests.sh

set -euo pipefail

# Configuration
CONFIG_PATH="${1:-./config.toml}"
AUTO_DISCOVER="${AUTO_DISCOVER:-false}"
VERBOSE="${VERBOSE:-false}"

echo "========================================"
echo "  rust-comm Hardware Test Runner"
echo "========================================"
echo ""

# Helper function to parse TOML values (basic implementation)
parse_toml() {
    local file="$1"
    local section="$2"
    local key="$3"

    if [[ ! -f "$file" ]]; then
        echo ""
        return
    fi

    # Simple TOML parser using awk
    awk -v section="$section" -v key="$key" '
        BEGIN { in_section = 0 }
        /^\[/ {
            gsub(/[\[\]]/, "")
            in_section = ($0 == section) ? 1 : 0
        }
        in_section && $1 == key {
            gsub(/^[^=]+=[ \t]*/, "")
            gsub(/^"/, "")
            gsub(/"$/, "")
            gsub(/[ \t]*#.*$/, "")
            print
            exit
        }
    ' "$file"
}

# Load configuration
if [[ -f "$CONFIG_PATH" ]]; then
    echo "Loading config from: $CONFIG_PATH"
    CONFIG_PORT=$(parse_toml "$CONFIG_PATH" "testing" "port")
    CONFIG_BAUD=$(parse_toml "$CONFIG_PATH" "testing" "baud")
    CONFIG_TIMEOUT=$(parse_toml "$CONFIG_PATH" "testing" "timeout_ms")
    CONFIG_LOOPBACK=$(parse_toml "$CONFIG_PATH" "testing" "loopback_enabled")
    CONFIG_DISCOVERY=$(parse_toml "$CONFIG_PATH" "testing.discovery" "enabled")
else
    echo "Config file not found, using defaults and auto-discovery"
    CONFIG_PORT=""
    CONFIG_BAUD=""
    CONFIG_TIMEOUT=""
    CONFIG_LOOPBACK=""
    CONFIG_DISCOVERY="true"
fi

# Environment variable overrides
TEST_PORT="${TEST_PORT:-$CONFIG_PORT}"
TEST_BAUD="${TEST_BAUD:-${CONFIG_BAUD:-115200}}"
TEST_TIMEOUT="${TEST_TIMEOUT:-${CONFIG_TIMEOUT:-2000}}"
LOOPBACK_ENABLED="${LOOPBACK_ENABLED:-${CONFIG_LOOPBACK:-false}}"

# Auto-discover if needed
if [[ -z "$TEST_PORT" ]] || [[ "$AUTO_DISCOVER" == "true" ]] || [[ "$CONFIG_DISCOVERY" == "true" ]]; then
    echo "Auto-discovering test ports..."

    # Platform-specific port discovery
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        PORTS=$(ls /dev/tty.usbserial* /dev/tty.usbmodem* /dev/tty.SLAB* 2>/dev/null || true)
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux
        PORTS=$(ls /dev/ttyUSB* /dev/ttyACM* /dev/ttyS* 2>/dev/null | grep -v "ttyS[0-3]$" || true)
    elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
        # Windows (Git Bash, Cygwin)
        # Use PowerShell to list ports
        PORTS=$(powershell -Command "[System.IO.Ports.SerialPort]::GetPortNames()" 2>/dev/null | tr -d '\r' || true)
    else
        echo "Warning: Unknown OS type, manual port specification required"
        PORTS=""
    fi

    if [[ -n "$PORTS" ]]; then
        echo "Available ports:"
        echo "$PORTS" | while read -r port; do
            echo "  - $port"
        done

        # Select first port (filter out common system ports)
        TEST_PORT=$(echo "$PORTS" | grep -v -E "^(/dev/ttyS[0-3]|COM[12])$" | head -1 || echo "$PORTS" | head -1)

        if [[ -z "$TEST_PORT" ]]; then
            echo "Error: No serial ports discovered"
            exit 1
        fi

        echo "Selected port: $TEST_PORT"
    else
        echo "Error: No serial ports discovered"
        exit 1
    fi
fi

if [[ -z "$TEST_PORT" ]]; then
    echo "Error: No test port available"
    echo "Set TEST_PORT environment variable or configure in config.toml"
    exit 1
fi

# Export environment variables for tests
export TEST_PORT
export TEST_BAUD
export TEST_TIMEOUT
export LOOPBACK_ENABLED

echo ""
echo "Test Configuration:"
echo "  Port:     $TEST_PORT"
echo "  Baud:     $TEST_BAUD"
echo "  Timeout:  $TEST_TIMEOUT ms"
echo "  Loopback: $LOOPBACK_ENABLED"
echo ""

# Build test arguments
TEST_ARGS="--release --features auto-negotiation -- --ignored --test-threads=1"

if [[ "$VERBOSE" == "true" ]]; then
    TEST_ARGS="$TEST_ARGS --nocapture"
fi

# Run tests
echo "Running hardware tests..."
echo "Command: cargo test $TEST_ARGS"
echo ""

set +e
cargo test $TEST_ARGS
EXIT_CODE=$?
set -e

echo ""
if [[ $EXIT_CODE -eq 0 ]]; then
    echo "========================================"
    echo "  Hardware tests PASSED!"
    echo "========================================"
else
    echo "========================================"
    echo "  Hardware tests FAILED (exit code: $EXIT_CODE)"
    echo "========================================"
fi

exit $EXIT_CODE
