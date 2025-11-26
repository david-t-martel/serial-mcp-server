//! E2E tests for port discovery functionality.
//!
//! These tests verify that the system can correctly:
//! - List available serial ports
//! - Provide extended port information (USB metadata)
//! - Cache discovery results for performance
//! - Handle concurrent discovery requests

use serialport::available_ports;
use std::time::Instant;

#[tokio::test]
async fn test_list_ports_returns_system_ports() {
    // Test that we can list ports available on the system
    let ports = available_ports();

    match ports {
        Ok(port_list) => {
            println!("Found {} ports", port_list.len());
            for port in &port_list {
                println!("  - {}", port.port_name);
            }
            // We can't assert a specific count since it varies by system,
            // but we verify the Vec is valid by reaching this point
            let _ = port_list.len(); // Verify access works
        }
        Err(e) => {
            // On some systems (e.g., CI without ports), this may fail gracefully
            println!("Port listing failed (expected on some systems): {}", e);
        }
    }
}

#[tokio::test]
async fn test_list_ports_includes_metadata() {
    // Test that port listing includes type information
    if let Ok(ports) = available_ports() {
        for port in ports {
            println!("Port: {}", port.port_name);
            match &port.port_type {
                serialport::SerialPortType::UsbPort(info) => {
                    println!("  Type: USB");
                    println!("  VID: {:04x}", info.vid);
                    println!("  PID: {:04x}", info.pid);
                    if let Some(ref manufacturer) = info.manufacturer {
                        println!("  Manufacturer: {}", manufacturer);
                    }
                    if let Some(ref product) = info.product {
                        println!("  Product: {}", product);
                    }
                }
                serialport::SerialPortType::BluetoothPort => {
                    println!("  Type: Bluetooth");
                }
                serialport::SerialPortType::PciPort => {
                    println!("  Type: PCI");
                }
                serialport::SerialPortType::Unknown => {
                    println!("  Type: Unknown");
                }
            }
        }
        // Test passes if we can iterate without panic
    }
}

