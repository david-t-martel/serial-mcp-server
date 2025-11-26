//! Standard baud rate probing strategy.
//!
//! Sequentially tests common baud rates by attempting to open the port
//! and optionally send probe data to verify communication.

use super::{NegotiatedParams, NegotiationError, NegotiationHints, NegotiationStrategy};
use crate::port::{DataBits, FlowControl, Parity, PortConfiguration, StopBits};
use async_trait::async_trait;
use std::time::Duration;
use tracing::{debug, warn};

#[cfg(feature = "async-serial")]
use crate::port::{AsyncSerialPortAdapter, TokioSerialPort};

/// Common baud rates to test, ordered by popularity.
///
/// This list prioritizes the most commonly used rates first to minimize
/// detection time for typical devices.
pub const STANDARD_BAUD_RATES: &[u32] = &[
    9600,   // Most common default
    115200, // Modern devices, microcontrollers
    19200,  // Legacy devices
    38400,  // Medium speed devices
    57600,  // High speed legacy
    230400, // Very high speed
    460800, // Ultra high speed
    921600, // Maximum typical speed
    4800,   // Very slow legacy
    2400,   // Ancient devices
    1200,   // Historical
];

/// Strategy that tries standard baud rates sequentially.
pub struct StandardBaudsStrategy {
    /// Optional custom baud rates to try instead of defaults.
    custom_rates: Option<Vec<u32>>,

    /// Whether to send probe data and check for response.
    verify_with_probe: bool,
}

impl StandardBaudsStrategy {
    /// Create a new standard baud rate strategy with defaults.
    pub fn new() -> Self {
        Self {
            custom_rates: None,
            verify_with_probe: false,
        }
    }

    /// Create strategy with custom baud rates.
    pub fn with_custom_rates(rates: Vec<u32>) -> Self {
        Self {
            custom_rates: Some(rates),
            verify_with_probe: false,
        }
    }

    /// Enable probe verification (send test data and check for response).
    pub fn with_probe_verification(mut self) -> Self {
        self.verify_with_probe = true;
        self
    }

    /// Get the list of baud rates to try.
    fn get_baud_rates<'a>(&'a self, hints: &'a NegotiationHints) -> Vec<u32> {
        // Priority order:
        // 1. Suggested rates from hints (if not restricting)
        // 2. Custom rates from strategy
        // 3. Standard defaults

        let mut rates = Vec::new();

        // Add suggested rates first if available
        if !hints.suggested_baud_rates.is_empty() {
            rates.extend_from_slice(&hints.suggested_baud_rates);
        }

        // If hints restrict to suggested only, stop here
        if hints.restrict_to_suggested && !rates.is_empty() {
            return rates;
        }

        // Add custom or standard rates
        match &self.custom_rates {
            Some(custom) => {
                for &rate in custom {
                    if !rates.contains(&rate) {
                        rates.push(rate);
                    }
                }
            }
            None => {
                for &rate in STANDARD_BAUD_RATES {
                    if !rates.contains(&rate) {
                        rates.push(rate);
                    }
                }
            }
        }

        rates
    }

    #[cfg(feature = "async-serial")]
    async fn try_baud_rate(
        port_name: &str,
        baud_rate: u32,
        timeout: Duration,
        verify: bool,
    ) -> Result<Option<f32>, NegotiationError> {
        debug!("Probing {} at {} baud", port_name, baud_rate);

        let config = PortConfiguration {
            baud_rate,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            timeout,
        };

        // Try to open the port
        let mut port = match TokioSerialPort::open(port_name, &config) {
            Ok(p) => p,
            Err(e) => {
                debug!("Failed to open at {} baud: {}", baud_rate, e);
                return Ok(None);
            }
        };

        // If not verifying with probe, just opening successfully is enough
        if !verify {
            debug!(
                "Port opened successfully at {} baud (no verification)",
                baud_rate
            );
            return Ok(Some(0.3)); // Low confidence - just opened
        }

        // Try to send a newline and see if we get any response or error
        match port.write_bytes(b"\r\n").await {
            Ok(_) => {
                // Give device brief time to respond
                tokio::time::sleep(Duration::from_millis(50)).await;

                // Try to read any response
                let mut buffer = vec![0u8; 256];
                match tokio::time::timeout(Duration::from_millis(100), port.read_bytes(&mut buffer))
                    .await
                {
                    Ok(Ok(n)) if n > 0 => {
                        debug!("Got {} bytes response at {} baud", n, baud_rate);
                        Ok(Some(0.6)) // Medium confidence - got response
                    }
                    Ok(Ok(_)) => {
                        debug!("Write succeeded but no response at {} baud", baud_rate);
                        Ok(Some(0.4)) // Low-medium confidence
                    }
                    Ok(Err(e)) => {
                        warn!("Read error at {} baud: {}", baud_rate, e);
                        Ok(None)
                    }
                    Err(_) => {
                        debug!("No response within timeout at {} baud", baud_rate);
                        Ok(Some(0.3)) // Low confidence - timeout but no error
                    }
                }
            }
            Err(e) => {
                warn!("Write failed at {} baud: {}", baud_rate, e);
                Ok(None)
            }
        }
    }
}

