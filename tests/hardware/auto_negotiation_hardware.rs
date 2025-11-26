//! Hardware tests for auto-negotiation with real devices.
//!
//! These tests verify that auto-negotiation strategies work correctly
//! with actual serial hardware, not just mocks.
//!
//! # Running These Tests
//!
//! ```bash
//! # Basic auto-negotiation test
//! TEST_PORT=COM3 cargo test --features auto-negotiation --ignored test_real_auto_negotiation
//!
//! # Manufacturer profile test (requires USB device)
//! TEST_PORT=COM3 cargo test --features auto-negotiation --ignored test_real_manufacturer_profile
//!
//! # Baud rate detection test
//! TEST_PORT=COM3 TEST_BAUD=115200 cargo test --features auto-negotiation --ignored test_real_baud_detection
//! ```

#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::assertions_on_constants)]

use serial_mcp_agent::negotiation::{
    strategies::{ManufacturerStrategy, StandardBaudsStrategy},
    AutoNegotiator, NegotiationHints,
};
use serialport::SerialPortType;
use std::time::Duration;

use crate::hardware::utils::{
    discover_available_ports, get_port_info, get_port_vid, print_available_ports, TestPortConfig,
    TimingHelper,
};

#[tokio::test]
#[ignore] // Requires hardware
async fn test_real_auto_negotiation_with_timing() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            print_available_ports();
            return;
        }
    };

    let timer = TimingHelper::new(&format!(
        "Auto-negotiation on {} at {} baud",
        config.port_name, config.baud_rate
    ));

    let negotiator = AutoNegotiator::new();
    let mut hints = NegotiationHints::default();
    hints.timeout_ms = 2000;
    hints.vid = get_port_vid(&config.port_name);

    timer.checkpoint("Starting negotiation");

    let result = negotiator.detect(&config.port_name, Some(hints)).await;

    let elapsed = timer.finish_with_result(&result);

    match result {
        Ok(params) => {
            println!("✅ Auto-negotiation succeeded:");
            println!("   Baud rate: {}", params.baud_rate);
            println!("   Strategy: {}", params.strategy_used);
            println!("   Confidence: {:.2}", params.confidence);
            println!("   Data bits: {:?}", params.data_bits);
            println!("   Parity: {:?}", params.parity);
            println!("   Stop bits: {:?}", params.stop_bits);
            println!("   Time taken: {:?}", elapsed);

            assert!(params.baud_rate > 0);
            assert!(params.confidence >= 0.0 && params.confidence <= 1.0);
            assert!(
                elapsed < Duration::from_secs(10),
                "Negotiation took too long"
            );
        }
        Err(e) => {
            println!("⚠️  Negotiation failed (may be expected for some devices):");
            println!("   Error: {}", e);
            println!("   Time taken: {:?}", elapsed);
            // Don't fail - some devices don't support auto-negotiation
        }
    }
}

