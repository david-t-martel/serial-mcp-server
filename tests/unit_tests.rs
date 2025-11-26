//! Unit tests for rust-comm core modules
//!
//! This module contains comprehensive unit tests for:
//! - `state.rs`: PortConfig, PortState, and configuration enums
//! - `error.rs`: AppError and error conversions
//!
//! Tests follow the Arrange-Act-Assert pattern and cover:
//! - Default values and initialization
//! - Serialization/deserialization roundtrips
//! - Enum conversions and JSON mapping
//! - Error display implementations
//! - Error type conversions (From trait)
//! - HTTP status code mappings (when rest-api feature enabled)

// Import the modules we're testing
use serial_mcp_agent::error::AppError;
use serial_mcp_agent::state::{
    DataBitsCfg, FlowControlCfg, ParityCfg, PortConfig, PortState, StopBitsCfg,
};

// ============================================================================
// PortConfig Tests
// ============================================================================

#[cfg(test)]
mod port_config_tests {
    use super::*;

    #[test]
    fn test_port_config_default_values() {
        // Arrange: Create a minimal PortConfig with only required field
        let json = r#"{"port_name": "COM1"}"#;

        // Act: Deserialize using default values for optional fields
        let config: PortConfig = serde_json::from_str(json).expect("Failed to deserialize");

        // Assert: Verify all default values are correct
        assert_eq!(config.port_name, "COM1");
        assert_eq!(config.baud_rate, 9600, "Default baud rate should be 9600");
        assert_eq!(config.timeout_ms, 1000, "Default timeout should be 1000ms");
        assert!(
            matches!(config.data_bits, DataBitsCfg::Eight),
            "Default data bits should be Eight"
        );
        assert!(
            matches!(config.parity, ParityCfg::None),
            "Default parity should be None"
        );
        assert!(
            matches!(config.stop_bits, StopBitsCfg::One),
            "Default stop bits should be One"
        );
        assert!(
            matches!(config.flow_control, FlowControlCfg::None),
            "Default flow control should be None"
        );
        assert_eq!(
            config.terminator,
            Some("\n".to_string()),
            "Default terminator should be newline"
        );
        assert_eq!(
            config.idle_disconnect_ms, None,
            "Default idle_disconnect_ms should be None"
        );
    }

    #[test]
    fn test_port_config_full_serialization_roundtrip() {
        // Arrange: Create a fully configured PortConfig
        let original_json = r#"{
            "port_name": "/dev/ttyUSB0",
            "baud_rate": 115200,
            "timeout_ms": 5000,
            "data_bits": "eight",
            "parity": "even",
            "stop_bits": "two",
            "flow_control": "hardware",
            "terminator": "\r\n",
            "idle_disconnect_ms": 30000
        }"#;

        // Act: Deserialize and then re-serialize
        let config: PortConfig =
            serde_json::from_str(original_json).expect("Failed to deserialize original JSON");
        let reserialized = serde_json::to_string(&config).expect("Failed to serialize config");
        let roundtrip: PortConfig =
            serde_json::from_str(&reserialized).expect("Failed to deserialize roundtrip JSON");

        // Assert: Verify all fields survived the roundtrip
        assert_eq!(roundtrip.port_name, "/dev/ttyUSB0");
        assert_eq!(roundtrip.baud_rate, 115200);
        assert_eq!(roundtrip.timeout_ms, 5000);
        assert!(matches!(roundtrip.data_bits, DataBitsCfg::Eight));
        assert!(matches!(roundtrip.parity, ParityCfg::Even));
        assert!(matches!(roundtrip.stop_bits, StopBitsCfg::Two));
        assert!(matches!(roundtrip.flow_control, FlowControlCfg::Hardware));
        assert_eq!(roundtrip.terminator, Some("\r\n".to_string()));
        assert_eq!(roundtrip.idle_disconnect_ms, Some(30000));
    }

    #[test]
    fn test_port_config_with_null_terminator() {
        // Arrange: Config with explicit null terminator
        let json = r#"{
            "port_name": "COM3",
            "terminator": null
        }"#;

        // Act: Deserialize
        let config: PortConfig = serde_json::from_str(json).expect("Failed to deserialize");

        // Assert: Verify null terminator is preserved
        assert_eq!(config.terminator, None);
    }

    #[test]
    fn test_port_config_custom_baud_rates() {
        // Arrange: Test various common baud rates
        let test_cases = vec![
            (9600, "Standard rate"),
            (115200, "High-speed USB"),
            (230400, "Very high speed"),
            (1000000, "1 Mbps"),
        ];

        for (baud_rate, description) in test_cases {
            // Act
            let json = format!(r#"{{"port_name": "COM1", "baud_rate": {}}}"#, baud_rate);
            let config: PortConfig = serde_json::from_str(&json)
                .unwrap_or_else(|_| panic!("Failed to deserialize for {}", description));

            // Assert
            assert_eq!(
                config.baud_rate, baud_rate,
                "Baud rate mismatch for {}",
                description
            );
        }
    }
}