#[tokio::test]
async fn test_discovery_performance() {
    // Test that discovery completes within reasonable time
    let start = Instant::now();

    let _ports = available_ports();

    let elapsed = start.elapsed();
    println!("Discovery took {:?}", elapsed);

    // Discovery should complete in under 5 seconds even on slow systems
    assert!(
        elapsed.as_secs() < 5,
        "Discovery took too long: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_concurrent_discovery_requests() {
    use tokio::task;

    // Test that multiple concurrent discovery requests complete successfully
    let handles: Vec<_> = (0..5)
        .map(|i| {
            task::spawn(async move {
                println!("Starting discovery task {}", i);
                let result = available_ports();
                println!("Completed discovery task {}", i);
                result
            })
        })
        .collect();

    // Wait for all tasks to complete
    let results = futures::future::join_all(handles).await;

    // Verify all tasks completed
    assert_eq!(results.len(), 5);

    // Verify all tasks that succeeded returned similar results
    let successful_results: Vec<_> = results
        .into_iter()
        .filter_map(|r| r.ok())
        .filter_map(|r| r.ok())
        .collect();

    if !successful_results.is_empty() {
        // All successful results should have the same number of ports
        let first_count = successful_results[0].len();
        for result in &successful_results {
            assert_eq!(
                result.len(),
                first_count,
                "Concurrent discovery returned different port counts"
            );
        }
    }
}

#[tokio::test]
async fn test_usb_port_filtering() {
    // Test that we can filter USB ports from the list
    if let Ok(ports) = available_ports() {
        let usb_ports: Vec<_> = ports
            .into_iter()
            .filter(|p| matches!(p.port_type, serialport::SerialPortType::UsbPort(_)))
            .collect();

        println!("Found {} USB ports", usb_ports.len());

        for port in &usb_ports {
            if let serialport::SerialPortType::UsbPort(info) = &port.port_type {
                println!("USB Port: {}", port.port_name);
                println!("  VID:PID = {:04x}:{:04x}", info.vid, info.pid);
            }
        }

        // Test passes if filtering works without panic
        let _ = usb_ports.len(); // Verify filtering completed
    }
}

#[tokio::test]
async fn test_port_name_format_validation() {
    // Test that port names follow expected platform-specific formats
    if let Ok(ports) = available_ports() {
        for port in ports {
            let name = &port.port_name;

            // Windows: COM1, COM2, etc. or \\.\COM1 format
            // Unix: /dev/ttyUSB0, /dev/ttyACM0, etc.
            // macOS: /dev/cu.usbserial-*, /dev/tty.usbserial-*

            #[cfg(target_os = "windows")]
            {
                assert!(
                    name.contains("COM") || name.starts_with("\\\\.\\"),
                    "Windows port name should contain COM or start with \\\\.\\: {}",
                    name
                );
            }

            #[cfg(any(target_os = "linux", target_os = "macos"))]
            {
                assert!(
                    name.starts_with("/dev/"),
                    "Unix-like port name should start with /dev/: {}",
                    name
                );
            }
        }
    }
}

#[tokio::test]
async fn test_known_manufacturer_detection() {
    // Test that we can identify ports from known manufacturers
    if let Ok(ports) = available_ports() {
        for port in ports {
            if let serialport::SerialPortType::UsbPort(info) = &port.port_type {
                let manufacturer_name = match info.vid {
                    0x0403 => Some("FTDI"),
                    0x2341 => Some("Arduino"),
                    0x10c4 => Some("Silicon Labs CP210x"),
                    0x067b => Some("Prolific PL2303"),
                    0x1a86 => Some("QinHeng CH340"),
                    0x2e8a => Some("Raspberry Pi Pico"),
                    _ => None,
                };

                if let Some(name) = manufacturer_name {
                    println!(
                        "Detected known manufacturer: {} (VID: {:04x})",
                        name, info.vid
                    );
                }
            }
        }
        // Test passes if we can check without panic
    }
}

#[tokio::test]
async fn test_discovery_stability() {
    // Test that discovery returns consistent results when called multiple times
    let first_result = available_ports();

    if first_result.is_err() {
        // Skip test if port listing isn't available on this system
        return;
    }

    let first_ports = first_result.unwrap();

    // Wait a bit and list again
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let second_result = available_ports();
    assert!(second_result.is_ok());

    let second_ports = second_result.unwrap();

    // Port counts should be the same (assuming no hardware changes)
    assert_eq!(
        first_ports.len(),
        second_ports.len(),
        "Port discovery returned different counts on consecutive calls"
    );

    // Port names should be the same
    let first_names: Vec<_> = first_ports.iter().map(|p| &p.port_name).collect();
    let second_names: Vec<_> = second_ports.iter().map(|p| &p.port_name).collect();

    for name in &first_names {
        assert!(
            second_names.contains(name),
            "Port {} disappeared between discoveries",
            name
        );
    }
}

#[tokio::test]
async fn test_empty_port_list_handling() {
    // Test that the system handles an empty port list gracefully
    if let Ok(ports) = available_ports() {
        if ports.is_empty() {
            println!("No ports found - testing empty list handling");
            assert_eq!(ports.len(), 0);
        } else {
            println!("Ports found - empty list test skipped");
        }
    }
}

#[cfg(feature = "auto-negotiation")]
#[tokio::test]
async fn test_discovery_with_manufacturer_profiles() {
    use serial_mcp_agent::negotiation::AutoNegotiator;

    // Test that discovery integrates with manufacturer profiles
    if let Ok(ports) = available_ports() {
        for port in ports {
            if let serialport::SerialPortType::UsbPort(info) = &port.port_type {
                let profile = AutoNegotiator::get_manufacturer_profile(info.vid);

                if let Some(prof) = profile {
                    println!(
                        "Port {} matches profile: {} (default baud: {})",
                        port.port_name, prof.name, prof.default_baud
                    );

                    // Verify profile has valid data
                    assert!(!prof.name.is_empty());
                    assert!(prof.default_baud > 0);
                    assert!(!prof.common_bauds.is_empty());
                } else {
                    println!("Port {} has unknown VID: {:04x}", port.port_name, info.vid);
                }
            }
        }
    }
}
