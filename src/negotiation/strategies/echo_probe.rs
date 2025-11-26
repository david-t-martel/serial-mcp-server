//! Echo probe negotiation strategy.
//!
//! Sends probe commands (e.g., AT commands) and checks for expected responses
//! to validate the correct baud rate and communication parameters.

use super::{NegotiatedParams, NegotiationError, NegotiationHints, NegotiationStrategy};
use crate::port::{DataBits, FlowControl, Parity, PortConfiguration, StopBits};
use async_trait::async_trait;
use std::time::Duration;
use tracing::{debug, warn};

#[cfg(feature = "async-serial")]
use crate::port::{AsyncSerialPortAdapter, TokioSerialPort};

/// Probe sequence definition.
#[derive(Debug, Clone)]
pub struct ProbeSequence {
    /// The command to send.
    pub command: Vec<u8>,

    /// Expected response patterns (any match is success).
    pub expected_responses: Vec<Vec<u8>>,

    /// Human-readable description.
    pub description: &'static str,
}

impl ProbeSequence {
    /// Create a new probe sequence.
    pub fn new(
        command: impl Into<Vec<u8>>,
        expected_responses: Vec<Vec<u8>>,
        description: &'static str,
    ) -> Self {
        Self {
            command: command.into(),
            expected_responses,
            description,
        }
    }

    /// Check if response matches any expected pattern.
    pub fn matches(&self, response: &[u8]) -> bool {
        self.expected_responses.iter().any(|expected| {
            if response.len() < expected.len() {
                return false;
            }
            response
                .windows(expected.len())
                .any(|window| window == expected.as_slice())
        })
    }
}

/// Common probe sequences for different device types.
pub struct CommonProbes;

impl CommonProbes {
    /// AT command probe (for modems, GPS modules, etc.)
    pub fn at_command() -> ProbeSequence {
        ProbeSequence::new(
            b"AT\r\n".to_vec(),
            vec![b"OK".to_vec(), b"ok".to_vec(), b"AT".to_vec()],
            "AT command",
        )
    }

    /// Simple echo probe (send newline, expect some response)
    pub fn newline_echo() -> ProbeSequence {
        ProbeSequence::new(
            b"\r\n".to_vec(),
            vec![
                b"\r\n".to_vec(),
                b">".to_vec(),
                b"$".to_vec(),
                b"#".to_vec(),
            ],
            "Newline echo",
        )
    }

    /// Hayes command set probe
    pub fn hayes_modem() -> ProbeSequence {
        ProbeSequence::new(
            b"ATI\r\n".to_vec(),
            vec![b"OK".to_vec(), b"Modem".to_vec(), b"Hayes".to_vec()],
            "Hayes modem",
        )
    }

    /// GPS NMEA probe
    pub fn nmea_gps() -> ProbeSequence {
        ProbeSequence::new(
            b"\r\n".to_vec(),
            vec![b"$GP".to_vec(), b"$GN".to_vec(), b"$GL".to_vec()],
            "NMEA GPS",
        )
    }
}

/// Baud rates commonly used by devices that respond to AT commands.
const AT_COMMAND_BAUD_RATES: &[u32] = &[9600, 115200, 19200, 38400, 57600, 4800, 2400];

/// Strategy that uses echo probes to detect the correct baud rate.
pub struct EchoProbeStrategy {
    /// Probe sequences to try.
    probe_sequences: Vec<ProbeSequence>,

    /// Baud rates to test.
    baud_rates: Vec<u32>,
}

impl EchoProbeStrategy {
    /// Create a new echo probe strategy with default AT command probes.
    pub fn new() -> Self {
        Self {
            probe_sequences: vec![CommonProbes::at_command(), CommonProbes::newline_echo()],
            baud_rates: AT_COMMAND_BAUD_RATES.to_vec(),
        }
    }

    /// Create strategy with custom probe sequences.
    pub fn with_probes(probe_sequences: Vec<ProbeSequence>) -> Self {
        Self {
            probe_sequences,
            baud_rates: AT_COMMAND_BAUD_RATES.to_vec(),
        }
    }

    /// Set custom baud rates to test.
    pub fn with_baud_rates(mut self, baud_rates: Vec<u32>) -> Self {
        self.baud_rates = baud_rates;
        self
    }

    /// Add a probe sequence to the existing set.
    pub fn add_probe(mut self, probe: ProbeSequence) -> Self {
        self.probe_sequences.push(probe);
        self
    }

    #[cfg(feature = "async-serial")]
    async fn try_probe_at_baud(
        port_name: &str,
        baud_rate: u32,
        probe: &ProbeSequence,
        timeout: Duration,
    ) -> Result<Option<f32>, NegotiationError> {
        debug!(
            "Trying {} probe at {} baud on {}",
            probe.description, baud_rate, port_name
        );

        let config = PortConfiguration {
            baud_rate,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            timeout,
        };

        let mut port = match TokioSerialPort::open(port_name, &config) {
            Ok(p) => p,
            Err(e) => {
                debug!("Failed to open at {} baud: {}", baud_rate, e);
                return Ok(None);
            }
        };

        // Send the probe command
        if let Err(e) = port.write_bytes(&probe.command).await {
            warn!("Failed to send probe at {} baud: {}", baud_rate, e);
            return Ok(None);
        }

        // Brief delay to let device process
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Read response
        let mut buffer = vec![0u8; 1024];
        match tokio::time::timeout(timeout, port.read_bytes(&mut buffer)).await {
            Ok(Ok(n)) if n > 0 => {
                let response = &buffer[..n];
                debug!(
                    "Got {} bytes response: {:?}",
                    n,
                    String::from_utf8_lossy(response)
                );

                if probe.matches(response) {
                    debug!(
                        "Probe '{}' matched at {} baud!",
                        probe.description, baud_rate
                    );
                    Ok(Some(0.95)) // Very high confidence - expected response
                } else {
                    debug!(
                        "Got response but no match for probe '{}'",
                        probe.description
                    );
                    Ok(Some(0.4)) // Some confidence - got response but wrong pattern
                }
            }
            Ok(Ok(_)) => {
                debug!("No response to probe at {} baud", baud_rate);
                Ok(None)
            }
            Ok(Err(e)) => {
                warn!("Read error at {} baud: {}", baud_rate, e);
                Ok(None)
            }
            Err(_) => {
                debug!("Timeout waiting for response at {} baud", baud_rate);
                Ok(None)
            }
        }
    }
}

