//! Port discovery and enumeration tests.
//!
//! These tests don't require specific hardware but will use any available
//! ports on the system. They are still marked as ignored because they require
//! at least some serial hardware to be meaningful.

#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::assertions_on_constants)]

use serial_mcp_agent::port::{PortConfiguration, SyncSerialPort};
use serialport::SerialPortType;
use std::collections::HashSet;
use std::time::Duration;

use crate::hardware::utils::{
    discover_available_ports, discover_usb_ports, get_port_info, print_available_ports,
};

#[test]
#[ignore] // Requires hardware
fn test_port_discovery() {
    println!("Testing port discovery...");

    let ports = discover_available_ports();

    if ports.is_empty() {
        println!("⚠️  No ports found - skipping test");
        println!("   This test requires at least one serial port");
        print_available_ports();
        return;
    }

    println!("✅ Found {} port(s)", ports.len());

    for port in &ports {
        println!("  - {} ({:?})", port.port_name, port.port_type);
    }

    assert!(!ports.is_empty());
}

#[test]
#[ignore] // Requires USB hardware
fn test_usb_port_discovery() {
    println!("Testing USB port discovery...");

    let usb_ports = discover_usb_ports();

    if usb_ports.is_empty() {
        println!("⚠️  No USB ports found");
        println!("   This test requires at least one USB serial device");
        print_available_ports();
        return;
    }

    println!("✅ Found {} USB port(s)", usb_ports.len());

    for port in &usb_ports {
        if let SerialPortType::UsbPort(usb_info) = &port.port_type {
            println!(
                "  - {} (VID:PID = {:04x}:{:04x})",
                port.port_name, usb_info.vid, usb_info.pid
            );
        }
    }

    assert!(!usb_ports.is_empty());
}

#[test]
#[ignore] // Requires hardware
fn test_port_name_uniqueness() {
    println!("Testing port name uniqueness...");

    let ports = discover_available_ports();

    if ports.is_empty() {
        println!("⚠️  No ports found - skipping test");
        return;
    }

    let mut port_names = HashSet::new();
    let mut duplicates = Vec::new();

    for port in &ports {
        if !port_names.insert(port.port_name.clone()) {
            duplicates.push(port.port_name.clone());
        }
    }

    if !duplicates.is_empty() {
        println!("❌ Found duplicate port names:");
        for name in &duplicates {
            println!("  - {}", name);
        }
        panic!("Port names should be unique");
    }

    println!("✅ All {} port names are unique", port_names.len());
}

#[test]
#[ignore] // Requires hardware
fn test_port_info_retrieval() {
    println!("Testing port info retrieval...");

    let ports = discover_available_ports();

    if ports.is_empty() {
        println!("⚠️  No ports found - skipping test");
        return;
    }

    let first_port = &ports[0];
    println!("Testing with port: {}", first_port.port_name);

    let info = get_port_info(&first_port.port_name);

    assert!(info.is_some(), "Should be able to retrieve port info");

    let info = info.unwrap();
    assert_eq!(info.port_name, first_port.port_name);

    println!("✅ Port info retrieved successfully");
}

#[test]
#[ignore] // Requires hardware
fn test_open_all_available_ports() {
    println!("Testing opening all available ports...");

    let ports = discover_available_ports();

    if ports.is_empty() {
        println!("⚠️  No ports found - skipping test");
        return;
    }

    println!("Found {} port(s), attempting to open each...", ports.len());

    let mut config = PortConfiguration::default();
    config.baud_rate = 9600;
    config.timeout = Duration::from_millis(1000);

    let mut success_count = 0;
    let mut failed_ports = Vec::new();

    for port_info in &ports {
        print!("  Opening {}... ", port_info.port_name);

        match SyncSerialPort::open(&port_info.port_name, config.clone()) {
            Ok(port) => {
                println!("✅");
                success_count += 1;
                drop(port); // Close immediately
            }
            Err(e) => {
                println!("❌ {}", e);
                failed_ports.push((port_info.port_name.clone(), e.to_string()));
            }
        }

        // Small delay between opens
        std::thread::sleep(Duration::from_millis(100));
    }

    println!(
        "\nResults: {}/{} ports opened successfully",
        success_count,
        ports.len()
    );

    if !failed_ports.is_empty() {
        println!("\nFailed ports:");
        for (port, error) in &failed_ports {
            println!("  - {}: {}", port, error);
        }
        println!("Note: Some failures are expected if ports are in use");
    }

    // Should be able to open at least one port
    assert!(
        success_count > 0,
        "Should be able to open at least one port"
    );
}

