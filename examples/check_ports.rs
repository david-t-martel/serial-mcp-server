//! Check available serial ports on the system.
//!
//! This utility displays all available serial ports and their properties,
//! which is useful for hardware testing and debugging.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example check_ports
//! ```

use serialport::{available_ports, SerialPortType};

fn main() {
    println!("Serial Port Detection Utility");
    println!("{:=<70}", "");
    println!();

    match available_ports() {
        Ok(ports) => {
            if ports.is_empty() {
                println!("❌ No serial ports detected on this system");
                println!();
                println!("This could mean:");
                println!("  - No serial devices are connected");
                println!("  - USB-to-serial drivers are not installed");
                println!("  - Insufficient permissions to access serial ports");
                return;
            }

            println!("✅ Found {} serial port(s):", ports.len());
            println!();

            for (idx, port) in ports.iter().enumerate() {
                println!("{}. {}", idx + 1, port.port_name);
                println!("{:-<70}", "");

                match &port.port_type {
                    SerialPortType::UsbPort(usb_info) => {
                        println!("   Type:         USB Serial Port");
                        println!("   VID:          0x{:04X}", usb_info.vid);
                        println!("   PID:          0x{:04X}", usb_info.pid);

                        if let Some(ref manufacturer) = usb_info.manufacturer {
                            println!("   Manufacturer: {}", manufacturer);
                        }

                        if let Some(ref product) = usb_info.product {
                            println!("   Product:      {}", product);
                        }

                        if let Some(ref serial) = usb_info.serial_number {
                            println!("   Serial#:      {}", serial);
                        }

                        // Check against known manufacturers
                        #[cfg(feature = "auto-negotiation")]
                        {
                            use serial_mcp_agent::negotiation::AutoNegotiator;

                            if let Some(profile) =
                                AutoNegotiator::get_manufacturer_profile(usb_info.vid)
                            {
                                println!();
                                println!("   📋 Known Device Profile:");
                                println!("      Name:         {}", profile.name);
                                println!("      Default Baud: {}", profile.default_baud);
                                println!("      Common Bauds: {:?}", profile.common_bauds);
                            }
                        }
                    }
                    SerialPortType::BluetoothPort => {
                        println!("   Type: Bluetooth Serial Port");
                    }
                    SerialPortType::PciPort => {
                        println!("   Type: PCI Serial Port");
                    }
                    SerialPortType::Unknown => {
                        println!("   Type: Unknown");
                    }
                }

                println!();
            }

            // Print usage instructions for testing
            println!("{:=<70}", "");
            println!("Hardware Testing Instructions:");
            println!("{:=<70}", "");
            println!();
            println!("To run hardware tests with a specific port:");
            println!();

            if !ports.is_empty() {
                println!("  # Windows:");
                println!("  set TEST_PORT={}", ports[0].port_name);
                println!("  set TEST_BAUD=9600");
                println!("  cargo test --features auto-negotiation --ignored");
                println!();
                println!("  # Linux/macOS:");
                println!("  export TEST_PORT={}", ports[0].port_name);
                println!("  export TEST_BAUD=9600");
                println!("  cargo test --features auto-negotiation --ignored");
                println!();
            }

            println!("For loopback tests (requires TX-RX connected):");
            println!();
            println!("  # Windows:");
            println!("  set TEST_LOOPBACK=1");
            println!();
            println!("  # Linux/macOS:");
            println!("  export TEST_LOOPBACK=1");
            println!();

            #[cfg(feature = "auto-negotiation")]
            {
                println!("{:=<70}", "");
                println!("Known Manufacturer Profiles:");
                println!("{:=<70}", "");
                println!();

                use serial_mcp_agent::negotiation::AutoNegotiator;

                for profile in AutoNegotiator::all_manufacturer_profiles() {
                    println!(
                        "  0x{:04X} - {} (default: {} baud)",
                        profile.vid, profile.name, profile.default_baud
                    );
                }
            }
        }
        Err(e) => {
            println!("❌ Error detecting serial ports: {}", e);
            println!();
            println!("Possible causes:");
            println!("  - Insufficient permissions");
            println!("  - Serial port drivers not installed");
            println!("  - System API not available");
        }
    }
}
