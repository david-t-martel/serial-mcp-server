//! Async serial port implementation using tokio-serial.
//!
//! Provides async serial port operations for use in Tokio-based applications,
//! with both native async support and a blocking wrapper for sync ports.
//!
//! Note: This module is gated behind the `async-serial` feature flag.

#![allow(clippy::duplicated_attributes)]

use super::error::PortError;
use super::sync_port::SyncSerialPort;
use super::traits::{PortConfiguration, SerialPortAdapter};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Async trait for serial port operations using Tokio.
///
/// This trait provides an async interface for serial port I/O operations,
/// enabling non-blocking communication in async applications.
///
/// Note: This trait requires `Send` but not `Sync` because serial ports
/// are typically accessed exclusively (mutable access only).
#[async_trait]
pub trait AsyncSerialPortAdapter: Send {
    /// Write bytes to the serial port asynchronously.
    ///
    /// Returns the number of bytes actually written.
    async fn write_bytes(&mut self, data: &[u8]) -> Result<usize, PortError>;

    /// Read bytes from the serial port into the provided buffer asynchronously.
    ///
    /// Returns the number of bytes actually read.
    async fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, PortError>;

    /// Get the name/path of this serial port.
    fn name(&self) -> &str;

    /// Get the port configuration.
    fn config(&self) -> &PortConfiguration;

    /// Get the number of bytes available to read.
    ///
    /// This may not be supported on all platforms.
    async fn bytes_available(&mut self) -> Result<u32, PortError>;
}

/// Native async serial port implementation using tokio-serial.
///
/// This implementation provides true async I/O operations using the tokio-serial
/// crate, which integrates with Tokio's async runtime.
pub struct TokioSerialPort {
    /// The underlying tokio-serial stream.
    inner: tokio_serial::SerialStream,
    /// Port configuration for reference.
    config: PortConfiguration,
    /// Port name/path for identification.
    name: String,
}

impl TokioSerialPort {
    /// Open a serial port with async I/O support.
    ///
    /// # Arguments
    /// * `config` - Configuration parameters for the port
    ///
    /// # Example
    /// ```no_run
    /// use serial_mcp_agent::port::{TokioSerialPort, PortConfiguration};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut config = PortConfiguration::default();
    /// config.baud_rate = 115200;
    /// let port = TokioSerialPort::open("/dev/ttyUSB0", &config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open(port_name: &str, config: &PortConfiguration) -> Result<Self, PortError> {
        // Build tokio-serial configuration
        let mut builder = tokio_serial::new(port_name, config.baud_rate);

        // Configure serial port parameters
        builder = builder
            .data_bits(convert_data_bits(config.data_bits))
            .flow_control(convert_flow_control(config.flow_control))
            .parity(convert_parity(config.parity))
            .stop_bits(convert_stop_bits(config.stop_bits))
            .timeout(config.timeout);

        // Open the port
        let inner = tokio_serial::SerialStream::open(&builder).map_err(|e| match e.kind {
            tokio_serial::ErrorKind::NoDevice => PortError::not_found(port_name),
            tokio_serial::ErrorKind::InvalidInput => PortError::config(e.to_string()),
            _ => PortError::Io(std::io::Error::other(
                e.to_string(),
            )),
        })?;

        Ok(Self {
            inner,
            config: config.clone(),
            name: port_name.to_string(),
        })
    }

    /// Get a reference to the underlying tokio_serial::SerialStream.
    ///
    /// This can be useful for accessing platform-specific features.
    pub fn as_raw(&self) -> &tokio_serial::SerialStream {
        &self.inner
    }

    /// Get a mutable reference to the underlying tokio_serial::SerialStream.
    ///
    /// This can be useful for accessing platform-specific features.
    pub fn as_raw_mut(&mut self) -> &mut tokio_serial::SerialStream {
        &mut self.inner
    }
}

#[async_trait]
impl AsyncSerialPortAdapter for TokioSerialPort {
    async fn write_bytes(&mut self, data: &[u8]) -> Result<usize, PortError> {
        self.inner.write(data).await.map_err(PortError::Io)
    }

    async fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, PortError> {
        self.inner.read(buffer).await.map_err(PortError::Io)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &PortConfiguration {
        &self.config
    }

    async fn bytes_available(&mut self) -> Result<u32, PortError> {
        use serialport::SerialPort;
        self.inner
            .bytes_to_read()
            .map_err(|e| PortError::Io(std::io::Error::other(e)))
    }
}

impl std::fmt::Debug for TokioSerialPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokioSerialPort")
            .field("name", &self.name)
            .field("config", &self.config)
            .finish()
    }
}

/// Wrapper that provides async interface for synchronous serial ports.
///
/// This wrapper uses `tokio::task::spawn_blocking` to run blocking serial port
/// operations in a separate thread pool, preventing them from blocking the async
/// runtime. This is useful for integrating legacy sync code into async applications.
pub struct BlockingSerialPortWrapper {
    /// The sync port wrapped in Arc<Mutex> for shared access.
    inner: Arc<Mutex<SyncSerialPort>>,
    /// Port configuration for reference.
    config: PortConfiguration,
    /// Port name cached for quick access.
    name: String,
}

