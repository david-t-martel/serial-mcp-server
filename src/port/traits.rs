//! Core traits for serial port abstraction.
//!
//! Defines the `SerialPortAdapter` trait that allows both real serial ports
//! and mock implementations to be used interchangeably.

use super::error::PortError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration parameters for a serial port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfiguration {
    /// Baud rate (bits per second).
    pub baud_rate: u32,

    /// Number of data bits (5, 6, 7, or 8).
    pub data_bits: DataBits,

    /// Flow control mode.
    pub flow_control: FlowControl,

    /// Parity checking mode.
    pub parity: Parity,

    /// Number of stop bits.
    pub stop_bits: StopBits,

    /// Read/write timeout.
    pub timeout: Duration,
}

impl Default for PortConfiguration {
    fn default() -> Self {
        Self {
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_secs(1),
        }
    }
}

/// Number of data bits per character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
}

impl From<DataBits> for serialport::DataBits {
    fn from(bits: DataBits) -> Self {
        match bits {
            DataBits::Five => serialport::DataBits::Five,
            DataBits::Six => serialport::DataBits::Six,
            DataBits::Seven => serialport::DataBits::Seven,
            DataBits::Eight => serialport::DataBits::Eight,
        }
    }
}

/// Flow control modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowControl {
    None,
    Software,
    Hardware,
}

impl From<FlowControl> for serialport::FlowControl {
    fn from(flow: FlowControl) -> Self {
        match flow {
            FlowControl::None => serialport::FlowControl::None,
            FlowControl::Software => serialport::FlowControl::Software,
            FlowControl::Hardware => serialport::FlowControl::Hardware,
        }
    }
}

/// Parity checking modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Parity {
    None,
    Odd,
    Even,
}

impl From<Parity> for serialport::Parity {
    fn from(parity: Parity) -> Self {
        match parity {
            Parity::None => serialport::Parity::None,
            Parity::Odd => serialport::Parity::Odd,
            Parity::Even => serialport::Parity::Even,
        }
    }
}

/// Number of stop bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopBits {
    One,
    Two,
}

impl From<StopBits> for serialport::StopBits {
    fn from(bits: StopBits) -> Self {
        match bits {
            StopBits::One => serialport::StopBits::One,
            StopBits::Two => serialport::StopBits::Two,
        }
    }
}

/// Trait for serial port I/O operations.
///
/// This trait abstracts over synchronous serial port operations, allowing both
/// real hardware ports and mock implementations for testing.
pub trait SerialPortAdapter: Send + std::fmt::Debug {
    /// Write bytes to the serial port.
    ///
    /// Returns the number of bytes actually written.
    fn write_bytes(&mut self, data: &[u8]) -> Result<usize, PortError>;

    /// Read bytes from the serial port into the provided buffer.
    ///
    /// Returns the number of bytes actually read.
    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, PortError>;

    /// Get the name/path of this serial port.
    fn name(&self) -> &str;

    /// Set the read/write timeout for this port.
    fn set_timeout(&mut self, timeout: Duration) -> Result<(), PortError>;

    /// Clear both input and output buffers.
    ///
    /// This discards any unread data in the receive buffer and any unsent
    /// data in the transmit buffer.
    fn clear_buffers(&mut self) -> Result<(), PortError>;

    /// Get the current bytes available to read (if supported).
    ///
    /// Returns `None` if the operation is not supported or cannot be determined.
    fn bytes_to_read(&self) -> Option<usize> {
        None
    }

    /// Get the current bytes waiting to be written (if supported).
    ///
    /// Returns `None` if the operation is not supported or cannot be determined.
    fn bytes_to_write(&self) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = PortConfiguration::default();
        assert_eq!(config.baud_rate, 9600);
        assert_eq!(config.data_bits, DataBits::Eight);
        assert_eq!(config.flow_control, FlowControl::None);
        assert_eq!(config.parity, Parity::None);
        assert_eq!(config.stop_bits, StopBits::One);
        assert_eq!(config.timeout, Duration::from_secs(1));
    }

    #[test]
    fn test_data_bits_conversion() {
        let bits = DataBits::Eight;
        let serialport_bits: serialport::DataBits = bits.into();
        assert_eq!(serialport_bits, serialport::DataBits::Eight);
    }

    #[test]
    fn test_flow_control_conversion() {
        let flow = FlowControl::Hardware;
        let serialport_flow: serialport::FlowControl = flow.into();
        assert_eq!(serialport_flow, serialport::FlowControl::Hardware);
    }

    #[test]
    fn test_parity_conversion() {
        let parity = Parity::Even;
        let serialport_parity: serialport::Parity = parity.into();
        assert_eq!(serialport_parity, serialport::Parity::Even);
    }

    #[test]
    fn test_stop_bits_conversion() {
        let stop_bits = StopBits::Two;
        let serialport_stop_bits: serialport::StopBits = stop_bits.into();
        assert_eq!(serialport_stop_bits, serialport::StopBits::Two);
    }
}