#[tokio::test]
#[ignore] // Requires USB hardware
async fn test_real_manufacturer_profile_detection() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            print_available_ports();
            return;
        }
    };

    println!(
        "Testing manufacturer profile detection on: {}",
        config.port_name
    );

    // Get port info to check if it's a USB device
    let port_info = match get_port_info(&config.port_name) {
        Some(info) => info,
        None => {
            println!("❌ Port {} not found", config.port_name);
            panic!("Port not found in available ports list");
        }
    };

    match &port_info.port_type {
        SerialPortType::UsbPort(usb_info) => {
            println!("✅ USB Port detected:");
            println!("   VID: {:04x}", usb_info.vid);
            println!("   PID: {:04x}", usb_info.pid);

            if let Some(ref manufacturer) = usb_info.manufacturer {
                println!("   Manufacturer: {}", manufacturer);
            }

            if let Some(ref product) = usb_info.product {
                println!("   Product: {}", product);
            }

            // Check if we have a manufacturer profile
            if let Some(profile) = AutoNegotiator::get_manufacturer_profile(usb_info.vid) {
                println!("✅ Matched manufacturer profile:");
                println!("   Name: {}", profile.name);
                println!("   Default baud: {}", profile.default_baud);
                println!("   Common bauds: {:?}", profile.common_bauds);

                assert_eq!(profile.vid, usb_info.vid);

                // Test negotiation with manufacturer strategy
                let negotiator = AutoNegotiator::new();
                let mut hints = NegotiationHints::default();
                hints.timeout_ms = 2000;
                hints.vid = Some(usb_info.vid);

                let timer = TimingHelper::new("Manufacturer-based negotiation");
                let result = negotiator.detect(&config.port_name, Some(hints)).await;
                let elapsed = timer.finish_with_result(&result);

                if let Ok(params) = result {
                    println!("✅ Manufacturer negotiation succeeded:");
                    println!("   Detected baud: {}", params.baud_rate);
                    println!("   Expected baud: {}", profile.default_baud);
                    println!("   Time: {:?}", elapsed);

                    // Should have used manufacturer strategy
                    assert_eq!(params.strategy_used, "manufacturer");

                    // Should have high confidence
                    assert!(
                        params.confidence >= 0.7,
                        "Expected high confidence, got {}",
                        params.confidence
                    );
                } else {
                    println!("⚠️  Manufacturer negotiation failed (device may be in use)");
                }
            } else {
                println!("⚠️  No manufacturer profile for VID {:04x}", usb_info.vid);
                println!("   This is not a test failure - just an unknown device");
            }
        }
        other => {
            println!("ℹ️  Non-USB port type: {:?}", other);
            println!("   Manufacturer detection requires USB devices");
        }
    }
}

#[tokio::test]
#[ignore] // Requires hardware
async fn test_real_baud_rate_detection() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    println!("Testing baud rate detection on: {}", config.port_name);
    println!("Expected baud: {}", config.baud_rate);

    let negotiator = AutoNegotiator::new();
    let mut hints = NegotiationHints::default();
    hints.timeout_ms = 1500;
    hints.vid = get_port_vid(&config.port_name);

    let timer = TimingHelper::new("Baud rate detection");
    let result = negotiator.detect(&config.port_name, Some(hints)).await;
    let elapsed = timer.finish_with_result(&result);

    match result {
        Ok(params) => {
            println!("✅ Baud detection succeeded:");
            println!("   Detected: {} baud", params.baud_rate);
            println!("   Expected: {} baud", config.baud_rate);
            println!("   Strategy: {}", params.strategy_used);
            println!("   Confidence: {:.2}", params.confidence);
            println!("   Time: {:?}", elapsed);

            // Note: Detected baud may differ from TEST_BAUD if device auto-detects
            assert!(params.baud_rate > 0);
        }
        Err(e) => {
            println!("⚠️  Baud detection failed: {}", e);
            // Don't panic - device might be busy or not responsive
        }
    }
}

#[tokio::test]
#[ignore] // Requires hardware
async fn test_real_standard_bauds_strategy() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    println!("Testing standard bauds strategy on: {}", config.port_name);

    // Use only the standard bauds strategy
    let strategy = StandardBaudsStrategy::new();
    let negotiator = AutoNegotiator::with_strategies(vec![Box::new(strategy)]);

    let mut hints = NegotiationHints::default();
    hints.timeout_ms = 500; // Short timeout per baud

    let timer = TimingHelper::new("Standard bauds strategy");
    let result = negotiator.detect(&config.port_name, Some(hints)).await;
    let elapsed = timer.finish_with_result(&result);

    match result {
        Ok(params) => {
            println!("✅ Standard bauds strategy succeeded:");
            println!("   Detected baud: {}", params.baud_rate);
            println!("   Strategy: {}", params.strategy_used);
            println!("   Confidence: {:.2}", params.confidence);
            println!("   Time: {:?}", elapsed);

            assert_eq!(params.strategy_used, "standard_bauds");
            assert!(params.baud_rate > 0);

            // Common baud rates
            assert!(
                [4800, 9600, 19200, 38400, 57600, 115200].contains(&params.baud_rate),
                "Expected standard baud rate, got {}",
                params.baud_rate
            );
        }
        Err(e) => {
            println!("⚠️  Standard bauds strategy failed: {}", e);
            println!("   This can happen if the device requires specific settings");
        }
    }
}