impl BlockingSerialPortWrapper {
    /// Create a new wrapper around a synchronous serial port.
    ///
    /// # Example
    /// ```no_run
    /// use serial_mcp_agent::port::{SyncSerialPort, BlockingSerialPortWrapper, PortConfiguration};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = PortConfiguration::default();
    /// let sync_port = SyncSerialPort::open("/dev/ttyUSB0", config.clone())?;
    /// let async_port = BlockingSerialPortWrapper::new(sync_port, config);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(port: SyncSerialPort, config: PortConfiguration) -> Self {
        let name = port.name().to_string();
        Self {
            inner: Arc::new(Mutex::new(port)),
            config,
            name,
        }
    }

    /// Open a serial port and wrap it for async use.
    ///
    /// This is a convenience method that combines `SyncSerialPort::open` with wrapping.
    pub fn open(port_name: &str, config: PortConfiguration) -> Result<Self, PortError> {
        let port = SyncSerialPort::open(port_name, config.clone())?;
        Ok(Self::new(port, config))
    }
}

#[async_trait]
impl AsyncSerialPortAdapter for BlockingSerialPortWrapper {
    async fn write_bytes(&mut self, data: &[u8]) -> Result<usize, PortError> {
        let data = data.to_vec();
        let inner = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || {
            let mut port = inner.lock();
            port.write_bytes(&data)
        })
        .await
        .map_err(|e| PortError::Io(std::io::Error::other(e)))?
    }

    async fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, PortError> {
        let buffer_len = buffer.len();
        let inner = Arc::clone(&self.inner);

        let result = tokio::task::spawn_blocking(move || {
            let mut port = inner.lock();
            let mut temp_buffer = vec![0u8; buffer_len];
            let bytes_read = port.read_bytes(&mut temp_buffer)?;
            Ok::<(Vec<u8>, usize), PortError>((temp_buffer, bytes_read))
        })
        .await
        .map_err(|e| PortError::Io(std::io::Error::other(e)))??;

        let (temp_buffer, bytes_read) = result;
        buffer[..bytes_read].copy_from_slice(&temp_buffer[..bytes_read]);
        Ok(bytes_read)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &PortConfiguration {
        &self.config
    }

    async fn bytes_available(&mut self) -> Result<u32, PortError> {
        let inner = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || {
            let port = inner.lock();
            port.bytes_to_read()
                .ok_or_else(|| PortError::config("bytes_to_read not supported"))
                .map(|n| n as u32)
        })
        .await
        .map_err(|e| PortError::Io(std::io::Error::other(e)))?
    }
}

impl std::fmt::Debug for BlockingSerialPortWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockingSerialPortWrapper")
            .field("name", &self.name)
            .field("config", &self.config)
            .finish()
    }
}

// Helper conversion functions for tokio-serial types

fn convert_data_bits(bits: super::traits::DataBits) -> tokio_serial::DataBits {
    use super::traits::DataBits;
    match bits {
        DataBits::Five => tokio_serial::DataBits::Five,
        DataBits::Six => tokio_serial::DataBits::Six,
        DataBits::Seven => tokio_serial::DataBits::Seven,
        DataBits::Eight => tokio_serial::DataBits::Eight,
    }
}

fn convert_flow_control(flow: super::traits::FlowControl) -> tokio_serial::FlowControl {
    use super::traits::FlowControl;
    match flow {
        FlowControl::None => tokio_serial::FlowControl::None,
        FlowControl::Software => tokio_serial::FlowControl::Software,
        FlowControl::Hardware => tokio_serial::FlowControl::Hardware,
    }
}

fn convert_parity(parity: super::traits::Parity) -> tokio_serial::Parity {
    use super::traits::Parity;
    match parity {
        Parity::None => tokio_serial::Parity::None,
        Parity::Odd => tokio_serial::Parity::Odd,
        Parity::Even => tokio_serial::Parity::Even,
    }
}

fn convert_stop_bits(stop_bits: super::traits::StopBits) -> tokio_serial::StopBits {
    use super::traits::StopBits;
    match stop_bits {
        StopBits::One => tokio_serial::StopBits::One,
        StopBits::Two => tokio_serial::StopBits::Two,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::traits::DataBits;

    #[test]
    fn test_data_bits_conversion() {
        assert_eq!(
            convert_data_bits(DataBits::Eight),
            tokio_serial::DataBits::Eight
        );
        assert_eq!(
            convert_data_bits(DataBits::Seven),
            tokio_serial::DataBits::Seven
        );
    }

    #[test]
    fn test_flow_control_conversion() {
        use super::super::traits::FlowControl;
        assert_eq!(
            convert_flow_control(FlowControl::Hardware),
            tokio_serial::FlowControl::Hardware
        );
        assert_eq!(
            convert_flow_control(FlowControl::None),
            tokio_serial::FlowControl::None
        );
    }

    #[test]
    fn test_parity_conversion() {
        use super::super::traits::Parity;
        assert_eq!(convert_parity(Parity::Even), tokio_serial::Parity::Even);
        assert_eq!(convert_parity(Parity::None), tokio_serial::Parity::None);
    }

    #[test]
    fn test_stop_bits_conversion() {
        use super::super::traits::StopBits;
        assert_eq!(
            convert_stop_bits(StopBits::Two),
            tokio_serial::StopBits::Two
        );
        assert_eq!(
            convert_stop_bits(StopBits::One),
            tokio_serial::StopBits::One
        );
    }

    #[tokio::test]
    async fn test_tokio_port_not_found_error() {
        let config = PortConfiguration::default();
        let result = TokioSerialPort::open("/dev/nonexistent_async_port_12345", &config);

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

    #[tokio::test]
    async fn test_blocking_wrapper_not_found_error() {
        let config = PortConfiguration::default();
        let result =
            BlockingSerialPortWrapper::open("/dev/nonexistent_blocking_port_12345", config);

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

    // Integration test with mock would go here
    // For now, we test conversions and error handling
}