impl Default for StandardBaudsStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NegotiationStrategy for StandardBaudsStrategy {
    fn name(&self) -> &'static str {
        "standard_bauds"
    }

    fn priority(&self) -> u8 {
        30 // Lower priority - brute force approach
    }

    #[cfg(feature = "async-serial")]
    async fn negotiate(
        &self,
        port_name: &str,
        hints: &NegotiationHints,
    ) -> Result<NegotiatedParams, NegotiationError> {
        let rates = self.get_baud_rates(hints);
        let timeout = hints.timeout();

        debug!("Trying {} baud rates for port {}", rates.len(), port_name);

        let mut best_result: Option<(u32, f32)> = None;

        for baud_rate in rates {
            match Self::try_baud_rate(port_name, baud_rate, timeout, self.verify_with_probe).await?
            {
                Some(confidence) => {
                    debug!("Baud rate {} has confidence {}", baud_rate, confidence);

                    // Keep track of best result
                    if best_result.is_none() || confidence > best_result.unwrap().1 {
                        best_result = Some((baud_rate, confidence));
                    }

                    // If we have high confidence, stop searching
                    if confidence >= 0.8 {
                        break;
                    }
                }
                None => continue,
            }
        }

        if let Some((baud_rate, confidence)) = best_result {
            Ok(NegotiatedParams::new(baud_rate, self.name()).with_confidence(confidence))
        } else {
            Err(NegotiationError::StrategyError {
                strategy: self.name().to_string(),
                message: "No baud rate worked".to_string(),
            })
        }
    }

    #[cfg(not(feature = "async-serial"))]
    async fn negotiate(
        &self,
        _port_name: &str,
        _hints: &NegotiationHints,
    ) -> Result<NegotiatedParams, NegotiationError> {
        Err(NegotiationError::StrategyError {
            strategy: self.name().to_string(),
            message: "async-serial feature required".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_baud_rates() {
        assert!(STANDARD_BAUD_RATES.contains(&9600));
        assert!(STANDARD_BAUD_RATES.contains(&115200));
        assert_eq!(STANDARD_BAUD_RATES[0], 9600); // Most common first
    }

    #[test]
    fn test_get_baud_rates_defaults() {
        let strategy = StandardBaudsStrategy::new();
        let hints = NegotiationHints::default();
        let rates = strategy.get_baud_rates(&hints);

        assert_eq!(rates.len(), STANDARD_BAUD_RATES.len());
        assert_eq!(rates[0], 9600);
    }

    #[test]
    fn test_get_baud_rates_with_suggestions() {
        let strategy = StandardBaudsStrategy::new();
        let hints = NegotiationHints::with_baud_rates(vec![57600, 115200]);
        let rates = strategy.get_baud_rates(&hints);

        // Should start with suggested rates
        assert_eq!(rates[0], 57600);
        assert_eq!(rates[1], 115200);
        // Then include standard rates
        assert!(rates.contains(&9600));
    }

    #[test]
    fn test_get_baud_rates_restricted() {
        let strategy = StandardBaudsStrategy::new();
        let mut hints = NegotiationHints::with_baud_rates(vec![57600, 115200]);
        hints.restrict_to_suggested = true;
        let rates = strategy.get_baud_rates(&hints);

        // Should only have suggested rates
        assert_eq!(rates.len(), 2);
        assert_eq!(rates[0], 57600);
        assert_eq!(rates[1], 115200);
    }

    #[test]
    fn test_custom_rates() {
        let strategy = StandardBaudsStrategy::with_custom_rates(vec![1200, 2400, 4800]);
        let hints = NegotiationHints::default();
        let rates = strategy.get_baud_rates(&hints);

        assert_eq!(rates[0], 1200);
        assert_eq!(rates[1], 2400);
        assert_eq!(rates[2], 4800);
    }

    #[test]
    fn test_strategy_priority() {
        let strategy = StandardBaudsStrategy::new();
        assert_eq!(strategy.priority(), 30);
    }

    #[test]
    fn test_with_probe_verification() {
        let strategy = StandardBaudsStrategy::new().with_probe_verification();
        assert!(strategy.verify_with_probe);
    }
}
