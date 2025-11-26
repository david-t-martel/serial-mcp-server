# Hardware Testing Guide

This directory contains comprehensive hardware tests for the Serial MCP Server that require real serial devices to run.

## Test Organization

### Test Modules

1. **utils.rs** - Test utilities and helpers
   - Port discovery functions
   - Test fixture management
   - Timing utilities
   - Environment configuration helpers

2. **real_port_tests.rs** - Basic hardware tests
   - Port open/close
   - Basic read/write
   - Timeout behavior
   - Buffer management
   - Baud rate switching
   - Concurrent access handling

3. **enhanced_real_port_tests.rs** - Advanced hardware tests
   - Write performance testing
   - Large data loopback tests
   - Configuration variations (data bits, parity, stop bits, flow control)
   - Timeout precision testing
   - Rapid open/close stress testing
   - Buffer overflow handling
   - Invalid configuration handling

4. **port_discovery_tests.rs** - Port enumeration tests
   - Port discovery and enumeration
   - USB port detection
   - Port metadata validation
   - Repeated discovery consistency

5. **auto_negotiation_hardware.rs** - Auto-negotiation tests (requires `auto-negotiation` feature)
   - Real-time auto-negotiation
   - Manufacturer profile detection
   - Baud rate detection
   - Strategy-specific testing
   - Multi-port detection

## Running Hardware Tests

### Prerequisites

1. **Serial Hardware**
   - At least one available serial port (USB-to-serial adapter, Arduino, etc.)
   - For loopback tests: TX-RX jumper wire or null modem adapter

2. **Check Available Ports**
   ```bash
   cargo run --example check_ports
   ```

### Environment Variables

```bash
# Windows
set TEST_PORT=COM15            # Required: Your serial port
set TEST_BAUD=9600             # Optional: Baud rate (default: 9600)
set TEST_LOOPBACK=1            # Optional: Enable loopback tests (requires hardware)

# Linux/macOS
export TEST_PORT=/dev/ttyUSB0  # Required: Your serial port
export TEST_BAUD=9600          # Optional: Baud rate (default: 9600)
export TEST_LOOPBACK=1         # Optional: Enable loopback tests (requires hardware)
```

### Running Tests

#### Run all hardware tests (basic features)
```bash
cargo test --tests --ignored
```

#### Run all hardware tests (with auto-negotiation)
```bash
cargo test --features auto-negotiation --ignored
```

#### Run specific test modules
```bash
# Port discovery tests
cargo test --test integration_hardware port_discovery --ignored

# Real port tests
cargo test --test integration_hardware real_port_tests --ignored

# Enhanced tests
cargo test --test integration_hardware enhanced_real_port_tests --ignored

# Auto-negotiation tests (requires feature)
cargo test --features auto-negotiation --test integration_hardware auto_negotiation_hardware --ignored
```

#### Run specific tests
```bash
# Basic open/close test
TEST_PORT=COM15 cargo test --ignored test_real_port_open_close -- --exact

# Loopback test (requires TX-RX connected)
TEST_PORT=COM15 TEST_LOOPBACK=1 cargo test --ignored test_real_port_loopback_communication -- --exact

# Manufacturer detection (requires USB device)
TEST_PORT=COM15 cargo test --features auto-negotiation --ignored test_real_manufacturer_profile_detection -- --exact

# Performance test
TEST_PORT=COM15 cargo test --ignored test_real_port_write_performance -- --exact
```

## Test Categories

### Tests That Don't Require Specific Hardware

These tests will work with any available serial port:

- `test_real_port_open_close` - Basic port access
- `test_real_port_timeout_behavior` - Timeout handling
- `test_real_port_buffer_sizes` - Buffer info queries
- `test_real_port_baud_rate_switching` - Configuration changes
- `test_real_port_multiple_open_close` - Stress testing
- `test_port_discovery` - Enumeration
- `test_usb_port_discovery` - USB detection
- All port discovery tests

### Tests Requiring Loopback Hardware

These tests need TX and RX physically connected:

- `test_real_port_loopback_communication` - Basic loopback
- `test_real_port_loopback_large_data` - Large data transfer
- `test_real_port_buffer_overflow_handling` - Buffer stress
- `test_real_port_clear_buffers_effectiveness` - Buffer management

### Tests Requiring USB Devices

These tests work best with USB-to-serial adapters:

- `test_real_port_manufacturer_detection` - VID/PID detection
- `test_real_manufacturer_profile_detection` - Profile matching
- `test_usb_vid_detection` - USB metadata
- `test_real_manufacturer_strategy_only` - Manufacturer-based negotiation

