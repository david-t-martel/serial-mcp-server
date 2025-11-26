# Serial MCP Server Test Suite

This directory contains comprehensive tests for the Serial MCP Server, organized into multiple test categories for different use cases.

## Test Organization

```
tests/
├── common/              # Shared test utilities and helpers
│   └── mod.rs          # Mock port creation, JSON assertions, test harness
├── e2e/                # End-to-end tests (no hardware required)
│   ├── mod.rs
│   ├── discovery_tests.rs     # Port discovery and enumeration
│   ├── negotiation_tests.rs   # Auto-negotiation strategies
│   └── workflow_tests.rs      # Complete workflows
├── hardware/           # Hardware-specific tests (real devices required)
│   ├── mod.rs
│   └── real_port_tests.rs    # Tests with actual serial ports
├── e2e.rs             # E2E test suite entry point
├── hardware.rs        # Hardware test suite entry point
└── README.md          # This file
```

## Test Categories

### 1. E2E Tests (No Hardware Required)

These tests run against the system using mock serial ports and don't require real hardware.

**Running E2E tests:**
```bash
# Run all E2E tests
cargo test --all-features

# Run specific test module
cargo test --all-features discovery_tests
cargo test --all-features negotiation_tests
cargo test --all-features workflow_tests

# Run with verbose output
cargo test --all-features -- --nocapture
```

**E2E test coverage:**
- ✅ Port discovery and enumeration
- ✅ USB metadata extraction (VID, PID, manufacturer)
- ✅ Auto-negotiation with manufacturer profiles
- ✅ Strategy priority ordering
- ✅ Complete workflows (open → write → read → close)
- ✅ Session management and persistence
- ✅ Message filtering and export
- ✅ Idle disconnect behavior
- ✅ Buffer management
- ✅ Timeout handling

### 2. Hardware Tests (Real Devices Required)

These tests require actual serial hardware and are **ignored by default**. They must be explicitly enabled and require environment configuration.

**Environment Variables:**

| Variable | Description | Example | Required |
|----------|-------------|---------|----------|
| `TEST_PORT` | Serial port to use | `COM3` (Windows)<br>`/dev/ttyUSB0` (Linux)<br>`/dev/cu.usbserial-*` (macOS) | **Yes** |
| `TEST_BAUD` | Baud rate for testing | `9600`, `115200` | No (default: 9600) |
| `TEST_LOOPBACK` | Enable loopback tests | `1` | No (default: disabled) |

**Running hardware tests:**

```bash
# Windows (CMD)
set TEST_PORT=COM3
set TEST_BAUD=9600
cargo test --all-features -- --ignored

# Windows (PowerShell)
$env:TEST_PORT="COM3"
$env:TEST_BAUD="9600"
cargo test --all-features -- --ignored

# Linux/macOS
export TEST_PORT=/dev/ttyUSB0
export TEST_BAUD=9600
cargo test --all-features -- --ignored

# With loopback enabled (TX-RX connected)
export TEST_LOOPBACK=1
cargo test --all-features -- --ignored
```

**Hardware test coverage:**
- ✅ Real port open/close operations
- ✅ Auto-negotiation with real devices
- ✅ Loopback communication (requires TX-RX connection)
- ✅ Manufacturer detection and VID/PID matching
- ✅ Buffer size queries
- ✅ Timeout behavior verification
- ✅ Multiple open/close cycles
- ✅ Baud rate switching
- ✅ Concurrent access handling

### 3. Integration Tests

Integration tests for specific features:

```bash
# Test negotiation module
cargo test --features auto-negotiation test_negotiation

# Test async serial support
cargo test --features async-serial

# Test with all features
cargo test --all-features
```

### 4. Unit Tests

Module-level unit tests are embedded in source files:

```bash
# Run all unit tests
cargo test --lib

# Run specific module tests
cargo test --lib port::mock
cargo test --lib session
cargo test --lib negotiation
```

## Test Utilities (`tests/common/`)

The `common` module provides shared testing infrastructure:

### `TestHarness`

Complete test environment with state and session management:

```rust
use crate::common::TestHarness;

#[tokio::test]
async fn my_test() {
    let harness = TestHarness::new().await;
    assert!(!harness.is_port_open());
}
```

### Mock Port Creation

```rust
use crate::common::{create_mock_port_with_responses, create_manufacturer_mock_port};

// Create mock with pre-programmed responses
let mock = create_mock_port_with_responses("MOCK0", vec![
    b"OK\r\n",
    b"READY\r\n",
]);

// Create mock simulating manufacturer ID
let mock = create_manufacturer_mock_port("MOCK0", "FTDI FT232R");
```

### JSON Assertions

```rust
use crate::common::assert_json_contains;
use serde_json::json;

let actual = json!({"status": "ok", "port": "COM1", "extra": "data"});
let expected = json!({"status": "ok", "port": "COM1"});

assert_json_contains(&actual, &expected); // Passes
```

### Port Configuration Builder

