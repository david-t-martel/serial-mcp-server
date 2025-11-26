//! Utility functions for hardware testing.
//!
//! Provides helpers for port discovery, test setup/teardown, and timing utilities.

#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::assertions_on_constants)]

use serial_mcp_agent::port::{PortConfiguration, SerialPortAdapter, SyncSerialPort};
use serialport::{available_ports, SerialPortInfo, SerialPortType};
use std::env;
use std::time::{Duration, Instant};

/// Test port configuration from environment.
pub struct TestPortConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub loopback_enabled: bool,
}

impl TestPortConfig {
    /// Get test configuration from environment variables.
    pub fn from_env() -> Option<Self> {
        let port_name = env::var("TEST_PORT").ok()?;
        let baud_rate = env::var("TEST_BAUD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(9600);
        let loopback_enabled = env::var("TEST_LOOPBACK").ok().as_deref() == Some("1");

        Some(TestPortConfig {
            port_name,
            baud_rate,
            loopback_enabled,
        })
    }

    /// Create a port configuration for testing.
    pub fn to_port_config(&self) -> PortConfiguration {
        let mut config = PortConfiguration::default();
        config.baud_rate = self.baud_rate;
        config.timeout = Duration::from_millis(1000);
        config
    }
}

/// Discover all available serial ports on the system.
pub fn discover_available_ports() -> Vec<SerialPortInfo> {
    available_ports().unwrap_or_default()
}

/// Find USB serial ports (excludes Bluetooth and other types).
pub fn discover_usb_ports() -> Vec<SerialPortInfo> {
    discover_available_ports()
        .into_iter()
        .filter(|port| matches!(port.port_type, SerialPortType::UsbPort(_)))
        .collect()
}

/// Print available ports for debugging.
pub fn print_available_ports() {
    let ports = discover_available_ports();

    if ports.is_empty() {
        println!("No serial ports detected on this system");
        return;
    }

    println!("Available serial ports ({}):", ports.len());
    for (idx, port) in ports.iter().enumerate() {
        println!("  {}. {}", idx + 1, port.port_name);

        match &port.port_type {
            SerialPortType::UsbPort(usb_info) => {
                println!("     Type: USB");
                println!("     VID:PID = {:04x}:{:04x}", usb_info.vid, usb_info.pid);

                if let Some(ref manufacturer) = usb_info.manufacturer {
                    println!("     Manufacturer: {}", manufacturer);
                }

                if let Some(ref product) = usb_info.product {
                    println!("     Product: {}", product);
                }

                if let Some(ref serial) = usb_info.serial_number {
                    println!("     Serial: {}", serial);
                }
            }
            SerialPortType::BluetoothPort => {
                println!("     Type: Bluetooth");
            }
            SerialPortType::PciPort => {
                println!("     Type: PCI");
            }
            SerialPortType::Unknown => {
                println!("     Type: Unknown");
            }
        }
    }
}

/// Check if a specific port is available.
pub fn is_port_available(port_name: &str) -> bool {
    discover_available_ports()
        .iter()
        .any(|p| p.port_name == port_name)
}

/// Get USB VID for a port if available.
pub fn get_port_vid(port_name: &str) -> Option<u16> {
    discover_available_ports()
        .iter()
        .find(|p| p.port_name == port_name)
        .and_then(|port| {
            if let SerialPortType::UsbPort(usb_info) = &port.port_type {
                Some(usb_info.vid)
            } else {
                None
            }
        })
}

/// Get port info for a specific port.
pub fn get_port_info(port_name: &str) -> Option<SerialPortInfo> {
    discover_available_ports()
        .into_iter()
        .find(|p| p.port_name == port_name)
}

/// Timing helper for measuring operation duration.
pub struct TimingHelper {
    start: Instant,
    name: String,
}

impl TimingHelper {
    pub fn new(name: &str) -> Self {
        println!("⏱️  Starting: {}", name);
        TimingHelper {
            start: Instant::now(),
            name: name.to_string(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn checkpoint(&self, message: &str) {
        println!("   {} - {:?} elapsed", message, self.elapsed());
    }

    pub fn finish(self) -> Duration {
        let elapsed = self.elapsed();
        println!("✅ Completed: {} in {:?}", self.name, elapsed);
        elapsed
    }

    pub fn finish_with_result<T>(self, result: &Result<T, impl std::fmt::Display>) -> Duration {
        let elapsed = self.elapsed();
        match result {
            Ok(_) => println!("✅ Completed: {} in {:?}", self.name, elapsed),
            Err(e) => println!("❌ Failed: {} in {:?} - {}", self.name, elapsed, e),
        }
        elapsed
    }
}

/// Test fixture for serial port testing.
pub struct PortTestFixture {
    pub port: SyncSerialPort,
    config: TestPortConfig,
}

impl PortTestFixture {
    /// Create a new test fixture from environment configuration.
    pub fn setup() -> Option<Self> {
        let config = TestPortConfig::from_env()?;

        println!(
            "Setting up test fixture for {} at {} baud",
            config.port_name, config.baud_rate
        );

        let port = match SyncSerialPort::open(&config.port_name, config.to_port_config()) {
            Ok(p) => p,
            Err(e) => {
                println!("Failed to open port: {}", e);
                return None;
            }
        };

        Some(PortTestFixture { port, config })
    }

    /// Get reference to the port.
    pub fn port(&self) -> &SyncSerialPort {
        &self.port
    }

    /// Get mutable reference to the port.
    pub fn port_mut(&mut self) -> &mut SyncSerialPort {
        &mut self.port
    }

    /// Check if loopback is enabled.
    pub fn is_loopback(&self) -> bool {
        self.config.loopback_enabled
    }

    /// Get the port name.
    pub fn port_name(&self) -> &str {
        &self.config.port_name
    }

    /// Get the baud rate.
    pub fn baud_rate(&self) -> u32 {
        self.config.baud_rate
    }

    /// Clear port buffers.
    pub fn clear_buffers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.port.clear_buffers()?;
        Ok(())
    }

    /// Cleanup (called automatically on drop).
    pub fn teardown(self) {
        println!("Tearing down test fixture for {}", self.config.port_name);
        drop(self.port);
    }
}

/// Skip test with a clear message if hardware is not available.
#[macro_export]
macro_rules! skip_without_hardware {
    () => {
        if $crate::hardware::utils::TestPortConfig::from_env().is_none() {
            println!("⏭️  Skipping: TEST_PORT environment variable not set");
            println!("   Set TEST_PORT=COM3 (or /dev/ttyUSB0) to run hardware tests");
            return;
        }
    };
}

/// Skip test with a clear message if loopback is not enabled.
#[macro_export]
macro_rules! skip_without_loopback {
    () => {
        let config = match $crate::hardware::utils::TestPortConfig::from_env() {
            Some(c) => c,
            None => {
                println!("⏭️  Skipping: TEST_PORT environment variable not set");
                return;
            }
        };

        if !config.loopback_enabled {
            println!("⏭️  Skipping: TEST_LOOPBACK not set to 1");
            println!("   This test requires a loopback adapter (TX connected to RX)");
            return;
        }
    };
}

/// Assert that duration is within expected range.
pub fn assert_duration_within(
    actual: Duration,
    expected: Duration,
    tolerance: Duration,
    message: &str,
) {
    let lower = expected.saturating_sub(tolerance);
    let upper = expected + tolerance;

    assert!(
        actual >= lower && actual <= upper,
        "{}: expected {:?} ± {:?}, got {:?}",
        message,
        expected,
        tolerance,
        actual
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_ports() {
        // Should not panic
        let ports = discover_available_ports();
        println!("Found {} ports", ports.len());
        // Can't assert specific count as it depends on system
    }

    #[test]
    fn test_discover_usb_ports() {
        // Should not panic
        let usb_ports = discover_usb_ports();
        println!("Found {} USB ports", usb_ports.len());
    }

    #[test]
    fn test_timing_helper() {
        let timer = TimingHelper::new("test operation");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.finish();
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_assert_duration_within() {
        let duration = Duration::from_millis(100);
        assert_duration_within(
            duration,
            Duration::from_millis(95),
            Duration::from_millis(10),
            "should be within tolerance",
        );
    }

    #[test]
    #[should_panic]
    fn test_assert_duration_out_of_range() {
        let duration = Duration::from_millis(200);
        assert_duration_within(
            duration,
            Duration::from_millis(100),
            Duration::from_millis(10),
            "should panic",
        );
    }
}