### Tests Requiring Auto-Negotiation Feature

These tests require building with `--features auto-negotiation`:

- `test_real_auto_negotiation_with_timing` - Full negotiation
- `test_real_manufacturer_profile_detection` - Profile-based detection
- `test_real_baud_rate_detection` - Baud detection
- `test_real_standard_bauds_strategy` - Brute-force detection
- `test_real_multi_port_detection` - Parallel detection
- `test_list_all_manufacturer_profiles` - Profile database

## Hardware Test Coverage

The hardware test suite provides comprehensive coverage of:

### Core Functionality (70%+ coverage goal)
- ✅ Port opening and closing
- ✅ Read/write operations
- ✅ Timeout handling
- ✅ Buffer management
- ✅ Configuration variations
- ✅ Error handling
- ✅ Concurrent access

### Advanced Features
- ✅ Auto-negotiation strategies
- ✅ Manufacturer profile detection
- ✅ USB device enumeration
- ✅ Performance testing
- ✅ Stress testing
- ✅ Edge cases and error paths

### Platform-Specific
- ✅ Windows COM port handling
- ✅ USB device metadata (VID/PID)
- ✅ Port type detection

## Test Results Interpretation

### Success Indicators
- ✅ Checkmarks in output
- Tests return without panicking
- Performance metrics within expected ranges

### Expected Failures
Some tests may fail or skip gracefully:
- Auto-negotiation can fail for non-responsive devices (this is OK)
- Concurrent access tests may allow multiple opens on some platforms
- Some flow control modes may not be supported by all devices
- Manufacturer profiles may not exist for unknown VID/PID

### Actual Failures
These indicate real problems:
- Port fails to open when it should be available
- Read/write operations fail on known-good hardware
- Timeouts don't respect configured values
- Buffer operations don't work correctly

## Continuous Integration

Hardware tests are **ignored by default** and don't run in CI because they require physical devices.

To run hardware tests in CI:
1. Set up a test machine with serial hardware
2. Configure environment variables in CI config
3. Use `cargo test --features auto-negotiation --ignored`

## Adding New Hardware Tests

When adding new hardware tests:

1. **Mark as ignored**: Use `#[ignore]` attribute
2. **Use utilities**: Import from `crate::hardware::utils`
3. **Check environment**: Use `TestPortConfig::from_env()`
4. **Skip gracefully**: Return early if hardware not available
5. **Print progress**: Use `println!` for user feedback
6. **Document requirements**: Specify what hardware is needed

Example:
```rust
#[test]
#[ignore] // Requires hardware
fn test_new_feature() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    // Test implementation...
    println!("✅ Test passed");
}
```

## Known Manufacturer Profiles

The auto-negotiation system includes profiles for:

- **FTDI** (VID: 0x0403) - Default: 115200 baud
- **Silicon Labs CP210x** (VID: 0x10C4) - Default: 9600 baud
- **WCH CH340/CH341** (VID: 0x1A86) - Default: 9600 baud
- **Arduino** (VID: 0x2341) - Default: 9600 baud
- **Adafruit** (VID: 0x239A) - Default: 115200 baud
- **Raspberry Pi Pico** (VID: 0x2E8A) - Default: 115200 baud
- **Prolific PL2303** (VID: 0x067B) - Default: 9600 baud
- **STMicroelectronics** (VID: 0x0483) - Default: 115200 baud

See `src/negotiation/strategies/manufacturer.rs` for complete list.

## Troubleshooting

### Port Not Found
- Check device is connected: `cargo run --example check_ports`
- Verify drivers are installed (Windows: Device Manager)
- Check permissions (Linux: user in `dialout` group)

### Port Access Denied
- Close other applications using the port
- On Linux: `sudo chmod 666 /dev/ttyUSB0` (temporary)
- Windows: Check if COM port is already open in another program

### Tests Timeout
- Increase `TEST_BAUD` if device requires specific baud rate
- Some devices need time to initialize after opening
- Check physical connections for loopback tests

### Loopback Tests Fail
- Verify TX and RX are connected
- Check for proper ground connection
- Try with null modem adapter if direct connection fails
- Some USB adapters have internal loopback - test with known device

## Test Statistics

Current hardware test coverage:

- **Total hardware tests**: 35+
- **Basic functionality**: 9 tests
- **Enhanced functionality**: 12 tests
- **Port discovery**: 10 tests
- **Auto-negotiation**: 7 tests
- **Utility tests**: 5 tests

Target: 70%+ code coverage with hardware tests enabled.
