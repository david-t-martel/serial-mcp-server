//! Example demonstrating async serial port usage with tokio-serial.
//!
//! This example shows how to use both the native `TokioSerialPort` and the
//! `BlockingSerialPortWrapper` for async serial communication.
//!
//! Run with:
//! ```bash
//! cargo run --example async_port_usage --features async-serial
//! ```

#[cfg(feature = "async-serial")]
use serial_mcp_agent::port::{
    AsyncSerialPortAdapter, BlockingSerialPortWrapper, PortConfiguration, TokioSerialPort,
};

#[cfg(feature = "async-serial")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Async Serial Port Usage Example");
    println!("================================\n");

    // Create a configuration
    let mut config = PortConfiguration::default();
    config.baud_rate = 115200;

    println!("Configuration:");
    println!("  Baud Rate: {}", config.baud_rate);
    println!("  Data Bits: {:?}", config.data_bits);
    println!("  Parity: {:?}", config.parity);
    println!("  Stop Bits: {:?}", config.stop_bits);
    println!("  Timeout: {:?}\n", config.timeout);

    // Example 1: Using TokioSerialPort (native async)
    println!("Example 1: Native Async with TokioSerialPort");
    println!("--------------------------------------------");
    demonstrate_tokio_port(&config).await;

    // Example 2: Using BlockingSerialPortWrapper
    println!("\nExample 2: Blocking Wrapper for Sync Ports");
    println!("------------------------------------------");
    demonstrate_blocking_wrapper(&config).await;

    Ok(())
}

#[cfg(feature = "async-serial")]
async fn demonstrate_tokio_port(config: &PortConfiguration) {
    // Try to open a port (will fail if no port exists, but shows the API)
    let port_name = if cfg!(windows) {
        "COM3"
    } else {
        "/dev/ttyUSB0"
    };

    println!("Attempting to open {} with native async...", port_name);

    match TokioSerialPort::open(port_name, config) {
        Ok(mut port) => {
            println!("✓ Port opened successfully: {}", port.name());
            println!("  Config: {:?}", port.config());

            // Example: Write data
            let test_data = b"AT\r\n";
            match port.write_bytes(test_data).await {
                Ok(bytes_written) => {
                    println!("✓ Wrote {} bytes", bytes_written);
                }
                Err(e) => {
                    println!("✗ Write error: {}", e);
                }
            }

            // Example: Read data
            let mut buffer = [0u8; 128];
            match port.read_bytes(&mut buffer).await {
                Ok(bytes_read) => {
                    println!("✓ Read {} bytes", bytes_read);
                    if bytes_read > 0 {
                        println!("  Data: {}", String::from_utf8_lossy(&buffer[..bytes_read]));
                    }
                }
                Err(e) => {
                    println!("✗ Read error: {}", e);
                }
            }

            // Example: Check bytes available
            match port.bytes_available().await {
                Ok(available) => {
                    println!("✓ Bytes available: {}", available);
                }
                Err(e) => {
                    println!("✗ bytes_available error: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to open port: {}", e);
            println!("  (This is expected if no serial device is connected)");
        }
    }
}

#[cfg(feature = "async-serial")]
async fn demonstrate_blocking_wrapper(config: &PortConfiguration) {
    let port_name = if cfg!(windows) {
        "COM3"
    } else {
        "/dev/ttyUSB0"
    };

    println!("Attempting to open {} with blocking wrapper...", port_name);

    match BlockingSerialPortWrapper::open(port_name, config.clone()) {
        Ok(mut port) => {
            println!("✓ Port opened successfully: {}", port.name());
            println!("  Config: {:?}", port.config());

            // The API is identical to TokioSerialPort
            let test_data = b"AT\r\n";
            match port.write_bytes(test_data).await {
                Ok(bytes_written) => {
                    println!("✓ Wrote {} bytes (using spawn_blocking)", bytes_written);
                }
                Err(e) => {
                    println!("✗ Write error: {}", e);
                }
            }

            // Read with blocking wrapper
            let mut buffer = [0u8; 128];
            match port.read_bytes(&mut buffer).await {
                Ok(bytes_read) => {
                    println!("✓ Read {} bytes (using spawn_blocking)", bytes_read);
                    if bytes_read > 0 {
                        println!("  Data: {}", String::from_utf8_lossy(&buffer[..bytes_read]));
                    }
                }
                Err(e) => {
                    println!("✗ Read error: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to open port: {}", e);
            println!("  (This is expected if no serial device is connected)");
        }
    }
}

#[cfg(not(feature = "async-serial"))]
fn main() {
    eprintln!("This example requires the 'async-serial' feature.");
    eprintln!("Run with: cargo run --example async_port_usage --features async-serial");
    std::process::exit(1);
}