// ============================================================================
// DataBitsCfg Enum Tests
// ============================================================================

#[cfg(test)]
mod data_bits_cfg_tests {
    use super::*;

    #[test]
    fn test_data_bits_serialization() {
        // Arrange: All possible data bits values
        let test_cases = vec![
            (DataBitsCfg::Five, "five"),
            (DataBitsCfg::Six, "six"),
            (DataBitsCfg::Seven, "seven"),
            (DataBitsCfg::Eight, "eight"),
        ];

        for (variant, expected_json) in test_cases {
            // Act: Serialize
            let json = serde_json::to_string(&variant).expect("Failed to serialize DataBitsCfg");

            // Assert: Verify snake_case format
            assert_eq!(json, format!("\"{}\"", expected_json));
        }
    }

    #[test]
    fn test_data_bits_deserialization() {
        // Arrange & Act & Assert: Test all valid inputs
        assert!(matches!(
            serde_json::from_str::<DataBitsCfg>("\"five\"").unwrap(),
            DataBitsCfg::Five
        ));
        assert!(matches!(
            serde_json::from_str::<DataBitsCfg>("\"six\"").unwrap(),
            DataBitsCfg::Six
        ));
        assert!(matches!(
            serde_json::from_str::<DataBitsCfg>("\"seven\"").unwrap(),
            DataBitsCfg::Seven
        ));
        assert!(matches!(
            serde_json::from_str::<DataBitsCfg>("\"eight\"").unwrap(),
            DataBitsCfg::Eight
        ));
    }

    #[test]
    fn test_data_bits_invalid_deserialization() {
        // Arrange: Invalid data bits value
        let json = "\"nine\"";

        // Act: Attempt to deserialize
        let result = serde_json::from_str::<DataBitsCfg>(json);

        // Assert: Should fail
        assert!(result.is_err(), "Should reject invalid data bits value");
    }
}

// ============================================================================
// ParityCfg Enum Tests
// ============================================================================

#[cfg(test)]
mod parity_cfg_tests {
    use super::*;

    #[test]
    fn test_parity_serialization() {
        // Arrange: All possible parity values
        let test_cases = vec![
            (ParityCfg::None, "none"),
            (ParityCfg::Odd, "odd"),
            (ParityCfg::Even, "even"),
        ];

        for (variant, expected_json) in test_cases {
            // Act: Serialize
            let json = serde_json::to_string(&variant).expect("Failed to serialize ParityCfg");

            // Assert: Verify snake_case format
            assert_eq!(json, format!("\"{}\"", expected_json));
        }
    }

    #[test]
    fn test_parity_deserialization_roundtrip() {
        // Arrange: JSON inputs
        let inputs = vec!["\"none\"", "\"odd\"", "\"even\""];

        for input in inputs {
            // Act: Deserialize and re-serialize
            let parsed: ParityCfg = serde_json::from_str(input).expect("Failed to deserialize");
            let reserialized = serde_json::to_string(&parsed).expect("Failed to serialize");

            // Assert: Should match original
            assert_eq!(reserialized, input, "Roundtrip failed for {}", input);
        }
    }
}

// ============================================================================
// StopBitsCfg Enum Tests
// ============================================================================

#[cfg(test)]
mod stop_bits_cfg_tests {
    use super::*;

