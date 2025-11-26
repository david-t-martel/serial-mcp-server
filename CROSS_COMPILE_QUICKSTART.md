# Cross-Compilation Quick Start

## Quick Commands

### Install cross tool (one-time setup)
```bash
cargo install cross --git https://github.com/cross-rs/cross
```

### Build for ARM64 Linux

**Linux/macOS/WSL:**
```bash
chmod +x build-arm64.sh
./build-arm64.sh release    # Release build
./build-arm64.sh debug      # Debug build
./build-arm64.sh small      # Size-optimized build
```

**Windows PowerShell:**
```powershell
.\build-arm64.ps1 -BuildType release    # Release build
.\build-arm64.ps1 -BuildType debug      # Debug build
.\build-arm64.ps1 -BuildType small      # Size-optimized build
```

**Manual build:**
```bash
cross build --target aarch64-unknown-linux-gnu --release
```

## Output Location
```
target/aarch64-unknown-linux-gnu/release/serial_mcp_agent
```

## Deploy to Raspberry Pi
```bash
# Copy binary
scp target/aarch64-unknown-linux-gnu/release/serial_mcp_agent pi@raspberrypi.local:~/

# SSH and run
ssh pi@raspberrypi.local
./serial_mcp_agent --version
```

## Deploy to AWS Graviton
```bash
# Copy binary
scp target/aarch64-unknown-linux-gnu/release/serial_mcp_agent ec2-user@<instance-ip>:~/

# SSH and run
ssh ec2-user@<instance-ip>
./serial_mcp_agent --version
```

## Verify Binary
```bash
file target/aarch64-unknown-linux-gnu/release/serial_mcp_agent
# Expected: ELF 64-bit LSB pie executable, ARM aarch64
```

## Files Created

- `Cross.toml` - Cross-compilation configuration
- `BUILD_CROSS_COMPILE.md` - Comprehensive documentation
- `build-arm64.sh` - Linux/macOS build script
- `build-arm64.ps1` - Windows PowerShell build script
- `Cargo.toml` - Updated with platform-specific dependencies

## Platform-Specific Dependencies

Added to `Cargo.toml`:
```toml
[target.'cfg(unix)'.dependencies]
libc = "0.2"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winuser", "winerror"] }
```

These ensure proper serial port access on each platform.

## Troubleshooting

### "cross: command not found"
```bash
cargo install cross --git https://github.com/cross-rs/cross
```

### "Cannot connect to Docker daemon"
Start Docker Desktop and ensure it's running.

### Permission denied on target device
```bash
# On ARM64 device
sudo usermod -aG dialout $USER
# Log out and back in
```

## See Also

- Full documentation: `BUILD_CROSS_COMPILE.md`
- Cross tool: https://github.com/cross-rs/cross
- Rust platform support: https://doc.rust-lang.org/nightly/rustc/platform-support.html