#[tokio::test]
#[ignore] // Requires hardware with manufacturer profile
async fn test_real_manufacturer_strategy_only() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    // Get VID for hints
    let vid = match get_port_vid(&config.port_name) {
        Some(v) => v,
        None => {
            println!("⏭️  Skipping: Port is not a USB device");
            return;
        }
    };

    // Check if we have a profile for this VID
    if AutoNegotiator::get_manufacturer_profile(vid).is_none() {
        println!("⏭️  Skipping: No manufacturer profile for VID {:04x}", vid);
        return;
    }

    println!("Testing manufacturer strategy on: {}", config.port_name);
    println!("VID: {:04x}", vid);

    // Use only manufacturer strategy
    let strategy = ManufacturerStrategy::new();
    let negotiator = AutoNegotiator::with_strategies(vec![Box::new(strategy)]);

    let mut hints = NegotiationHints::default();
    hints.timeout_ms = 1000;
    hints.vid = Some(vid);

    let timer = TimingHelper::new("Manufacturer strategy");
    let result = negotiator.detect(&config.port_name, Some(hints)).await;
    let elapsed = timer.finish_with_result(&result);

    match result {
        Ok(params) => {
            println!("✅ Manufacturer strategy succeeded:");
            println!("   Detected baud: {}", params.baud_rate);
            println!("   Strategy: {}", params.strategy_used);
            println!("   Confidence: {:.2}", params.confidence);
            println!("   Time: {:?}", elapsed);

            assert_eq!(params.strategy_used, "manufacturer");
            assert!(params.baud_rate > 0);
            assert!(
                params.confidence >= 0.7,
                "Expected high confidence from manufacturer profile"
            );

            // Should be fast since it tries manufacturer-specific bauds
            assert!(
                elapsed < Duration::from_secs(5),
                "Manufacturer strategy should be fast"
            );
        }
        Err(e) => {
            println!("❌ Manufacturer strategy failed: {}", e);
            panic!("Manufacturer strategy should work when profile exists");
        }
    }
}

#[tokio::test]
#[ignore] // Requires multiple ports
async fn test_real_multi_port_detection() {
    let ports = discover_available_ports();

    if ports.len() < 2 {
        println!("⏭️  Skipping: Need at least 2 ports for multi-port test");
        println!("   Found {} port(s)", ports.len());
        return;
    }

    println!("Testing multi-port detection on {} ports", ports.len());

    let negotiator = AutoNegotiator::new();

    // Build hints for each port
    let port_configs: Vec<(String, Option<NegotiationHints>)> = ports
        .iter()
        .take(3) // Limit to first 3 ports
        .map(|port| {
            let vid = if let SerialPortType::UsbPort(usb_info) = &port.port_type {
                Some(usb_info.vid)
            } else {
                None
            };

            let mut hints = NegotiationHints::default();
            hints.timeout_ms = 1000;
            hints.vid = vid;

            (port.port_name.clone(), Some(hints))
        })
        .collect();

    let timer = TimingHelper::new(&format!(
        "Multi-port detection ({} ports)",
        port_configs.len()
    ));
    let results = negotiator.detect_multiple(port_configs).await;
    let elapsed = timer.finish();

    println!("\nResults:");
    for (port_name, result) in results {
        match result {
            Ok(params) => {
                println!(
                    "✅ {}: {} baud ({})",
                    port_name, params.baud_rate, params.strategy_used
                );
            }
            Err(e) => {
                println!("❌ {}: {}", port_name, e);
            }
        }
    }

    println!("\nTotal time: {:?}", elapsed);
    let port_count = ports.len().min(3);
    if port_count > 0 {
        println!("Average time per port: {:?}", elapsed / (port_count as u32));
    }
}

#[test]
#[ignore] // Requires hardware
fn test_list_all_manufacturer_profiles() {
    println!("Known manufacturer profiles:");
    println!("{:=<70}", "");

    for profile in AutoNegotiator::all_manufacturer_profiles() {
        println!("VID: 0x{:04X} - {}", profile.vid, profile.name);
        println!("  Default baud: {}", profile.default_baud);
        println!("  Common bauds: {:?}", profile.common_bauds);
        println!("{:-<70}", "");
    }

    assert!(!AutoNegotiator::all_manufacturer_profiles().is_empty());
}