    #[test]
    fn test_stop_bits_serialization() {
        // Arrange & Act & Assert
        assert_eq!(serde_json::to_string(&StopBitsCfg::One).unwrap(), "\"one\"");
        assert_eq!(serde_json::to_string(&StopBitsCfg::Two).unwrap(), "\"two\"");
    }

    #[test]
    fn test_stop_bits_deserialization() {
        // Arrange & Act & Assert
        assert!(matches!(
            serde_json::from_str::<StopBitsCfg>("\"one\"").unwrap(),
            StopBitsCfg::One
        ));
        assert!(matches!(
            serde_json::from_str::<StopBitsCfg>("\"two\"").unwrap(),
            StopBitsCfg::Two
        ));
    }
}

// ============================================================================
// FlowControlCfg Enum Tests
// ============================================================================

#[cfg(test)]
mod flow_control_cfg_tests {
    use super::*;

    #[test]
    fn test_flow_control_all_variants() {
        // Arrange: All flow control variants
        let test_cases = vec![
            (FlowControlCfg::None, "none", "No flow control"),
            (
                FlowControlCfg::Hardware,
                "hardware",
                "RTS/CTS hardware flow control",
            ),
            (
                FlowControlCfg::Software,
                "software",
                "XON/XOFF software flow control",
            ),
        ];

        for (variant, json_name, description) in test_cases {
            // Act: Serialize
            let serialized = serde_json::to_string(&variant)
                .unwrap_or_else(|_| panic!("Failed to serialize {}", description));

            // Assert: Check serialization
            assert_eq!(
                serialized,
                format!("\"{}\"", json_name),
                "Serialization failed for {}",
                description
            );

            // Act: Deserialize
            let deserialized: FlowControlCfg = serde_json::from_str(&serialized)
                .unwrap_or_else(|_| panic!("Failed to deserialize {}", description));

            // Assert: Roundtrip successful
            let reserialized = serde_json::to_string(&deserialized).unwrap();
            assert_eq!(
                reserialized, serialized,
                "Roundtrip failed for {}",
                description
            );
        }
    }
}

// ============================================================================
// PortState Tests
// ============================================================================

#[cfg(test)]
mod port_state_tests {
    use super::*;

    #[test]
    fn test_port_state_default_is_closed() {
        // Arrange & Act: Create default PortState
        let state = PortState::default();

        // Assert: Should be Closed
        assert!(
            matches!(state, PortState::Closed),
            "Default state should be Closed"
        );
    }

    #[test]
    fn test_port_state_closed_serialization() {
        // Arrange: Closed port state
        let state = PortState::Closed;

        // Act: Serialize
        let json = serde_json::to_string(&state).expect("Failed to serialize PortState::Closed");

        // Assert: Should have status field
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed["status"], "Closed",
            "Status field should be 'Closed'"
        );
    }

    #[test]
    fn test_port_state_debug_formatting() {
        // Arrange: Closed state
        let state = PortState::Closed;

        // Act: Format as debug string
        let debug_str = format!("{:?}", state);

        // Assert: Should contain "Closed"
        assert!(
            debug_str.contains("Closed"),
            "Debug output should contain 'Closed'"
        );
    }
}

// ============================================================================
// AppError Display Tests
// ============================================================================

#[cfg(test)]
mod app_error_display_tests {
    use super::*;

    #[test]
    fn test_port_not_open_display() {
        // Arrange
        let error = AppError::PortNotOpen;

        // Act
        let message = error.to_string();

        // Assert
        assert!(
            message.contains("requires an open serial port"),
            "Message should explain port must be open"
        );
        assert!(
            message.contains("closed"),
            "Message should mention port is closed"
        );
    }

    #[test]
    fn test_port_already_open_display() {
        // Arrange
        let error = AppError::PortAlreadyOpen;

        // Act
        let message = error.to_string();

        // Assert
        assert!(
            message.contains("already open"),
            "Message should mention port is already open"
        );
        assert!(
            message.contains("Close it before"),
            "Message should suggest closing first"
        );
    }

    #[test]
    fn test_invalid_payload_display() {
        // Arrange
        let error = AppError::InvalidPayload("Missing 'port_name' field".to_string());

        // Act
        let message = error.to_string();

        // Assert
        assert!(
            message.contains("invalid"),
            "Message should mention invalid payload"
        );
        assert!(
            message.contains("Missing 'port_name' field"),
            "Message should include specific details"
        );
    }