```rust
use crate::common::PortConfigBuilder;

let config = PortConfigBuilder::new("COM1")
    .baud_rate(115200)
    .timeout_ms(500)
    .build();
```

## Hardware Test Setup

### Loopback Testing

For comprehensive hardware testing, connect TX and RX pins on your serial port:

**USB-to-Serial Adapter Loopback:**
```
┌─────────────────┐
│  USB Serial     │
│  ┌───┐          │
│  │TX ├──┐       │
│  └───┘  │       │
│         │ wire  │
│  ┌───┐  │       │
│  │RX ├──┘       │
│  └───┘          │
│  GND            │
└─────────────────┘
```

**DB9 Connector Loopback:**
- Connect pins 2 (TX) and 3 (RX)
- Connect pin 5 (GND) to ground

### Supported Devices

Hardware tests are designed for:
- **FTDI** devices (VID: 0x0403)
- **Arduino** boards (VID: 0x2341)
- **Silicon Labs CP210x** (VID: 0x10c4)
- **Prolific PL2303** (VID: 0x067b)
- **CH340/CH341** (VID: 0x1a86)
- **Raspberry Pi Pico** (VID: 0x2e8a)
- Generic serial ports

## Continuous Integration

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      # E2E tests (no hardware)
      - name: Run E2E tests
        run: cargo test --all-features

      # Hardware tests are skipped in CI
      # They require environment setup and real devices
```

### Local Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "Running E2E tests..."
cargo test --all-features

if [ $? -ne 0 ]; then
    echo "Tests failed. Commit aborted."
    exit 1
fi
```

## Test Coverage

Generate coverage reports:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --all-features --out Html

# Open coverage report
# Results in tarpaulin-report.html
```

## Debugging Tests

### Verbose Output

```bash
# Show println! output
cargo test --all-features -- --nocapture

# Show test names as they run
cargo test --all-features -- --test-threads=1 --nocapture
```

### Run Specific Test

```bash
# Run single test
cargo test --all-features test_list_ports_returns_system_ports

# Run tests matching pattern
cargo test --all-features discovery

# Run with backtrace
RUST_BACKTRACE=1 cargo test --all-features
```

### Test Filters

```bash
# Run only E2E tests
cargo test --all-features --test e2e

# Run only hardware tests
cargo test --all-features --test hardware -- --ignored

# Run all tests including ignored
cargo test --all-features -- --include-ignored
```

## Writing New Tests

### E2E Test Template

```rust
#[tokio::test]
async fn test_my_feature() {
    // Arrange
    let harness = TestHarness::new().await;
    let mock = create_mock_port_with_responses("MOCK0", vec![b"OK\r\n"]);

    // Act
    // ... perform operations

    // Assert
    assert!(harness.is_port_open());
}
```

### Hardware Test Template

```rust
#[test]
#[ignore]
fn test_my_hardware_feature() {
    let port_name = match skip_without_hardware() {
        Some(p) => p,
        None => return,
    };

    // Test with real hardware
    let config = PortConfiguration::new().baud_rate(9600);
    let port = SyncSerialPort::open(&port_name, config).unwrap();

    // ... test operations
}
```

## Test Best Practices

### Do's ✅

- **Use descriptive test names**: `test_negotiation_with_ftdi_device`
- **Test one thing per test**: Single responsibility
- **Use test fixtures**: Leverage `TestHarness` and helpers
- **Clean up resources**: Tests should be idempotent
- **Mock external dependencies**: Use `MockSerialPort` for E2E
- **Document test requirements**: Especially for hardware tests

### Don'ts ❌

- **Don't share state between tests**: Each test should be independent
- **Don't commit hardware tests without `#[ignore]`**: Would break CI
- **Don't use real devices in E2E tests**: Use mocks
- **Don't use hardcoded ports**: Use environment variables
- **Don't skip error handling**: Tests should verify error cases

## Troubleshooting

### Common Issues

**Issue:** `TEST_PORT not set` message
- **Solution:** Set the `TEST_PORT` environment variable before running hardware tests

**Issue:** Port already in use
- **Solution:** Close other applications using the port, or use a different port

**Issue:** Permission denied on Linux
- **Solution:** Add user to `dialout` group: `sudo usermod -a -G dialout $USER`

**Issue:** Tests hang during hardware tests
- **Solution:** Check timeout settings, ensure device is responding

**Issue:** Loopback test fails
- **Solution:** Verify TX-RX pins are properly connected

## Performance Benchmarks

Run performance benchmarks:

```bash
cargo bench

# Results in target/criterion/
```

Benchmark coverage:
- Port discovery speed
- Negotiation strategy performance
- Read/write throughput
- Session database operations

## Contributing

When adding new tests:

1. **Place tests in the appropriate category** (E2E vs hardware)
2. **Add test utilities to `common/`** if reusable
3. **Update this README** with new test descriptions
4. **Follow test naming conventions**: `test_<feature>_<scenario>`
5. **Add documentation comments** explaining test purpose
6. **Ensure tests pass** before submitting PR

## License

Tests are licensed under the same terms as the main project (MIT OR Apache-2.0).
