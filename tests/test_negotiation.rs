//! Integration tests for the auto-negotiation module.

#![cfg(feature = "auto-negotiation")]

use serial_mcp_agent::negotiation::{
    AutoNegotiator, NegotiatedParams, NegotiationHints, NegotiationStrategy,
};
use serial_mcp_agent::port::{DataBits, FlowControl, Parity, StopBits};

#[test]
fn test_negotiation_hints_builder() {
    let hints = NegotiationHints::with_vid(0x0403).with_timeout_ms(1000);

    assert_eq!(hints.vid, Some(0x0403));
    assert_eq!(hints.timeout_ms, 1000);
}

#[test]
fn test_negotiation_hints_with_baud_rates() {
    let hints = NegotiationHints::with_baud_rates(vec![9600, 115200]);

    assert_eq!(hints.suggested_baud_rates.len(), 2);
    assert_eq!(hints.suggested_baud_rates[0], 9600);
    assert_eq!(hints.suggested_baud_rates[1], 115200);
}

#[test]
fn test_negotiated_params_builder() {
    let params = NegotiatedParams::new(115200, "test_strategy")
        .with_confidence(0.95)
        .with_params(
            DataBits::Eight,
            Parity::None,
            StopBits::One,
            FlowControl::None,
        );

    assert_eq!(params.baud_rate, 115200);
    assert_eq!(params.strategy_used, "test_strategy");
    assert_eq!(params.confidence, 0.95);
    assert_eq!(params.data_bits, DataBits::Eight);
    assert_eq!(params.parity, Parity::None);
}

#[test]
fn test_negotiated_params_confidence_clamping() {
    let params = NegotiatedParams::new(9600, "test").with_confidence(1.5);
    assert_eq!(params.confidence, 1.0);

    let params = NegotiatedParams::new(9600, "test").with_confidence(-0.5);
    assert_eq!(params.confidence, 0.0);
}

#[test]
fn test_auto_negotiator_creation() {
    let negotiator = AutoNegotiator::new();
    let strategies = negotiator.strategies();

    // Should have 3 default strategies
    assert_eq!(strategies.len(), 3);

    // Verify strategies are sorted by priority
    assert!(strategies[0].priority() >= strategies[1].priority());
    assert!(strategies[1].priority() >= strategies[2].priority());
}

#[test]
fn test_auto_negotiator_strategy_order() {
    let negotiator = AutoNegotiator::new();
    let strategies = negotiator.strategies();

    // Manufacturer should be first (highest priority)
    assert_eq!(strategies[0].name(), "manufacturer");
    assert_eq!(strategies[0].priority(), 80);

    // Echo probe should be second
    assert_eq!(strategies[1].name(), "echo_probe");
    assert_eq!(strategies[1].priority(), 60);

    // Standard bauds should be last
    assert_eq!(strategies[2].name(), "standard_bauds");
    assert_eq!(strategies[2].priority(), 30);
}

#[test]
fn test_manufacturer_profile_lookup() {
    // Test FTDI lookup
    let profile = AutoNegotiator::get_manufacturer_profile(0x0403);
    assert!(profile.is_some());
    let profile = profile.unwrap();
    assert_eq!(profile.name, "FTDI");
    assert_eq!(profile.default_baud, 115200);
    assert!(profile.common_bauds.contains(&9600));
    assert!(profile.common_bauds.contains(&115200));

    // Test Arduino lookup
    let profile = AutoNegotiator::get_manufacturer_profile(0x2341);
    assert!(profile.is_some());
    let profile = profile.unwrap();
    assert_eq!(profile.name, "Arduino");
    assert_eq!(profile.default_baud, 9600);

    // Test unknown VID
    let profile = AutoNegotiator::get_manufacturer_profile(0xFFFF);
    assert!(profile.is_none());
}

#[test]
fn test_all_manufacturer_profiles() {
    let profiles = AutoNegotiator::all_manufacturer_profiles();

    // Should have multiple profiles
    assert!(profiles.len() >= 6);

    // Verify some well-known manufacturers are present
    assert!(profiles.iter().any(|p| p.name == "FTDI"));
    assert!(profiles.iter().any(|p| p.name == "Arduino"));
    assert!(profiles.iter().any(|p| p.name == "Silicon Labs CP210x"));
    assert!(profiles.iter().any(|p| p.name == "Raspberry Pi Pico"));

    // All profiles should have non-empty baud rate lists
    for profile in profiles {
        assert!(!profile.common_bauds.is_empty());
        assert!(profile.default_baud > 0);
        assert!(profile.vid > 0);
    }
}

