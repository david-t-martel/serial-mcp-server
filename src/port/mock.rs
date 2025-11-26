//! Mock serial port implementation for testing.
//!
//! Provides a `MockSerialPort` that simulates serial port behavior without
//! requiring actual hardware. Supports configurable read/write queues and
//! expectation verification.

use super::error::PortError;
use super::traits::SerialPortAdapter;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Inner state of the mock port, protected by a mutex for interior mutability.
#[derive(Debug, Default)]
struct MockPortState {
    /// Queue of bytes to be returned by read operations.
    read_queue: VecDeque<u8>,
    /// Log of all bytes written to the port.
    write_log: Vec<Vec<u8>>,
    /// Expected write operations (for verification).
    expected_writes: VecDeque<Vec<u8>>,
    /// Whether the next operation should time out.
    should_timeout: bool,
    /// Configured timeout duration.
    timeout: Duration,
    /// Whether buffers have been cleared.
    buffers_cleared: bool,
}

/// Mock serial port implementation for testing.
///
/// This implementation allows you to:
/// - Enqueue data to be returned by read operations
/// - Inspect what data was written
/// - Set expectations for write operations
/// - Simulate timeouts and errors
///
/// # Example
/// ```
/// use serial_mcp_agent::port::{MockSerialPort, SerialPortAdapter};
///
/// let mut port = MockSerialPort::new("MOCK0");
///
/// // Enqueue data to be read
/// port.enqueue_read(b"Hello, World!");
///
/// // Perform a read
/// let mut buffer = [0u8; 13];
/// let n = port.read_bytes(&mut buffer).unwrap();
/// assert_eq!(n, 13);
/// assert_eq!(&buffer[..n], b"Hello, World!");
///
/// // Write some data
/// port.write_bytes(b"Response").unwrap();
///
/// // Verify what was written
/// let writes = port.get_write_log();
/// assert_eq!(writes.len(), 1);
/// assert_eq!(writes[0], b"Response");
/// ```
#[derive(Clone)]
pub struct MockSerialPort {
    /// The port name/identifier.
    name: String,
    /// The internal state, wrapped in Arc<Mutex<>> for interior mutability.
    state: Arc<Mutex<MockPortState>>,
}

impl MockSerialPort {
    /// Create a new mock serial port with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: Arc::new(Mutex::new(MockPortState {
                timeout: Duration::from_secs(1),
                ..Default::default()
            })),
        }
    }

    /// Enqueue bytes to be returned by subsequent read operations.
    ///
    /// The bytes are added to the end of the read queue.
    pub fn enqueue_read(&mut self, data: &[u8]) {
        let mut state = self.state.lock().unwrap();
        state.read_queue.extend(data);
    }

    /// Expect a specific write operation.
    ///
    /// This adds an expectation that the given data will be written.
    /// Use `verify_expectations()` to check that all expected writes occurred.
    pub fn expect_write(&mut self, data: &[u8]) {
        let mut state = self.state.lock().unwrap();
        state.expected_writes.push_back(data.to_vec());
    }

    /// Verify that all expected writes have occurred in order.
    ///
    /// Returns `Ok(())` if all expectations were met, or an error describing
    /// what was expected vs. what actually happened.
    pub fn verify_expectations(&self) -> Result<(), String> {
        let state = self.state.lock().unwrap();

        if !state.expected_writes.is_empty() {
            return Err(format!(
                "Expected {} more write(s), but none occurred",
                state.expected_writes.len()
            ));
        }

        Ok(())
    }

    /// Get a copy of all data written to the port.
    pub fn get_write_log(&self) -> Vec<Vec<u8>> {
        let state = self.state.lock().unwrap();
        state.write_log.clone()
    }

    /// Clear the write log.
    pub fn clear_write_log(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.write_log.clear();
    }

    /// Set whether the next read/write operation should time out.
    pub fn set_should_timeout(&mut self, should_timeout: bool) {
        let mut state = self.state.lock().unwrap();
        state.should_timeout = should_timeout;
    }

    /// Get whether buffers have been cleared since the last reset.
    pub fn was_cleared(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.buffers_cleared
    }

    /// Reset the "buffers cleared" flag.
    pub fn reset_cleared_flag(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.buffers_cleared = false;
    }

    /// Get the number of bytes available to read.
    pub fn available_bytes(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.read_queue.len()
    }
}