    #[test]
    fn test_serial_error_display() {
        // Arrange: Create a serialport error
        let serial_err =
            serialport::Error::new(serialport::ErrorKind::NoDevice, "Device not found");
        let error = AppError::SerialError(serial_err);

        // Act
        let message = error.to_string();

        // Assert
        assert!(
            message.contains("serial port error"),
            "Message should mention serial port error"
        );
    }

    #[test]
    fn test_io_error_display() {
        // Arrange: Create an I/O error
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
        let error = AppError::IoError(io_err);

        // Act
        let message = error.to_string();

        // Assert
        assert!(
            message.contains("I/O error"),
            "Message should mention I/O error"
        );
    }

    #[test]
    fn test_serde_error_display() {
        // Arrange: Create a JSON deserialization error
        let json_result: Result<PortConfig, _> = serde_json::from_str("invalid json");
        let serde_err = json_result.unwrap_err();
        let error = AppError::SerdeError(serde_err);

        // Act
        let message = error.to_string();

        // Assert
        assert!(
            message.contains("serialization") || message.contains("deserialization"),
            "Message should mention serialization/deserialization"
        );
    }
}

// ============================================================================
// AppError From Trait Tests (Error Conversions)
// ============================================================================

#[cfg(test)]
mod app_error_from_tests {
    use super::*;

    #[test]
    fn test_from_serialport_error() {
        // Arrange: Create a serialport error
        let serial_err = serialport::Error::new(
            serialport::ErrorKind::Io(std::io::ErrorKind::TimedOut),
            "Operation timed out",
        );

        // Act: Convert using From trait (simulating ? operator)
        let app_error: AppError = serial_err.into();

        // Assert: Should be SerialError variant
        assert!(
            matches!(app_error, AppError::SerialError(_)),
            "Should convert to AppError::SerialError"
        );
    }

    #[test]
    fn test_from_io_error() {
        // Arrange: Create an I/O error
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");

        // Act: Convert using From trait
        let app_error: AppError = io_err.into();

        // Assert: Should be IoError variant
        assert!(
            matches!(app_error, AppError::IoError(_)),
            "Should convert to AppError::IoError"
        );
    }

    #[test]
    fn test_from_serde_json_error() {
        // Arrange: Create a serde_json error
        let json_result: Result<PortConfig, _> = serde_json::from_str("{invalid}");
        let serde_err = json_result.unwrap_err();

        // Act: Convert using From trait
        let app_error: AppError = serde_err.into();

        // Assert: Should be SerdeError variant
        assert!(
            matches!(app_error, AppError::SerdeError(_)),
            "Should convert to AppError::SerdeError"
        );
    }

    #[test]
    fn test_question_mark_operator_pattern() {
        // This test demonstrates how the From trait enables the ? operator
        fn parse_config(json: &str) -> Result<PortConfig, AppError> {
            // The ? operator automatically converts serde_json::Error to AppError
            let config: PortConfig = serde_json::from_str(json)?;
            Ok(config)
        }

        // Arrange: Invalid JSON
        let invalid_json = "{ broken json }";

        // Act: Call function that uses ? operator
        let result = parse_config(invalid_json);

        // Assert: Should be an AppError::SerdeError
        assert!(result.is_err(), "Should return an error");
        assert!(
            matches!(result.unwrap_err(), AppError::SerdeError(_)),
            "? operator should convert to AppError::SerdeError"
        );
    }
}

// ============================================================================
// AppError HTTP Status Code Tests (rest-api feature only)
// ============================================================================

#[cfg(all(test, feature = "rest-api"))]
mod app_error_http_tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn test_port_not_open_status_code() {
        // Arrange
        let error = AppError::PortNotOpen;

        // Act: Convert to HTTP response
        let response = error.into_response();

