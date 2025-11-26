# Hardware Tests - Quick Start Guide

Get started with hardware testing in under 5 minutes!

## Step 1: Check Your Hardware

```bash
cargo run --example check_ports
```

This will show all available serial ports on your system:

```
✅ Found 4 serial port(s):

1. COM15
   Type:         USB Serial Port
   VID:          0x0403
   PID:          0x6001
   Manufacturer: FTDI
   Product:      USB Serial Port
```

Copy the port name (e.g., `COM15` on Windows or `/dev/ttyUSB0` on Linux).

## Step 2: Set Environment Variable

### Windows (Command Prompt)
```cmd
set TEST_PORT=COM15
```

### Windows (PowerShell)
```powershell
$env:TEST_PORT="COM15"
```

### Linux/macOS (Bash)
```bash
export TEST_PORT=/dev/ttyUSB0
```

## Step 3: Run Hardware Tests

### Run All Tests
```bash
cargo test --features auto-negotiation --test integration_hardware -- --ignored
```

### Run Specific Test
```bash
# Test port opening
cargo test --test integration_hardware test_real_port_open_close -- --ignored --exact

# Test auto-negotiation
cargo test --features auto-negotiation --test integration_hardware test_real_auto_negotiation_with_timing -- --ignored --exact

# Test port discovery
cargo test --test integration_hardware test_port_discovery -- --ignored --exact
```

## Expected Output

```
running 1 test
test hardware::real_port_tests::test_real_port_open_close ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 34 filtered out
```

## Optional: Loopback Tests

If you have a loopback adapter (TX connected to RX):

### Windows
```cmd
set TEST_LOOPBACK=1
```

### Linux/macOS
```bash
export TEST_LOOPBACK=1
```

Then run loopback tests:
```bash
cargo test --test integration_hardware loopback -- --ignored
```

## What If I Don't Have Hardware?

No problem! The tests will skip gracefully:

```bash
cargo test --test integration_hardware test_real_port_open_close -- --ignored
```

Output:
```
⏭️  Skipping: TEST_PORT environment variable not set
   Set TEST_PORT=COM3 (or /dev/ttyUSB0) to run hardware tests
```

## Common Issues

### "Port not found"
- Check device is connected: `cargo run --example check_ports`
- Verify the port name matches exactly (case-sensitive on Linux)

### "Permission denied" (Linux)
```bash
# Add your user to dialout group
sudo usermod -a -G dialout $USER
# Log out and back in

# Or temporarily (not recommended)
sudo chmod 666 /dev/ttyUSB0
```

### "Port already in use"
- Close other applications (Arduino IDE, serial terminal, etc.)
- Check with: `lsof | grep /dev/ttyUSB0` (Linux)

## Test Categories

### No Special Hardware Needed
Run with any serial port:
```bash
cargo test --test integration_hardware test_port_discovery -- --ignored
cargo test --test integration_hardware test_real_port_open_close -- --ignored
```

### Requires Loopback (TX-RX connected)
```bash
TEST_LOOPBACK=1 cargo test --test integration_hardware loopback -- --ignored
```

### Requires USB Device
```bash
cargo test --test integration_hardware test_usb_vid_detection -- --ignored
```

## Full Test List

View all 35 hardware tests:
```bash
cargo test --features auto-negotiation --test integration_hardware -- --ignored --list
```

## For More Information

See [README.md](README.md) for:
- Complete test documentation
- Troubleshooting guide
- Advanced usage
- CI/CD integration

## Quick Reference Card

```bash
# 1. Check hardware
cargo run --example check_ports

# 2. Set port (Windows)
set TEST_PORT=COM15

# 3. Run tests
cargo test --features auto-negotiation --test integration_hardware -- --ignored

# 4. Run single test
cargo test --test integration_hardware test_real_port_open_close -- --ignored --exact

# 5. List all tests
cargo test --test integration_hardware -- --ignored --list
```

That's it! You're ready to run hardware tests. 🎉
