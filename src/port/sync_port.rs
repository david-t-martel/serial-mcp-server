//! Synchronous serial port implementation.
//!
//! Wraps the `serialport` crate's `SerialPort` trait with our own `SerialPortAdapter`
//! trait for dependency injection and testing.

use super::error::PortError;
use super::traits::{PortConfiguration, SerialPortAdapter};
use std::io::{Read, Write};
use std::time::Duration;

/// Synchronous serial port implementation wrapping `serialport::SerialPort`.
pub struct SyncSerialPort {
    /// The underlying serial port implementation.
    port: Box<dyn serialport::SerialPort>,
    /// The port name/path for identification.
    name: String,
}

impl SyncSerialPort {
    /// Open a serial port with the given configuration.
    ///
    /// # Arguments
    /// * `port_name` - The system path to the serial port (e.g., "/dev/ttyUSB0" or "COM3")
    /// * `config` - Configuration parameters for the port
    ///
    /// # Example
    /// ```no_run
    /// use serial_mcp_agent::port::{SyncSerialPort, PortConfiguration};
    ///
    /// let config = PortConfiguration::default();
    /// let port = SyncSerialPort::open("/dev/ttyUSB0", config)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open(port_name: &str, config: PortConfiguration) -> Result<Self, PortError> {
        let port = serialport::new(port_name, config.baud_rate)
            .data_bits(config.data_bits.into())
            .flow_control(config.flow_control.into())
            .parity(config.parity.into())
            .stop_bits(config.stop_bits.into())
            .timeout(config.timeout)
            .open()
            .map_err(|e| match e.kind() {
                serialport::ErrorKind::NoDevice => PortError::not_found(port_name),
                serialport::ErrorKind::InvalidInput => PortError::config(e.to_string()),
                _ => PortError::Serial(e),
            })?;

        Ok(Self {
            port,
            name: port_name.to_string(),
        })
    }

    /// Open a serial port with default configuration.
    ///
    /// This is a convenience method that uses 9600 baud, 8N1, no flow control.
    pub fn open_default(port_name: &str) -> Result<Self, PortError> {
        Self::open(port_name, PortConfiguration::default())
    }

    /// Get a reference to the underlying serialport implementation.
    ///
    /// This can be useful for accessing platform-specific features.
    pub fn as_raw(&self) -> &dyn serialport::SerialPort {
        &*self.port
    }

    /// Get a mutable reference to the underlying serialport implementation.
    ///
    /// This can be useful for accessing platform-specific features.
    pub fn as_raw_mut(&mut self) -> &mut dyn serialport::SerialPort {
        &mut *self.port
    }
}

impl SerialPortAdapter for SyncSerialPort {
    fn write_bytes(&mut self, data: &[u8]) -> Result<usize, PortError> {
        self.port.write(data).map_err(PortError::Io)
    }

    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, PortError> {
        self.port.read(buffer).map_err(PortError::Io)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_timeout(&mut self, timeout: Duration) -> Result<(), PortError> {
        self.port.set_timeout(timeout).map_err(PortError::Serial)
    }

    fn clear_buffers(&mut self) -> Result<(), PortError> {
        // Clear both input and output buffers
        self.port
            .clear(serialport::ClearBuffer::All)
            .map_err(PortError::Serial)
    }

    fn bytes_to_read(&self) -> Option<usize> {
        self.port.bytes_to_read().ok().map(|n| n as usize)
    }

    fn bytes_to_write(&self) -> Option<usize> {
        self.port.bytes_to_write().ok().map(|n| n as usize)
    }
}

impl std::fmt::Debug for SyncSerialPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncSerialPort")
            .field("name", &self.name)
            .field("baud_rate", &self.port.baud_rate())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_not_found_error() {
        let config = PortConfiguration::default();
        let result = SyncSerialPort::open("/dev/nonexistent_port_12345", config);

        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                PortError::NotFound(name) => {
                    assert!(name.contains("nonexistent"));
                }
                _ => panic!("Expected NotFound error, got: {:?}", e),
            }
        }
    }

    #[test]
    fn test_default_configuration() {
        let config = PortConfiguration::default();
        assert_eq!(config.baud_rate, 9600);
        assert_eq!(config.timeout, Duration::from_secs(1));
    }
}