#[test]
#[ignore] // Requires USB hardware
fn test_usb_vid_detection() {
    println!("Testing USB VID detection...");

    let usb_ports = discover_usb_ports();

    if usb_ports.is_empty() {
        println!("⚠️  No USB ports found");
        return;
    }

    println!("✅ Found {} USB port(s)", usb_ports.len());

    for port in &usb_ports {
        if let SerialPortType::UsbPort(usb_info) = &port.port_type {
            println!(
                "  {} - VID: 0x{:04x}, PID: 0x{:04x}",
                port.port_name, usb_info.vid, usb_info.pid
            );

            // VID should be non-zero for valid USB devices
            assert!(usb_info.vid != 0, "VID should not be zero");

            // Check if we have manufacturer info
            if let Some(ref manufacturer) = usb_info.manufacturer {
                println!("    Manufacturer: {}", manufacturer);
                assert!(!manufacturer.is_empty());
            }

            // Check if we have product info
            if let Some(ref product) = usb_info.product {
                println!("    Product: {}", product);
                assert!(!product.is_empty());
            }
        }
    }
}

#[test]
#[ignore] // Requires hardware
fn test_port_type_detection() {
    println!("Testing port type detection...");

    let ports = discover_available_ports();

    if ports.is_empty() {
        println!("⚠️  No ports found - skipping test");
        return;
    }

    println!("Port types found:");

    let mut usb_count = 0;
    let mut bluetooth_count = 0;
    let mut pci_count = 0;
    let mut unknown_count = 0;

    for port in &ports {
        match &port.port_type {
            SerialPortType::UsbPort(_) => {
                println!("  {} - USB", port.port_name);
                usb_count += 1;
            }
            SerialPortType::BluetoothPort => {
                println!("  {} - Bluetooth", port.port_name);
                bluetooth_count += 1;
            }
            SerialPortType::PciPort => {
                println!("  {} - PCI", port.port_name);
                pci_count += 1;
            }
            SerialPortType::Unknown => {
                println!("  {} - Unknown", port.port_name);
                unknown_count += 1;
            }
        }
    }

    println!("\nSummary:");
    println!("  USB:       {}", usb_count);
    println!("  Bluetooth: {}", bluetooth_count);
    println!("  PCI:       {}", pci_count);
    println!("  Unknown:   {}", unknown_count);

    assert_eq!(
        usb_count + bluetooth_count + pci_count + unknown_count,
        ports.len()
    );
}

#[test]
#[ignore] // Requires hardware
fn test_repeated_discovery() {
    println!("Testing repeated port discovery...");

    let iterations = 10;
    let mut all_results = Vec::new();

    for i in 1..=iterations {
        let ports = discover_available_ports();
        all_results.push(ports);

        if i % 3 == 0 {
            println!(
                "  Iteration {}/{}: {} ports",
                i,
                iterations,
                all_results.last().unwrap().len()
            );
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    // All iterations should return the same number of ports
    let first_count = all_results[0].len();

    for (i, result) in all_results.iter().enumerate() {
        assert_eq!(
            result.len(),
            first_count,
            "Iteration {} returned different port count: {} vs {}",
            i + 1,
            result.len(),
            first_count
        );
    }

    println!(
        "✅ All {} iterations returned consistent results ({} ports)",
        iterations, first_count
    );
}

#[test]
#[ignore] // Requires hardware
fn test_port_metadata_consistency() {
    println!("Testing port metadata consistency...");

    let ports = discover_available_ports();

    if ports.is_empty() {
        println!("⚠️  No ports found - skipping test");
        return;
    }

    for port in &ports {
        // Port name should not be empty
        assert!(!port.port_name.is_empty(), "Port name should not be empty");

        // Port name should be valid
        #[cfg(windows)]
        {
            assert!(
                port.port_name.starts_with("COM"),
                "Windows port should start with COM: {}",
                port.port_name
            );
        }

        #[cfg(unix)]
        {
            assert!(
                port.port_name.starts_with("/dev/"),
                "Unix port should start with /dev/: {}",
                port.port_name
            );
        }

        // USB ports should have valid VID/PID
        if let SerialPortType::UsbPort(usb_info) = &port.port_type {
            assert!(
                usb_info.vid != 0 || usb_info.pid != 0,
                "USB device should have non-zero VID or PID"
            );
        }

        println!("  ✅ {} - metadata OK", port.port_name);
    }

    println!("✅ All port metadata is consistent");
}
