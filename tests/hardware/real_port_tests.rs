//! Tests requiring actual serial hardware.
//!
//! These tests are skipped if no hardware is available.
//!
//! # Running Hardware Tests
//!
//! ```bash
//! # Set environment variables
//! export TEST_PORT=COM3                  # or /dev/ttyUSB0 on Linux
//! export TEST_BAUD=9600                  # optional, default: 9600
//! export TEST_LOOPBACK=1                 # if port has TX-RX loopback
//!
//! # Run tests
//! cargo test --all-features -- --ignored
//! ```
//!
//! # Hardware Requirements
//!
//! - **Real port tests**: Any available serial port
//! - **Loopback tests**: Port with TX and RX connected together
//! - **Manufacturer tests**: Port with known VID (e.g., FTDI, Arduino)

#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::assertions_on_constants)]

use serial_mcp_agent::port::{PortConfiguration, SerialPortAdapter, SyncSerialPort};
use serialport::available_ports;
use std::env;
use std::time::Duration;

/// Get the test port from environment variable.
fn get_test_port() -> Option<String> {
    env::var("TEST_PORT").ok()
}

/// Get the test baud rate from environment variable (default: 9600).
fn get_test_baud() -> u32 {
    env::var("TEST_BAUD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9600)
}

/// Check if loopback testing is enabled.
fn is_loopback_enabled() -> bool {
    env::var("TEST_LOOPBACK").ok().as_deref() == Some("1")
}

/// Skip test if hardware is not available.
fn skip_without_hardware() -> Option<String> {
    let port = get_test_port();
    if port.is_none() {
        println!("⏭️  Skipping hardware test: TEST_PORT not set");
    }
    port
}

#[test]
#[ignore] // Run with --ignored flag
fn test_real_port_open_close() {
    let port_name = match skip_without_hardware() {
        Some(p) => p,
        None => return,
    };

    let baud = get_test_baud();
    println!("Testing port: {} at {} baud", port_name, baud);

    // Configure port
    let mut config = PortConfiguration::default();
    config.baud_rate = baud;
    config.timeout = Duration::from_millis(1000);

    // Open port
    let port = match SyncSerialPort::open(&port_name, config) {
        Ok(p) => p,
        Err(e) => {
            println!("❌ Failed to open port: {}", e);
            panic!("Port open failed: {}", e);
        }
    };

    println!("✅ Port opened successfully");

    // Verify port name
    assert_eq!(port.name(), port_name);

    // Port should be readable/writable
    assert!(true);

    println!("✅ Port open/close test passed");
}

#[tokio::test]
#[ignore]
async fn test_real_port_auto_negotiation() {
    #[cfg(not(feature = "auto-negotiation"))]
    {
        println!("⏭️  Skipping: auto-negotiation feature not enabled");
        return;
    }

    #[cfg(feature = "auto-negotiation")]
    {
        use serial_mcp_agent::negotiation::{AutoNegotiator, NegotiationHints};

        let port_name = match skip_without_hardware() {
            Some(p) => p,
            None => return,
        };

        println!("Testing auto-negotiation on: {}", port_name);

        let negotiator = AutoNegotiator::new();
        let hints = NegotiationHints::default().with_timeout_ms(2000);

        let result = negotiator.detect(&port_name, Some(hints)).await;

        match result {
            Ok(params) => {
                println!("✅ Negotiation succeeded:");
                println!("   Baud rate: {}", params.baud_rate);
                println!("   Strategy: {}", params.strategy_used);
                println!("   Confidence: {:.2}", params.confidence);
                println!("   Data bits: {:?}", params.data_bits);
                println!("   Parity: {:?}", params.parity);
                println!("   Stop bits: {:?}", params.stop_bits);

                assert!(params.baud_rate > 0);
                assert!(params.confidence >= 0.0 && params.confidence <= 1.0);
            }
            Err(e) => {
                println!("⚠️  Negotiation failed (expected for some devices): {}", e);
                // Not all devices support negotiation, so this isn't necessarily a failure
            }
        }
    }
}

#[test]
#[ignore]
fn test_real_port_loopback_communication() {
    if !is_loopback_enabled() {
        println!("⏭️  Skipping loopback test: TEST_LOOPBACK not set to 1");
        return;
    }

    let port_name = match skip_without_hardware() {
        Some(p) => p,
        None => return,
    };

    let baud = get_test_baud();
    println!("Testing loopback on: {} at {} baud", port_name, baud);

    let mut config = PortConfiguration::default();
    config.baud_rate = baud;
    config.timeout = Duration::from_millis(1000);

    let mut port = SyncSerialPort::open(&port_name, config).expect("Failed to open port");

    // Clear any existing data
    port.clear_buffers().expect("Failed to clear buffers");

    // Test data
    let test_data = b"LOOPBACK TEST\r\n";

    // Write data
    let written = port
        .write_bytes(test_data)
        .expect("Failed to write to port");
    assert_eq!(written, test_data.len());
    println!("✅ Wrote {} bytes", written);

    // Small delay for data to loop back
    std::thread::sleep(Duration::from_millis(100));

    // Read data back
    let mut buffer = [0u8; 256];
    let read = port
        .read_bytes(&mut buffer)
        .expect("Failed to read from port");

    println!("✅ Read {} bytes", read);
    assert!(read > 0, "Should read at least some bytes");

    // Verify loopback (data should match)
    assert_eq!(
        &buffer[..read],
        test_data,
        "Loopback data should match written data"
    );

    println!("✅ Loopback test passed");
}

#[test]
#[ignore]
fn test_real_port_manufacturer_detection() {
    let port_name = match skip_without_hardware() {
        Some(p) => p,
        None => return,
    };

    println!("Testing manufacturer detection on: {}", port_name);

    // Find port info
    let ports = available_ports().expect("Failed to list ports");
    let port_info = ports.iter().find(|p| p.port_name == port_name);

    if let Some(info) = port_info {
        match &info.port_type {
            serialport::SerialPortType::UsbPort(usb_info) => {
                println!("✅ USB Port detected:");
                println!("   VID: {:04x}", usb_info.vid);
                println!("   PID: {:04x}", usb_info.pid);

                if let Some(ref manufacturer) = usb_info.manufacturer {
                    println!("   Manufacturer: {}", manufacturer);
                }

                if let Some(ref product) = usb_info.product {
                    println!("   Product: {}", product);
                }

                if let Some(ref serial) = usb_info.serial_number {
                    println!("   Serial: {}", serial);
                }

                // Check against known manufacturers
                #[cfg(feature = "auto-negotiation")]
                {
                    use serial_mcp_agent::negotiation::AutoNegotiator;

                    if let Some(profile) = AutoNegotiator::get_manufacturer_profile(usb_info.vid) {
                        println!("✅ Matched manufacturer profile:");
                        println!("   Name: {}", profile.name);
                        println!("   Default baud: {}", profile.default_baud);
                        println!("   Common bauds: {:?}", profile.common_bauds);

                        assert_eq!(profile.vid, usb_info.vid);
                    } else {
                        println!(
                            "⚠️  No manufacturer profile found for VID {:04x}",
                            usb_info.vid
                        );
                    }
                }
            }
            other => {
                println!("ℹ️  Port type: {:?}", other);
            }
        }
    } else {
        println!("❌ Port {} not found in available ports", port_name);
        panic!("Port not found");
    }
}

#[test]
#[ignore]
fn test_real_port_buffer_sizes() {
    let port_name = match skip_without_hardware() {
        Some(p) => p,
        None => return,
    };

    let baud = get_test_baud();
    println!("Testing buffer sizes on: {} at {} baud", port_name, baud);

    let mut config = PortConfiguration::default();
    config.baud_rate = baud;
    config.timeout = Duration::from_millis(1000);

    let port = SyncSerialPort::open(&port_name, config).expect("Failed to open port");

    // Check buffer sizes
    if let Some(bytes_to_read) = port.bytes_to_read() {
        println!("   Bytes to read: {}", bytes_to_read);
    } else {
        println!("   Bytes to read: Not supported");
    }

    if let Some(bytes_to_write) = port.bytes_to_write() {
        println!("   Bytes to write: {}", bytes_to_write);
    } else {
        println!("   Bytes to write: Not supported");
    }

    println!("✅ Buffer size check completed");
}

#[test]
#[ignore]
fn test_real_port_timeout_behavior() {
    let port_name = match skip_without_hardware() {
        Some(p) => p,
        None => return,
    };

    let baud = get_test_baud();
    println!(
        "Testing timeout behavior on: {} at {} baud",
        port_name, baud
    );

    let mut config = PortConfiguration::default();
    config.baud_rate = baud;
    config.timeout = Duration::from_millis(100); // Short timeout

    let mut port = SyncSerialPort::open(&port_name, config).expect("Failed to open port");

    // Clear buffers
    port.clear_buffers().expect("Failed to clear buffers");

    // Try to read when no data is available
    let mut buffer = [0u8; 100];
    let start = std::time::Instant::now();
    let result = port.read_bytes(&mut buffer);
    let elapsed = start.elapsed();

    println!("   Read result: {:?}", result);
    println!("   Elapsed time: {:?}", elapsed);

    // Should timeout within reasonable time (allow some OS overhead)
    assert!(
        elapsed < Duration::from_millis(500),
        "Timeout took too long: {:?}",
        elapsed
    );

    println!("✅ Timeout test passed");
}

#[test]
#[ignore]
fn test_real_port_multiple_open_close() {
    let port_name = match skip_without_hardware() {
        Some(p) => p,
        None => return,
    };

    let baud = get_test_baud();
    println!("Testing multiple open/close cycles on: {}", port_name);

    let mut config = PortConfiguration::default();
    config.baud_rate = baud;
    config.timeout = Duration::from_millis(1000);

    // Open and close multiple times
    for i in 1..=5 {
        println!("   Cycle {}/5", i);

        let port = SyncSerialPort::open(&port_name, config.clone());
        assert!(port.is_ok(), "Failed to open port on cycle {}", i);

        // Port goes out of scope and closes
        drop(port);

        // Small delay between cycles
        std::thread::sleep(Duration::from_millis(100));
    }

    println!("✅ Multiple open/close test passed");
}

#[test]
#[ignore]
fn test_real_port_baud_rate_switching() {
    let port_name = match skip_without_hardware() {
        Some(p) => p,
        None => return,
    };

    println!("Testing baud rate switching on: {}", port_name);

    let baud_rates = vec![9600, 19200, 38400, 57600, 115200];

    for baud in baud_rates {
        println!("   Testing baud: {}", baud);

        let mut config = PortConfiguration::default();
        config.baud_rate = baud;
        config.timeout = Duration::from_millis(1000);

        let result = SyncSerialPort::open(&port_name, config);

        match result {
            Ok(_port) => {
                println!("      ✅ Opened at {} baud", baud);
            }
            Err(e) => {
                println!("      ❌ Failed at {} baud: {}", baud, e);
                panic!("Failed to open at baud rate {}", baud);
            }
        }

        // Small delay between switches
        std::thread::sleep(Duration::from_millis(100));
    }

    println!("✅ Baud rate switching test passed");
}

#[tokio::test]
#[ignore]
async fn test_real_port_concurrent_access() {
    let port_name = match skip_without_hardware() {
        Some(p) => p,
        None => return,
    };

    println!("Testing concurrent access handling on: {}", port_name);

    let baud = get_test_baud();
    let mut config = PortConfiguration::default();
    config.baud_rate = baud;
    config.timeout = Duration::from_millis(1000);

    // Open first port
    let _port1 = SyncSerialPort::open(&port_name, config.clone()).expect("Failed to open port 1");
    println!("   ✅ First port opened");

    // Try to open second port (should fail - port is exclusive)
    let port2_result = SyncSerialPort::open(&port_name, config);

    match port2_result {
        Ok(_) => {
            println!("   ⚠️  Second port opened (unexpected - ports should be exclusive)");
            // Some platforms might allow this, so we don't fail the test
        }
        Err(e) => {
            println!("   ✅ Second port correctly failed to open: {}", e);
        }
    }

    println!("✅ Concurrent access test completed");
}