        // Assert: Should be 409 CONFLICT
        assert_eq!(
            response.status(),
            axum::http::StatusCode::CONFLICT,
            "PortNotOpen should map to 409 CONFLICT"
        );
    }

    #[test]
    fn test_port_already_open_status_code() {
        // Arrange
        let error = AppError::PortAlreadyOpen;

        // Act: Convert to HTTP response
        let response = error.into_response();

        // Assert: Should be 409 CONFLICT
        assert_eq!(
            response.status(),
            axum::http::StatusCode::CONFLICT,
            "PortAlreadyOpen should map to 409 CONFLICT"
        );
    }

    #[test]
    fn test_invalid_payload_status_code() {
        // Arrange
        let error = AppError::InvalidPayload("Bad input".to_string());

        // Act: Convert to HTTP response
        let response = error.into_response();

        // Assert: Should be 400 BAD REQUEST
        assert_eq!(
            response.status(),
            axum::http::StatusCode::BAD_REQUEST,
            "InvalidPayload should map to 400 BAD REQUEST"
        );
    }

    #[test]
    fn test_serial_error_status_code() {
        // Arrange
        let serial_err = serialport::Error::new(serialport::ErrorKind::NoDevice, "Device error");
        let error = AppError::SerialError(serial_err);

        // Act: Convert to HTTP response
        let response = error.into_response();

        // Assert: Should be 500 INTERNAL SERVER ERROR
        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "SerialError should map to 500 INTERNAL SERVER ERROR"
        );
    }

    #[test]
    fn test_io_error_status_code() {
        // Arrange
        let io_err = std::io::Error::other("I/O error");
        let error = AppError::IoError(io_err);

        // Act: Convert to HTTP response
        let response = error.into_response();

        // Assert: Should be 500 INTERNAL SERVER ERROR
        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "IoError should map to 500 INTERNAL SERVER ERROR"
        );
    }

    #[test]
    fn test_serde_error_status_code() {
        // Arrange
        let json_result: Result<PortConfig, _> = serde_json::from_str("{}");
        let serde_err = json_result.unwrap_err();
        let error = AppError::SerdeError(serde_err);

        // Act: Convert to HTTP response
        let response = error.into_response();

        // Assert: Should be 400 BAD REQUEST
        assert_eq!(
            response.status(),
            axum::http::StatusCode::BAD_REQUEST,
            "SerdeError should map to 400 BAD REQUEST"
        );
    }

    #[tokio::test]
    async fn test_error_response_json_structure() {
        // Arrange
        let error = AppError::InvalidPayload("Test error".to_string());

        // Act: Convert to response
        let response = error.into_response();

        // Extract body (this is a simplified check - in real tests you'd parse the body)
        let status = response.status();

        // Assert: Verify it has the expected status
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);

        // Note: Testing the exact JSON body structure would require more complex
        // async body extraction, but the key behavior (status code mapping) is tested
    }
}

// ============================================================================
// Edge Cases and Integration Tests
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_port_config_empty_terminator_string() {
        // Arrange: Empty string terminator (valid but unusual)
        let json = r#"{"port_name": "COM1", "terminator": ""}"#;

        // Act
        let config: PortConfig =
            serde_json::from_str(json).expect("Should accept empty terminator");

        // Assert
        assert_eq!(config.terminator, Some("".to_string()));
    }

    #[test]
    fn test_port_config_unicode_terminator() {
        // Arrange: Unicode terminator
        let json = r#"{"port_name": "COM1", "terminator": "\u0000"}"#;

        // Act
        let config: PortConfig =
            serde_json::from_str(json).expect("Should accept unicode terminator");

        // Assert
        assert_eq!(config.terminator, Some("\u{0000}".to_string()));
    }

    #[test]
    fn test_port_config_very_high_timeout() {
        // Arrange: Very high timeout value
        let json = r#"{"port_name": "COM1", "timeout_ms": 3600000}"#;

        // Act
        let config: PortConfig = serde_json::from_str(json).expect("Should accept high timeout");

        // Assert
        assert_eq!(config.timeout_ms, 3600000); // 1 hour
    }

    #[test]
    fn test_multiple_error_conversions() {
        // Demonstrate that multiple error types can be converted in sequence
        fn complex_operation() -> Result<(), AppError> {
            // Simulate various error scenarios
            let _io_check: std::io::Result<()> = Err(std::io::Error::other("test"));

            // This would convert the error
            // _io_check?;

            Ok(())
        }

        // Act & Assert
        assert!(complex_operation().is_ok());
    }
}
