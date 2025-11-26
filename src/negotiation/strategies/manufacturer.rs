//! Manufacturer-based negotiation strategy.
//!
//! Uses USB Vendor ID (VID) and Product ID (PID) to determine likely
//! baud rates based on known manufacturer profiles.

use super::{NegotiatedParams, NegotiationError, NegotiationHints, NegotiationStrategy};
use crate::port::{DataBits, FlowControl, Parity, PortConfiguration, StopBits};
use async_trait::async_trait;
use std::time::Duration;
use tracing::{debug, warn};

#[cfg(feature = "async-serial")]
use crate::port::{AsyncSerialPortAdapter, TokioSerialPort};

/// Known manufacturer profile with default communication parameters.
#[derive(Debug, Clone, Copy)]
pub struct ManufacturerProfile {
    /// USB Vendor ID.
    pub vid: u16,

    /// Human-readable manufacturer name.
    pub name: &'static str,

    /// Default/most common baud rate for this manufacturer.
    pub default_baud: u32,

    /// Common baud rates to try (ordered by likelihood).
    pub common_bauds: &'static [u32],
}

/// Database of known manufacturer profiles.
///
/// This list includes common USB-to-serial chips and development boards.
pub const MANUFACTURER_PROFILES: &[ManufacturerProfile] = &[
    ManufacturerProfile {
        vid: 0x0403,
        name: "FTDI",
        default_baud: 115200,
        common_bauds: &[9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600],
    },
    ManufacturerProfile {
        vid: 0x10C4,
        name: "Silicon Labs CP210x",
        default_baud: 9600,
        common_bauds: &[9600, 19200, 38400, 57600, 115200],
    },
    ManufacturerProfile {
        vid: 0x1A86,
        name: "WCH CH340/CH341",
        default_baud: 9600,
        common_bauds: &[9600, 19200, 57600, 115200],
    },
    ManufacturerProfile {
        vid: 0x2341,
        name: "Arduino",
        default_baud: 9600,
        common_bauds: &[9600, 57600, 115200],
    },
    ManufacturerProfile {
        vid: 0x239A,
        name: "Adafruit",
        default_baud: 115200,
        common_bauds: &[9600, 115200],
    },
    ManufacturerProfile {
        vid: 0x2E8A,
        name: "Raspberry Pi Pico",
        default_baud: 115200,
        common_bauds: &[9600, 115200],
    },
    ManufacturerProfile {
        vid: 0x067B,
        name: "Prolific PL2303",
        default_baud: 9600,
        common_bauds: &[9600, 19200, 38400, 57600, 115200],
    },
    ManufacturerProfile {
        vid: 0x0483,
        name: "STMicroelectronics",
        default_baud: 115200,
        common_bauds: &[9600, 38400, 115200],
    },
];

/// Strategy that uses manufacturer VID/PID to determine likely baud rates.
pub struct ManufacturerStrategy;

impl ManufacturerStrategy {
    /// Create a new manufacturer-based strategy.
    pub fn new() -> Self {
        Self
    }

    /// Look up a manufacturer profile by VID.
    pub fn get_profile(vid: u16) -> Option<&'static ManufacturerProfile> {
        MANUFACTURER_PROFILES.iter().find(|p| p.vid == vid)
    }

    /// Get all known manufacturer profiles.
    pub fn all_profiles() -> &'static [ManufacturerProfile] {
        MANUFACTURER_PROFILES
    }

    #[cfg(feature = "async-serial")]
    async fn try_baud_rate(
        port_name: &str,
        baud_rate: u32,
        timeout: Duration,
    ) -> Result<bool, NegotiationError> {
        debug!("Trying baud rate {} on {}", baud_rate, port_name);

        let config = PortConfiguration {
            baud_rate,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
            timeout,
        };

        // Try to open the port with this configuration
        match TokioSerialPort::open(port_name, &config) {
            Ok(mut port) => {
                // Successfully opened - this is a valid configuration
                // For manufacturer strategy, we trust the profile
                debug!("Successfully opened port at {} baud", baud_rate);

                // Try a simple write/flush to verify the port works
                if let Err(e) = port.write_bytes(b"\r\n").await {
                    warn!("Port opened but write failed: {}", e);
                    return Ok(false);
                }

                Ok(true)
            }
            Err(e) => {
                debug!("Failed to open at {} baud: {}", baud_rate, e);
                Ok(false)
            }
        }
    }
}

impl Default for ManufacturerStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NegotiationStrategy for ManufacturerStrategy {
    fn name(&self) -> &'static str {
        "manufacturer"
    }

    fn priority(&self) -> u8 {
        80 // High priority - manufacturer profiles are reliable
    }

    #[cfg(feature = "async-serial")]
    async fn negotiate(
        &self,
        port_name: &str,
        hints: &NegotiationHints,
    ) -> Result<NegotiatedParams, NegotiationError> {
        let vid = hints.vid.ok_or_else(|| NegotiationError::StrategyError {
            strategy: self.name().to_string(),
            message: "No VID provided in hints".to_string(),
        })?;

        let profile = Self::get_profile(vid).ok_or_else(|| NegotiationError::StrategyError {
            strategy: self.name().to_string(),
            message: format!("Unknown VID: 0x{:04X}", vid),
        })?;

        debug!(
            "Using manufacturer profile: {} (VID: 0x{:04X})",
            profile.name, profile.vid
        );

        let timeout = hints.timeout();

        // Try the default baud rate first
        if Self::try_baud_rate(port_name, profile.default_baud, timeout).await? {
            return Ok(
                NegotiatedParams::new(profile.default_baud, self.name()).with_confidence(0.9)
            ); // High confidence for manufacturer default
        }

        // Try other common baud rates
        for &baud_rate in profile.common_bauds {
            if baud_rate == profile.default_baud {
                continue; // Already tried
            }

            if Self::try_baud_rate(port_name, baud_rate, timeout).await? {
                return Ok(NegotiatedParams::new(baud_rate, self.name()).with_confidence(0.7));
                // Good confidence for manufacturer profile
            }
        }

        Err(NegotiationError::StrategyError {
            strategy: self.name().to_string(),
            message: format!("None of the common baud rates for {} worked", profile.name),
        })
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
    fn test_get_profile_ftdi() {
        let profile = ManufacturerStrategy::get_profile(0x0403).unwrap();
        assert_eq!(profile.name, "FTDI");
        assert_eq!(profile.default_baud, 115200);
        assert!(profile.common_bauds.contains(&9600));
        assert!(profile.common_bauds.contains(&115200));
    }

    #[test]
    fn test_get_profile_arduino() {
        let profile = ManufacturerStrategy::get_profile(0x2341).unwrap();
        assert_eq!(profile.name, "Arduino");
        assert_eq!(profile.default_baud, 9600);
    }

    #[test]
    fn test_get_profile_unknown() {
        let profile = ManufacturerStrategy::get_profile(0xFFFF);
        assert!(profile.is_none());
    }

    #[test]
    fn test_all_profiles() {
        let profiles = ManufacturerStrategy::all_profiles();
        assert!(!profiles.is_empty());
        assert!(profiles.iter().any(|p| p.name == "FTDI"));
        assert!(profiles.iter().any(|p| p.name == "Arduino"));
    }

    #[test]
    fn test_strategy_priority() {
        let strategy = ManufacturerStrategy::new();
        assert_eq!(strategy.priority(), 80);
    }
}
