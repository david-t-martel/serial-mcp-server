# Cross-Compilation Guide for rust-comm

This project supports cross-compilation to ARM64 Linux targets using the `cross` tool.

## Prerequisites

1. **Install cross**:
   ```bash
   cargo install cross --git https://github.com/cross-rs/cross
   ```

2. **Ensure Docker is running** (cross uses Docker containers for cross-compilation):
   ```bash
   docker --version
   docker ps
   ```

## Supported Targets

### ARM64 Linux (aarch64-unknown-linux-gnu)
- **Use cases**: Raspberry Pi 3/4/5, AWS Graviton, ARM-based servers
- **Image**: `ghcr.io/cross-rs/aarch64-unknown-linux-gnu:0.2.5`
- **Includes**: GCC 11, libudev-dev, pkg-config

## Build Commands

### ARM64 Linux Release Build
```bash
cross build --target aarch64-unknown-linux-gnu --release
```

### ARM64 Linux Debug Build
```bash
cross build --target aarch64-unknown-linux-gnu
```

### Build with specific features
```bash
# MCP + REST API (default)
cross build --target aarch64-unknown-linux-gnu --release

# MCP only
cross build --target aarch64-unknown-linux-gnu --release --no-default-features --features mcp

# With TUI
cross build --target aarch64-unknown-linux-gnu --release --features tui

# All features
cross build --target aarch64-unknown-linux-gnu --release --all-features
```

### Small binary build
```bash
cross build --target aarch64-unknown-linux-gnu --profile release-small
```

## Output Locations

Built binaries will be in:
```
target/aarch64-unknown-linux-gnu/release/serial_mcp_agent
target/aarch64-unknown-linux-gnu/release/serial-tui
```

## Deployment

### Copy to ARM64 device
```bash
# Example: Raspberry Pi
scp target/aarch64-unknown-linux-gnu/release/serial_mcp_agent pi@raspberrypi.local:~/

# Example: AWS Graviton
scp target/aarch64-unknown-linux-gnu/release/serial_mcp_agent ec2-user@<graviton-ip>:~/
```

### Verify on target device
```bash
# On the ARM64 device
./serial_mcp_agent --version
./serial_mcp_agent --help

# Check binary architecture
file ./serial_mcp_agent
# Expected output: ELF 64-bit LSB pie executable, ARM aarch64, version 1 (SYSV), ...
```

## Platform-Specific Dependencies

The project includes platform-specific dependencies configured in `Cargo.toml`:

### Unix/Linux (including ARM64)
- `libc`: Low-level C library bindings
- Serial port access via `serialport` crate (uses `libudev`)

### Windows
- `winapi`: Windows API bindings for serial port access

These are automatically selected based on the target platform during compilation.

## Troubleshooting

### Docker permission issues
```bash
# Linux: Add user to docker group
sudo usermod -aG docker $USER
# Then log out and back in
```

### Cross build fails with "image not found"
```bash
# Pull the image manually
docker pull ghcr.io/cross-rs/aarch64-unknown-linux-gnu:0.2.5
```

### Serial port permissions on target device
```bash
# Add user to dialout group (Linux/Raspberry Pi)
sudo usermod -aG dialout $USER
# Log out and back in for changes to take effect
```

### Custom Docker image needed
If you need additional dependencies, uncomment the custom Dockerfile section in `Cross.toml` and create:

```dockerfile
# docker/aarch64.Dockerfile
FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:0.2.5

RUN apt-get update && apt-get install -y \
    libudev-dev \
    pkg-config \
    # Add other dependencies here
    && rm -rf /var/lib/apt/lists/*
```

Then update `Cross.toml`:
```toml
[target.aarch64-unknown-linux-gnu]
dockerfile = "./docker/aarch64.Dockerfile"
```

## Testing Cross-Compiled Binaries

### Using QEMU (on development machine)
```bash
# Install QEMU
sudo apt-get install qemu-user-static

# Run ARM64 binary
qemu-aarch64-static target/aarch64-unknown-linux-gnu/release/serial_mcp_agent --help
```

### On actual hardware
Transfer the binary to an ARM64 device and run integration tests:
```bash
# On ARM64 device
./serial_mcp_agent --help
./serial_mcp_agent stdio < test_input.json
```

## Performance Considerations

### Binary Size
- **Default release**: ~15-20MB (with all features)
- **release-small profile**: ~8-12MB (optimized for size)
- **Stripped**: Symbols removed for smaller deployment

### Build Time
- First build: 5-15 minutes (depends on Docker image pull + compilation)
- Incremental builds: 1-3 minutes
- Use `sccache` for faster rebuilds:
  ```bash
  cargo install sccache
  export RUSTC_WRAPPER=sccache
  cross build --target aarch64-unknown-linux-gnu --release
  ```

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Cross-Compile ARM64

on: [push, pull_request]

jobs:
  build-arm64:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install cross
        run: cargo install cross --git https://github.com/cross-rs/cross
      - name: Build ARM64
        run: cross build --target aarch64-unknown-linux-gnu --release
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: serial-mcp-agent-arm64
          path: target/aarch64-unknown-linux-gnu/release/serial_mcp_agent
```

## Additional Resources

- [cross documentation](https://github.com/cross-rs/cross)
- [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [serialport crate](https://docs.rs/serialport/)
