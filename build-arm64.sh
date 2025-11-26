#!/usr/bin/env bash
# Build script for ARM64 Linux cross-compilation
# Usage: ./build-arm64.sh [release|debug|small]

set -euo pipefail

TARGET="aarch64-unknown-linux-gnu"
BUILD_TYPE="${1:-release}"

echo "Building rust-comm for ARM64 Linux..."
echo "Target: $TARGET"
echo "Build type: $BUILD_TYPE"
echo ""

# Check if cross is installed
if ! command -v cross &> /dev/null; then
    echo "Error: 'cross' is not installed."
    echo "Install with: cargo install cross --git https://github.com/cross-rs/cross"
    exit 1
fi

# Check if Docker is running
if ! docker ps &> /dev/null; then
    echo "Error: Docker is not running."
    echo "Please start Docker and try again."
    exit 1
fi

# Build based on type
case "$BUILD_TYPE" in
    release)
        echo "Building release binary..."
        cross build --target "$TARGET" --release
        BINARY_PATH="target/$TARGET/release/serial_mcp_agent"
        ;;
    debug)
        echo "Building debug binary..."
        cross build --target "$TARGET"
        BINARY_PATH="target/$TARGET/debug/serial_mcp_agent"
        ;;
    small)
        echo "Building size-optimized binary..."
        cross build --target "$TARGET" --profile release-small
        BINARY_PATH="target/$TARGET/release-small/serial_mcp_agent"
        ;;
    *)
        echo "Error: Unknown build type '$BUILD_TYPE'"
        echo "Usage: $0 [release|debug|small]"
        exit 1
        ;;
esac

echo ""
echo "Build complete!"
echo "Binary location: $BINARY_PATH"
echo ""

# Show binary info
if [ -f "$BINARY_PATH" ]; then
    echo "Binary size: $(du -h "$BINARY_PATH" | cut -f1)"
    echo ""
    echo "Verify architecture with: file $BINARY_PATH"
    echo "Expected: ELF 64-bit LSB pie executable, ARM aarch64"
else
    echo "Warning: Binary not found at expected location"
    exit 1
fi

echo ""
echo "Next steps:"
echo "  1. Transfer to ARM64 device: scp $BINARY_PATH user@device:~/"
echo "  2. Run on device: ./serial_mcp_agent --help"
echo "  3. Verify: file serial_mcp_agent"
