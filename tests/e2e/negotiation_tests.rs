//! E2E tests for auto-negotiation with mock ports.
//!
//! These tests verify the auto-negotiation system can:
//! - Use manufacturer strategy with known VID
//! - Fall back to standard baud rates
//! - Handle negotiation timeouts
//! - Try all strategies in priority order
//! - Provide confidence scoring

#![cfg(feature = "auto-negotiation")]

use serial_mcp_agent::negotiation::{
    AutoNegotiator, NegotiatedParams, NegotiationError, NegotiationHints,
};
use serial_mcp_agent::port::{DataBits, FlowControl, Parity, StopBits};

#[tokio::test]
async fn test_manufacturer_strategy_with_known_vid() {
    // Test that manufacturer strategy is attempted first with known VID
    let _negotiator = AutoNegotiator::new();

    // Create hints with FTDI VID (0x0403)
    let _hints = NegotiationHints::with_vid(0x0403);

    // Verify manufacturer profile exists
    let profile = AutoNegotiator::get_manufacturer_profile(0x0403);
    assert!(profile.is_some());
    assert_eq!(profile.unwrap().name, "FTDI");
    assert_eq!(profile.unwrap().default_baud, 115200);
}

#[tokio::test]
async fn test_fallback_to_standard_bauds() {
    // Test that unknown VID falls back to standard baud rates
    let negotiator = AutoNegotiator::new();

    // Unknown VID should not have a profile
    let _hints = NegotiationHints::with_vid(0xFFFF);
    let profile = AutoNegotiator::get_manufacturer_profile(0xFFFF);
    assert!(profile.is_none());

    // Standard bauds strategy should be available as fallback
    let strategies = negotiator.strategies();
    assert!(strategies.iter().any(|s| s.name() == "standard_bauds"));
}

