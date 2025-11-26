//! Example demonstrating the port abstraction module.
//!
//! This example shows how to use both the real `SyncSerialPort` and `MockSerialPort`
//! for serial communication with dependency injection.

#![allow(clippy::field_reassign_with_default)]
#![allow(dead_code)]

use serial_mcp_agent::port::{MockSerialPort, PortConfiguration, SerialPortAdapter};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Port Abstraction Example ===\n");

    // Example 1: Using MockSerialPort for testing
    println!("1. Using MockSerialPort (for testing):");
    mock_example()?;

    println!("\n2. Real port configuration (demonstration only):");
    real_port_config_example();

    println!("\n=== Example complete ===");
    Ok(())
}

/// Demonstrates using MockSerialPort for testing without hardware
fn mock_example() -> Result<(), Box<dyn std::error::Error>> {
    let mut port = MockSerialPort::new("MOCK0");

    // Enqueue some data to be read
    port.enqueue_read(b"Hello from mock port!\n");

    // Read the data
    let mut buffer = [0u8; 64];
    let bytes_read = port.read_bytes(&mut buffer)?;
    let data = String::from_utf8_lossy(&buffer[..bytes_read]);
    println!("  Read {} bytes: {}", bytes_read, data.trim());

    // Write some data
    let write_data = b"ACK\n";
    let bytes_written = port.write_bytes(write_data)?;
    println!("  Wrote {} bytes", bytes_written);

    // Verify what was written
    let write_log = port.get_write_log();
    println!("  Write log contains {} entries", write_log.len());

    // Test timeout simulation
    port.set_should_timeout(true);
    match port.read_bytes(&mut buffer) {
        Err(e) => println!("  Timeout simulated successfully: {}", e),
        Ok(_) => println!("  Unexpected: read succeeded during timeout simulation"),
    }

    // Test clear buffers
    port.enqueue_read(b"This will be cleared");
    port.clear_buffers()?;
    println!("  Buffers cleared: {}", port.was_cleared());

    Ok(())
}

/// Demonstrates creating a configuration for a real serial port
fn real_port_config_example() {
    use serial_mcp_agent::port::{DataBits, FlowControl, Parity, StopBits};

    let config = PortConfiguration {
        baud_rate: 115200,
        data_bits: DataBits::Eight,
        flow_control: FlowControl::None,
        parity: Parity::None,
        stop_bits: StopBits::One,
        timeout: Duration::from_millis(500),
    };

    println!("  Configuration:");
    println!("    Baud rate: {}", config.baud_rate);
    println!("    Data bits: {:?}", config.data_bits);
    println!("    Parity: {:?}", config.parity);
    println!("    Stop bits: {:?}", config.stop_bits);
    println!("    Timeout: {:?}", config.timeout);

    // Note: To actually open a port, you would do:
    // let port = SyncSerialPort::open("/dev/ttyUSB0", config)?;
    // But this requires actual hardware to be present
    println!("\n  To open a real port:");
    println!("    let port = SyncSerialPort::open(\"/dev/ttyUSB0\", config)?;");
    println!("  This requires the specified hardware to be connected.");
}

/// Example of a function that works with any SerialPortAdapter
fn communicate_with_device<P: SerialPortAdapter>(
    port: &mut P,
    command: &[u8],
) -> Result<String, Box<dyn std::error::Error>> {
    // Write command
    port.write_bytes(command)?;

    // Read response
    let mut buffer = [0u8; 256];
    let bytes_read = port.read_bytes(&mut buffer)?;

    Ok(String::from_utf8_lossy(&buffer[..bytes_read]).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_port_communication() {
        let mut port = MockSerialPort::new("TEST");
        port.enqueue_read(b"OK\n");

        let response = communicate_with_device(&mut port, b"PING\n").unwrap();
        assert_eq!(response, "OK\n");

        let writes = port.get_write_log();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0], b"PING\n");
    }

    #[test]
    fn test_port_configuration_default() {
        let config = PortConfiguration::default();
        assert_eq!(config.baud_rate, 9600);
        assert_eq!(config.data_bits, DataBits::Eight);
    }
}
