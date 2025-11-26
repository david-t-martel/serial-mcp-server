//! Port abstraction layer for serial communication.
//!
//! Provides traits and implementations for both sync and async serial I/O,
//! enabling dependency injection and testing via mocks.

pub mod error;
pub mod mock;
pub mod sync_port;
pub mod traits;

// Phase 3: Async serial port support
#[cfg(feature = "async-serial")]
pub mod async_port;

pub use error::PortError;
pub use mock::MockSerialPort;
pub use sync_port::*;
pub use traits::*;

// Re-export async types when the feature is enabled
#[cfg(feature = "async-serial")]
pub use async_port::{AsyncSerialPortAdapter, BlockingSerialPortWrapper, TokioSerialPort};