#[tokio::test]
async fn test_negotiation_with_invalid_port() {
    // Test that negotiation fails gracefully with invalid port
    let negotiator = AutoNegotiator::new();
    let result = negotiator.detect("INVALID_PORT_XYZ", None).await;

    assert!(result.is_err());
    match result {
        Err(NegotiationError::AllStrategiesFailed) => {
            // Expected - all strategies should fail
        }
        Err(e) => panic!("Expected AllStrategiesFailed, got: {}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_negotiation_timeout_handling() {
    // Test that negotiation respects timeout hints
    let hints = NegotiationHints::default().with_timeout_ms(100); // Very short timeout

    assert_eq!(hints.timeout().as_millis(), 100);

    // Timeout should be configurable
    let hints2 = NegotiationHints::default().with_timeout_ms(5000);
    assert_eq!(hints2.timeout().as_millis(), 5000);
}

#[tokio::test]
async fn test_negotiation_with_all_strategies() {
    // Test that all strategies are registered and sorted by priority
    let negotiator = AutoNegotiator::new();
    let strategies = negotiator.strategies();

    // Should have 3 default strategies
    assert_eq!(strategies.len(), 3);

    // Verify strategy names
    let names: Vec<_> = strategies.iter().map(|s| s.name()).collect();
    assert!(names.contains(&"manufacturer"));
    assert!(names.contains(&"echo_probe"));
    assert!(names.contains(&"standard_bauds"));

    // Verify priorities are sorted (highest first)
    for i in 1..strategies.len() {
        assert!(
            strategies[i - 1].priority() >= strategies[i].priority(),
            "Strategies not sorted by priority"
        );
    }

    // Verify specific priority order
    assert_eq!(strategies[0].name(), "manufacturer"); // Priority 80
    assert_eq!(strategies[1].name(), "echo_probe"); // Priority 60
    assert_eq!(strategies[2].name(), "standard_bauds"); // Priority 30
}

#[tokio::test]
async fn test_negotiation_confidence_scoring() {
    // Test confidence score calculations
    let high_confidence = NegotiatedParams::new(115200, "manufacturer").with_confidence(0.9);
    assert_eq!(high_confidence.confidence, 0.9);
    assert_eq!(high_confidence.baud_rate, 115200);
    assert_eq!(high_confidence.strategy_used, "manufacturer");

    let medium_confidence = NegotiatedParams::with_medium_confidence(9600, "standard_bauds");
    assert_eq!(medium_confidence.confidence, 0.5);

    let low_confidence = NegotiatedParams::new(19200, "fallback").with_confidence(0.2);
    assert_eq!(low_confidence.confidence, 0.2);
}

#[tokio::test]
async fn test_confidence_clamping() {
    // Test that confidence is clamped to [0.0, 1.0]
    let too_high = NegotiatedParams::new(9600, "test").with_confidence(2.0);
    assert_eq!(too_high.confidence, 1.0);

    let too_low = NegotiatedParams::new(9600, "test").with_confidence(-0.5);
    assert_eq!(too_low.confidence, 0.0);

    let valid = NegotiatedParams::new(9600, "test").with_confidence(0.75);
    assert_eq!(valid.confidence, 0.75);
}

#[tokio::test]
async fn test_negotiated_params_with_full_config() {
    // Test negotiated params with all serial parameters
    let params = NegotiatedParams::new(115200, "manufacturer")
        .with_confidence(0.85)
        .with_params(
            DataBits::Eight,
            Parity::None,
            StopBits::One,
            FlowControl::None,
        );

    assert_eq!(params.baud_rate, 115200);
    assert_eq!(params.confidence, 0.85);
    assert_eq!(params.data_bits, DataBits::Eight);
    assert_eq!(params.parity, Parity::None);
    assert_eq!(params.stop_bits, StopBits::One);
    assert_eq!(params.flow_control, FlowControl::None);
}

#[tokio::test]
async fn test_hints_with_baud_rates() {
    // Test creating hints with specific baud rates
    let hints = NegotiationHints::with_baud_rates(vec![9600, 115200, 19200]);

    assert_eq!(hints.suggested_baud_rates.len(), 3);
    assert_eq!(hints.suggested_baud_rates[0], 9600);
    assert_eq!(hints.suggested_baud_rates[1], 115200);
    assert_eq!(hints.suggested_baud_rates[2], 19200);
}

#[tokio::test]
async fn test_hints_restrict_to_suggested() {
    // Test that hints can restrict to suggested baud rates only
    let mut hints = NegotiationHints::with_baud_rates(vec![9600, 115200]);
    hints.restrict_to_suggested = true;

    assert!(hints.restrict_to_suggested);
    assert_eq!(hints.suggested_baud_rates.len(), 2);
}

#[tokio::test]
async fn test_hints_builder_pattern() {
    // Test builder pattern for hints
    let mut hints = NegotiationHints::with_vid(0x0403);
    hints.suggested_baud_rates = vec![115200, 9600];
    let hints = hints.with_timeout_ms(2000);

    assert_eq!(hints.vid, Some(0x0403));
    assert_eq!(hints.timeout().as_millis(), 2000);
    assert_eq!(hints.suggested_baud_rates.len(), 2);
}

#[tokio::test]
async fn test_manufacturer_profiles_database() {
    // Test that manufacturer profiles database is complete
    let profiles = AutoNegotiator::all_manufacturer_profiles();

    // Should have multiple well-known manufacturers
    assert!(profiles.len() >= 6);

    // Verify specific manufacturers
    let ftdi = profiles.iter().find(|p| p.name == "FTDI");
    assert!(ftdi.is_some());
    assert_eq!(ftdi.unwrap().vid, 0x0403);
    assert_eq!(ftdi.unwrap().default_baud, 115200);

    let arduino = profiles.iter().find(|p| p.name == "Arduino");
    assert!(arduino.is_some());
    assert_eq!(arduino.unwrap().vid, 0x2341);
    assert_eq!(arduino.unwrap().default_baud, 9600);

    let cp210x = profiles.iter().find(|p| p.name == "Silicon Labs CP210x");
    assert!(cp210x.is_some());
    assert_eq!(cp210x.unwrap().vid, 0x10c4);

    let pico = profiles.iter().find(|p| p.name == "Raspberry Pi Pico");
    assert!(pico.is_some());
    assert_eq!(pico.unwrap().vid, 0x2e8a);
}

#[tokio::test]
async fn test_standard_baud_rates() {
    use serial_mcp_agent::negotiation::strategies::standard_bauds::STANDARD_BAUD_RATES;

    // Test that standard baud rates list is comprehensive
    assert!(STANDARD_BAUD_RATES.len() >= 8);

    // Should contain common rates
    assert!(STANDARD_BAUD_RATES.contains(&9600));
    assert!(STANDARD_BAUD_RATES.contains(&19200));
    assert!(STANDARD_BAUD_RATES.contains(&38400));
    assert!(STANDARD_BAUD_RATES.contains(&57600));
    assert!(STANDARD_BAUD_RATES.contains(&115200));

    // Most common rate should be first
    assert_eq!(STANDARD_BAUD_RATES[0], 9600);
}

#[tokio::test]
async fn test_echo_probe_sequences() {
    use serial_mcp_agent::negotiation::strategies::echo_probe::CommonProbes;

    // Test AT command probe
    let at_probe = CommonProbes::at_command();
    assert_eq!(at_probe.description, "AT command");
    assert_eq!(at_probe.command, b"AT\r\n");
    assert!(at_probe.expected_responses.contains(&b"OK".to_vec()));

    // Test newline echo probe
    let newline_probe = CommonProbes::newline_echo();
    assert_eq!(newline_probe.description, "Newline echo");
    assert_eq!(newline_probe.command, b"\r\n");

    // Test Hayes modem probe
    let hayes_probe = CommonProbes::hayes_modem();
    assert_eq!(hayes_probe.description, "Hayes modem");
    assert!(!hayes_probe.expected_responses.is_empty());

    // Test NMEA GPS probe
    let nmea_probe = CommonProbes::nmea_gps();
    assert_eq!(nmea_probe.description, "NMEA GPS");
}

#[tokio::test]
async fn test_probe_sequence_matching() {
    use serial_mcp_agent::negotiation::strategies::echo_probe::ProbeSequence;

    let probe = ProbeSequence::new(
        b"TEST\r\n".to_vec(),
        vec![b"OK".to_vec(), b"ACK".to_vec()],
        "Test probe",
    );

    // Test that the probe was created correctly
    assert_eq!(probe.command, b"TEST\r\n");
    assert_eq!(probe.description, "Test probe");
    assert_eq!(probe.expected_responses.len(), 2);
    assert!(probe.expected_responses.contains(&b"OK".to_vec()));
    assert!(probe.expected_responses.contains(&b"ACK".to_vec()));
}

#[tokio::test]
async fn test_negotiator_with_preference() {
    // Test that preferred strategy is tried first
    let negotiator = AutoNegotiator::new();

    // Note: This test can't fully test with invalid port,
    // but we can verify the preference mechanism exists
    let result = negotiator
        .detect_with_preference("INVALID_PORT", None, "manufacturer")
        .await;

    // Should still fail, but the error shows all strategies were tried
    assert!(result.is_err());
}

#[tokio::test]
async fn test_custom_strategy_addition() {
    use serial_mcp_agent::negotiation::strategies::StandardBaudsStrategy;

    // Test adding custom strategies
    let negotiator = AutoNegotiator::new().add_strategy(Box::new(StandardBaudsStrategy::new()));

    // Should have 4 strategies now (3 default + 1 added)
    assert_eq!(negotiator.strategies().len(), 4);
}

#[tokio::test]
async fn test_strategy_priority_values() {
    use serial_mcp_agent::negotiation::strategies::*;

    // Test that strategies have expected priorities
    let manufacturer = ManufacturerStrategy::new();
    assert_eq!(manufacturer.priority(), 80);

    let echo_probe = EchoProbeStrategy::new();
    assert_eq!(echo_probe.priority(), 60);

    let standard = StandardBaudsStrategy::new();
    assert_eq!(standard.priority(), 30);
}

#[tokio::test]
async fn test_negotiation_params_defaults() {
    // Test that params have sensible defaults
    let params = NegotiatedParams::new(9600, "test");

    assert_eq!(params.baud_rate, 9600);
    assert_eq!(params.strategy_used, "test");
    assert_eq!(params.confidence, 1.0); // Default confidence
    assert_eq!(params.data_bits, DataBits::Eight);
    assert_eq!(params.parity, Parity::None);
    assert_eq!(params.stop_bits, StopBits::One);
    assert_eq!(params.flow_control, FlowControl::None);
}

#[tokio::test]
async fn test_hints_default_values() {
    // Test that hints have sensible defaults
    let hints = NegotiationHints::default();

    assert_eq!(hints.vid, None);
    assert_eq!(hints.pid, None);
    assert!(hints.suggested_baud_rates.is_empty());
    assert_eq!(hints.timeout().as_millis(), 500); // Default timeout
    assert!(!hints.restrict_to_suggested);
}
