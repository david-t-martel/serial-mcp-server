//! Port-specific error types.
//!
//! Defines error types for serial port operations, separate from application-level
//! errors to maintain clean separation of concerns.

use thiserror::Error;

/// Errors that can occur during serial port operations.
#[derive(Debug, Error)]
pub enum PortError {
    /// The specified serial port was not found on the system.
    #[error("Serial port not found: {0}")]
    NotFound(String),

    /// An I/O error occurred during port operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Port configuration failed.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Operation timed out.
    #[error("Operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Attempted to open a port that's already open.
    #[error("Port is already open")]
    AlreadyOpen,

    /// Attempted to use a port that's not open.
    #[error("Port is not open")]
    NotOpen,

    /// A serialport-specific error occurred.
    #[error("Serial port error: {0}")]
    Serial(#[from] serialport::Error),
}

/// Convert from serialport::ErrorKind to PortError for better error handling.
impl PortError {
    /// Create a NotFound error from a port name.
    pub fn not_found(port_name: impl Into<String>) -> Self {
        Self::NotFound(port_name.into())
    }

    /// Create a Config error from a message.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Create a Timeout error from a duration.
    pub fn timeout(duration: std::time::Duration) -> Self {
        Self::Timeout(duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PortError::not_found("/dev/ttyUSB0");
        assert_eq!(err.to_string(), "Serial port not found: /dev/ttyUSB0");

        let err = PortError::config("Invalid baud rate");
        assert_eq!(err.to_string(), "Configuration error: Invalid baud rate");

        let err = PortError::AlreadyOpen;
        assert_eq!(err.to_string(), "Port is already open");
    }

    #[test]
    fn test_timeout_error() {
        let duration = std::time::Duration::from_millis(500);
        let err = PortError::timeout(duration);
        assert!(err.to_string().contains("500ms"));
    }
}