impl Default for EchoProbeStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NegotiationStrategy for EchoProbeStrategy {
    fn name(&self) -> &'static str {
        "echo_probe"
    }

    fn priority(&self) -> u8 {
        60 // Medium-high priority - works well for interactive devices
    }

    #[cfg(feature = "async-serial")]
    async fn negotiate(
        &self,
        port_name: &str,
        hints: &NegotiationHints,
    ) -> Result<NegotiatedParams, NegotiationError> {
        let timeout = hints.timeout();

        // Use suggested baud rates if provided, otherwise use defaults
        let baud_rates = if !hints.suggested_baud_rates.is_empty() {
            &hints.suggested_baud_rates
        } else {
            &self.baud_rates
        };

        debug!(
            "Echo probe strategy testing {} baud rates with {} probes",
            baud_rates.len(),
            self.probe_sequences.len()
        );

        let mut best_result: Option<(u32, f32, String)> = None;

        // Try each baud rate with each probe
        for &baud_rate in baud_rates {
            for probe in &self.probe_sequences {
                match Self::try_probe_at_baud(port_name, baud_rate, probe, timeout).await? {
                    Some(confidence) => {
                        debug!(
                            "Baud {} with probe '{}' has confidence {}",
                            baud_rate, probe.description, confidence
                        );

                        // Update best result if this is better
                        if best_result.is_none() || confidence > best_result.as_ref().unwrap().1 {
                            best_result =
                                Some((baud_rate, confidence, probe.description.to_string()));
                        }

                        // If we have very high confidence, we can stop
                        if confidence >= 0.9 {
                            return Ok(NegotiatedParams::new(baud_rate, self.name())
                                .with_confidence(confidence));
                        }
                    }
                    None => continue,
                }
            }
        }

        if let Some((baud_rate, confidence, probe_desc)) = best_result {
            debug!(
                "Best result: {} baud (probe: {}, confidence: {})",
                baud_rate, probe_desc, confidence
            );
            Ok(NegotiatedParams::new(baud_rate, self.name()).with_confidence(confidence))
        } else {
            Err(NegotiationError::StrategyError {
                strategy: self.name().to_string(),
                message: "No probe received valid response".to_string(),
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
    fn test_probe_sequence_matches() {
        let probe = CommonProbes::at_command();
        assert!(probe.matches(b"OK\r\n"));
        assert!(probe.matches(b"AT\r\nOK\r\n"));
        assert!(probe.matches(b"Some data OK here"));
        assert!(!probe.matches(b"ERROR"));
    }

    #[test]
    fn test_common_probes() {
        let at = CommonProbes::at_command();
        assert_eq!(at.description, "AT command");
        assert_eq!(at.command, b"AT\r\n");

        let newline = CommonProbes::newline_echo();
        assert_eq!(newline.description, "Newline echo");
        assert_eq!(newline.command, b"\r\n");

        let hayes = CommonProbes::hayes_modem();
        assert_eq!(hayes.description, "Hayes modem");

        let nmea = CommonProbes::nmea_gps();
        assert_eq!(nmea.description, "NMEA GPS");
    }

    #[test]
    fn test_strategy_priority() {
        let strategy = EchoProbeStrategy::new();
        assert_eq!(strategy.priority(), 60);
    }

    #[test]
    fn test_with_custom_probes() {
        let custom_probe =
            ProbeSequence::new(b"TEST\r\n".to_vec(), vec![b"ACK".to_vec()], "Custom test");
        let strategy = EchoProbeStrategy::with_probes(vec![custom_probe]);
        assert_eq!(strategy.probe_sequences.len(), 1);
    }

    #[test]
    fn test_add_probe() {
        let custom_probe = ProbeSequence::new(
            b"HELLO\r\n".to_vec(),
            vec![b"WORLD".to_vec()],
            "Hello world",
        );
        let strategy = EchoProbeStrategy::new().add_probe(custom_probe);
        assert_eq!(strategy.probe_sequences.len(), 3); // 2 default + 1 custom
    }

    #[test]
    fn test_with_baud_rates() {
        let strategy = EchoProbeStrategy::new().with_baud_rates(vec![9600, 19200]);
        assert_eq!(strategy.baud_rates.len(), 2);
        assert_eq!(strategy.baud_rates[0], 9600);
    }

    #[test]
    fn test_probe_matches_partial() {
        let probe = ProbeSequence::new(b"CMD".to_vec(), vec![b"OK".to_vec()], "Test");

        // Should match "OK" anywhere in the response
        assert!(probe.matches(b"OK"));
        assert!(probe.matches(b"Response: OK\r\n"));
        assert!(probe.matches(b"CMD OK DONE"));
        assert!(!probe.matches(b"ERROR"));
        assert!(!probe.matches(b"O")); // Too short
    }
}
