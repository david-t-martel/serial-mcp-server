//! Negotiation strategies for automatic port configuration detection.
//!
//! This module defines the core traits and types used by all negotiation strategies,
//! as well as specific implementations for different detection methods.

use crate::port::{DataBits, FlowControl, Parity, PortError, StopBits};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

pub mod echo_probe;
pub mod manufacturer;
pub mod standard_bauds;

// Re-export strategy implementations
pub use echo_probe::EchoProbeStrategy;
pub use manufacturer::ManufacturerStrategy;
pub use standard_bauds::StandardBaudsStrategy;

/// Errors that can occur during port negotiation.
#[derive(Debug, Error)]
pub enum NegotiationError {
    /// The specified port was not found on the system.
    #[error("Port not found: {0}")]
    PortNotFound(String),

    /// All negotiation strategies failed to establish communication.
    #[error("All strategies failed")]
    AllStrategiesFailed,

    /// Negotiation timed out.
    #[error("Timeout during negotiation")]
    Timeout,

    /// A port-level error occurred.
    #[error("Port error: {0}")]
    PortError(#[from] PortError),

    /// Invalid configuration was detected or provided.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Strategy-specific error.
    #[error("Strategy error ({strategy}): {message}")]
    StrategyError { strategy: String, message: String },
}

/// Hints to guide the negotiation process.
///
/// These optional parameters can improve negotiation speed and accuracy
/// by providing context about the device.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NegotiationHints {
    /// USB Vendor ID (if available from device enumeration).
    pub vid: Option<u16>,

    /// USB Product ID (if available from device enumeration).
    pub pid: Option<u16>,

    /// Manufacturer name (if available from device enumeration).
    pub manufacturer: Option<String>,

    /// Suggested baud rates to try (ordered by likelihood).
    pub suggested_baud_rates: Vec<u32>,

    /// Maximum time to spend per strategy attempt (milliseconds).
    pub timeout_ms: u64,

    /// Whether to try only suggested baud rates (skip standard set).
    pub restrict_to_suggested: bool,
}

impl NegotiationHints {
    /// Create hints with only a vendor ID.
    pub fn with_vid(vid: u16) -> Self {
        Self {
            vid: Some(vid),
            ..Default::default()
        }
    }

    /// Create hints with vendor and product IDs.
    pub fn with_vid_pid(vid: u16, pid: u16) -> Self {
        Self {
            vid: Some(vid),
            pid: Some(pid),
            ..Default::default()
        }
    }

    /// Create hints with suggested baud rates.
    pub fn with_baud_rates(rates: Vec<u32>) -> Self {
        Self {
            suggested_baud_rates: rates,
            ..Default::default()
        }
    }

    /// Set the timeout for negotiation attempts.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Get the timeout as a Duration.
    pub fn timeout(&self) -> Duration {
        if self.timeout_ms > 0 {
            Duration::from_millis(self.timeout_ms)
        } else {
            Duration::from_millis(500) // Default 500ms
        }
    }
}

/// Parameters successfully negotiated for a serial port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiatedParams {
    /// Negotiated baud rate.
    pub baud_rate: u32,

    /// Number of data bits (typically 8).
    pub data_bits: DataBits,

    /// Parity checking mode (typically None).
    pub parity: Parity,

    /// Number of stop bits (typically 1).
    pub stop_bits: StopBits,

    /// Flow control mode (typically None).
    pub flow_control: FlowControl,

    /// Name of the strategy that successfully negotiated these parameters.
    pub strategy_used: String,

    /// Confidence level (0.0 - 1.0) in the negotiated parameters.
    /// 1.0 = high confidence (e.g., got expected response)
    /// 0.5 = medium confidence (e.g., got some response)
    /// 0.1 = low confidence (e.g., no errors but no confirmation)
    pub confidence: f32,
}

impl NegotiatedParams {
    /// Create negotiated parameters with high confidence.
    pub fn new(baud_rate: u32, strategy_used: impl Into<String>) -> Self {
        Self {
            baud_rate,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            strategy_used: strategy_used.into(),
            confidence: 1.0,
        }
    }

    /// Create parameters with medium confidence.
    pub fn with_medium_confidence(baud_rate: u32, strategy_used: impl Into<String>) -> Self {
        Self {
            baud_rate,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            strategy_used: strategy_used.into(),
            confidence: 0.5,
        }
    }

    /// Set the confidence level.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set custom serial parameters.
    pub fn with_params(
        mut self,
        data_bits: DataBits,
        parity: Parity,
        stop_bits: StopBits,
        flow_control: FlowControl,
    ) -> Self {
        self.data_bits = data_bits;
        self.parity = parity;
        self.stop_bits = stop_bits;
        self.flow_control = flow_control;
        self
    }
}

/// Trait for port negotiation strategies.
///
/// Each strategy implements a different method for detecting the correct
/// serial port parameters (primarily baud rate).
#[async_trait]
pub trait NegotiationStrategy: Send + Sync {
    /// Get the name of this strategy (for logging and debugging).
    fn name(&self) -> &'static str;

    /// Attempt to negotiate port parameters.
    ///
    /// # Arguments
    /// * `port_name` - The system path to the serial port (e.g., "/dev/ttyUSB0", "COM3")
    /// * `hints` - Optional hints to guide the negotiation process
    ///
    /// # Returns
    /// Successfully negotiated parameters, or an error if negotiation failed.
    async fn negotiate(
        &self,
        port_name: &str,
        hints: &NegotiationHints,
    ) -> Result<NegotiatedParams, NegotiationError>;

    /// Get the priority of this strategy (higher = tried first).
    ///
    /// Default priority is 50. Manufacturer-based strategies typically have
    /// higher priority (70-80), while brute-force strategies have lower (20-30).
    fn priority(&self) -> u8 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negotiation_hints_default() {
        let hints = NegotiationHints::default();
        assert!(hints.vid.is_none());
        assert!(hints.pid.is_none());
        assert!(hints.suggested_baud_rates.is_empty());
        assert_eq!(hints.timeout(), Duration::from_millis(500));
    }

    #[test]
    fn test_negotiation_hints_with_vid() {
        let hints = NegotiationHints::with_vid(0x0403);
        assert_eq!(hints.vid, Some(0x0403));
    }

    #[test]
    fn test_negotiation_hints_timeout() {
        let hints = NegotiationHints::default().with_timeout_ms(1000);
        assert_eq!(hints.timeout(), Duration::from_millis(1000));
    }

    #[test]
    fn test_negotiated_params_new() {
        let params = NegotiatedParams::new(115200, "test");
        assert_eq!(params.baud_rate, 115200);
        assert_eq!(params.strategy_used, "test");
        assert_eq!(params.confidence, 1.0);
        assert_eq!(params.data_bits, DataBits::Eight);
    }

    #[test]
    fn test_negotiated_params_confidence_clamping() {
        let params = NegotiatedParams::new(9600, "test").with_confidence(1.5);
        assert_eq!(params.confidence, 1.0);

        let params = NegotiatedParams::new(9600, "test").with_confidence(-0.5);
        assert_eq!(params.confidence, 0.0);
    }
}