impl SerialPortAdapter for MockSerialPort {
    fn write_bytes(&mut self, data: &[u8]) -> Result<usize, PortError> {
        let mut state = self.state.lock().unwrap();

        // Check if we should simulate a timeout
        if state.should_timeout {
            state.should_timeout = false;
            return Err(PortError::timeout(state.timeout));
        }

        // Log the write
        state.write_log.push(data.to_vec());

        // Check expectations if any exist
        if let Some(expected) = state.expected_writes.pop_front() {
            if expected != data {
                return Err(PortError::config(format!(
                    "Expected write: {:?}, got: {:?}",
                    expected, data
                )));
            }
        }

        Ok(data.len())
    }

    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, PortError> {
        let mut state = self.state.lock().unwrap();

        // Check if we should simulate a timeout
        if state.should_timeout {
            state.should_timeout = false;
            return Err(PortError::timeout(state.timeout));
        }

        // Read as many bytes as possible from the queue
        let mut bytes_read = 0;
        for byte in buffer.iter_mut() {
            if let Some(queued_byte) = state.read_queue.pop_front() {
                *byte = queued_byte;
                bytes_read += 1;
            } else {
                break;
            }
        }

        if bytes_read == 0 {
            // Simulate "would block" behavior by returning an I/O error
            Err(PortError::Io(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "No data available",
            )))
        } else {
            Ok(bytes_read)
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_timeout(&mut self, timeout: Duration) -> Result<(), PortError> {
        let mut state = self.state.lock().unwrap();
        state.timeout = timeout;
        Ok(())
    }

    fn clear_buffers(&mut self) -> Result<(), PortError> {
        let mut state = self.state.lock().unwrap();
        state.read_queue.clear();
        state.buffers_cleared = true;
        Ok(())
    }

    fn bytes_to_read(&self) -> Option<usize> {
        let state = self.state.lock().unwrap();
        Some(state.read_queue.len())
    }

    fn bytes_to_write(&self) -> Option<usize> {
        // For a mock port, there's no write buffer
        Some(0)
    }
}

impl std::fmt::Debug for MockSerialPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockSerialPort")
            .field("name", &self.name)
            .field("available_bytes", &self.available_bytes())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_and_read() {
        let mut port = MockSerialPort::new("MOCK0");
        port.enqueue_read(b"Hello");

        let mut buffer = [0u8; 10];
        let n = port.read_bytes(&mut buffer).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buffer[..n], b"Hello");
    }

    #[test]
    fn test_write_logging() {
        let mut port = MockSerialPort::new("MOCK0");
        port.write_bytes(b"Test1").unwrap();
        port.write_bytes(b"Test2").unwrap();

        let log = port.get_write_log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0], b"Test1");
        assert_eq!(log[1], b"Test2");
    }

    #[test]
    fn test_expect_write() {
        let mut port = MockSerialPort::new("MOCK0");
        port.expect_write(b"Expected");

        // Writing the expected data should succeed
        port.write_bytes(b"Expected").unwrap();

        // Verify all expectations were met
        assert!(port.verify_expectations().is_ok());
    }

    #[test]
    fn test_expect_write_mismatch() {
        let mut port = MockSerialPort::new("MOCK0");
        port.expect_write(b"Expected");

        // Writing different data should fail
        let result = port.write_bytes(b"Different");
        assert!(result.is_err());
    }

    #[test]
    fn test_timeout_simulation() {
        let mut port = MockSerialPort::new("MOCK0");
        port.set_should_timeout(true);

        let mut buffer = [0u8; 10];
        let result = port.read_bytes(&mut buffer);
        assert!(matches!(result, Err(PortError::Timeout(_))));
    }

    #[test]
    fn test_clear_buffers() {
        let mut port = MockSerialPort::new("MOCK0");
        port.enqueue_read(b"Should be cleared");

        port.clear_buffers().unwrap();
        assert!(port.was_cleared());
        assert_eq!(port.available_bytes(), 0);
    }

    #[test]
    fn test_empty_read() {
        let mut port = MockSerialPort::new("MOCK0");
        let mut buffer = [0u8; 10];

        // Reading when no data is available should return WouldBlock error
        let result = port.read_bytes(&mut buffer);
        assert!(result.is_err());
        if let Err(PortError::Io(e)) = result {
            assert_eq!(e.kind(), std::io::ErrorKind::WouldBlock);
        } else {
            panic!("Expected WouldBlock error");
        }
    }

    #[test]
    fn test_partial_read() {
        let mut port = MockSerialPort::new("MOCK0");
        port.enqueue_read(b"Hello, World!");

        // Read only first 5 bytes
        let mut buffer = [0u8; 5];
        let n = port.read_bytes(&mut buffer).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buffer[..n], b"Hello");

        // Remaining bytes should still be in queue
        assert_eq!(port.available_bytes(), 8);
    }

    #[test]
    fn test_set_timeout() {
        let mut port = MockSerialPort::new("MOCK0");
        let timeout = Duration::from_millis(500);

        port.set_timeout(timeout).unwrap();

        // Verify timeout is set by triggering a timeout error
        port.set_should_timeout(true);
        let mut buffer = [0u8; 10];
        let result = port.read_bytes(&mut buffer);

        if let Err(PortError::Timeout(d)) = result {
            assert_eq!(d, timeout);
        } else {
            panic!("Expected timeout error with duration {:?}", timeout);
        }
    }

    #[test]
    fn test_bytes_to_read() {
        let mut port = MockSerialPort::new("MOCK0");
        port.enqueue_read(b"Test data");

        assert_eq!(port.bytes_to_read(), Some(9));
    }
}