#[test]
fn test_negotiation_hints_timeout_default() {
    let hints = NegotiationHints::default();
    assert_eq!(hints.timeout().as_millis(), 500);
}

#[test]
fn test_negotiation_hints_timeout_custom() {
    let hints = NegotiationHints::default().with_timeout_ms(2000);
    assert_eq!(hints.timeout().as_millis(), 2000);
}

#[test]
fn test_medium_confidence_params() {
    let params = NegotiatedParams::with_medium_confidence(38400, "fallback");
    assert_eq!(params.baud_rate, 38400);
    assert_eq!(params.confidence, 0.5);
    assert_eq!(params.strategy_used, "fallback");
}

#[test]
fn test_negotiation_hints_restrict_to_suggested() {
    let mut hints = NegotiationHints::with_baud_rates(vec![9600, 115200]);
    hints.restrict_to_suggested = true;

    assert!(hints.restrict_to_suggested);
    assert_eq!(hints.suggested_baud_rates.len(), 2);
}

#[cfg(feature = "async-serial")]
#[tokio::test]
async fn test_auto_negotiator_with_invalid_port() {
    use serial_mcp_agent::negotiation::NegotiationError;

    let negotiator = AutoNegotiator::new();
    let result = negotiator.detect("INVALID_PORT_XYZ", None).await;

    // Should fail with AllStrategiesFailed since no strategy can open invalid port
    assert!(result.is_err());
    match result {
        Err(NegotiationError::AllStrategiesFailed) => {
            // Expected
        }
        Err(e) => panic!("Expected AllStrategiesFailed, got: {}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[test]
fn test_custom_strategy_priority() {
    use serial_mcp_agent::negotiation::strategies::StandardBaudsStrategy;

    let strategy = StandardBaudsStrategy::new();
    assert_eq!(strategy.priority(), 30);
}

#[test]
fn test_standard_baud_rates_list() {
    use serial_mcp_agent::negotiation::strategies::standard_bauds::STANDARD_BAUD_RATES;

    // Should start with most common rate
    assert_eq!(STANDARD_BAUD_RATES[0], 9600);

    // Should contain common rates
    assert!(STANDARD_BAUD_RATES.contains(&9600));
    assert!(STANDARD_BAUD_RATES.contains(&115200));
    assert!(STANDARD_BAUD_RATES.contains(&19200));
    assert!(STANDARD_BAUD_RATES.contains(&38400));
    assert!(STANDARD_BAUD_RATES.contains(&57600));
}

#[test]
fn test_echo_probe_sequences() {
    use serial_mcp_agent::negotiation::strategies::echo_probe::CommonProbes;

    let at_probe = CommonProbes::at_command();
    assert_eq!(at_probe.description, "AT command");
    assert_eq!(at_probe.command, b"AT\r\n");
    assert!(at_probe.expected_responses.contains(&b"OK".to_vec()));

    let newline_probe = CommonProbes::newline_echo();
    assert_eq!(newline_probe.description, "Newline echo");
    assert_eq!(newline_probe.command, b"\r\n");

    let hayes_probe = CommonProbes::hayes_modem();
    assert_eq!(hayes_probe.description, "Hayes modem");

    let nmea_probe = CommonProbes::nmea_gps();
    assert_eq!(nmea_probe.description, "NMEA GPS");
}

#[test]
fn test_probe_sequence_matching() {
    use serial_mcp_agent::negotiation::strategies::echo_probe::ProbeSequence;

    let probe = ProbeSequence::new(
        b"TEST\r\n".to_vec(),
        vec![b"OK".to_vec(), b"ACK".to_vec()],
        "Test probe",
    );

    // Should match "OK" in response
    assert!(probe.matches(b"Response: OK\r\n"));
    assert!(probe.matches(b"OK"));

    // Should match "ACK" in response
    assert!(probe.matches(b"ACK\r\n"));
    assert!(probe.matches(b"Device ACK received"));

    // Should not match other responses
    assert!(!probe.matches(b"ERROR"));
    assert!(!probe.matches(b"FAIL"));
}
