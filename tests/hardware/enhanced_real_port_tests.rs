//! Enhanced hardware tests with comprehensive coverage.
//!
//! These tests expand on the basic real_port_tests.rs with additional
//! edge cases, error handling, and stress testing scenarios.

#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::assertions_on_constants)]

use serial_mcp_agent::port::{
    DataBits, FlowControl, Parity, SerialPortAdapter, StopBits, SyncSerialPort,
};
use std::time::Duration;

use crate::hardware::utils::{
    assert_duration_within, print_available_ports, PortTestFixture, TestPortConfig, TimingHelper,
};

#[test]
#[ignore] // Requires hardware
fn test_real_port_write_performance() {
    let mut fixture = match PortTestFixture::setup() {
        Some(f) => f,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            print_available_ports();
            return;
        }
    };

    println!("Testing write performance on: {}", fixture.port_name());

    fixture.clear_buffers().expect("Failed to clear buffers");

    // Test data - 1KB
    let test_data = vec![0x55u8; 1024];
    let iterations = 100;

    let timer = TimingHelper::new(&format!(
        "Writing {} x {} bytes",
        iterations,
        test_data.len()
    ));

    let mut total_written = 0usize;
    for _ in 0..iterations {
        let written = fixture
            .port_mut()
            .write_bytes(&test_data)
            .expect("Write failed");
        total_written += written;
    }

    let elapsed = timer.finish();

    let bytes_per_sec = (total_written as f64 / elapsed.as_secs_f64()) as u64;
    let kb_per_sec = bytes_per_sec / 1024;

    println!("Total written: {} bytes", total_written);
    println!("Throughput: {} KB/s", kb_per_sec);
    println!(
        "Average time per write: {:?}",
        elapsed / (iterations as u32)
    );

    assert_eq!(total_written, test_data.len() * iterations);

    // At minimum baud rate (9600), we should get at least 960 bytes/sec
    // (accounting for overhead)
    let min_expected = (fixture.baud_rate() / 10) as u64 * 80 / 100; // 80% of theoretical
    assert!(
        bytes_per_sec >= min_expected,
        "Throughput too low: {} B/s (expected at least {} B/s)",
        bytes_per_sec,
        min_expected
    );
}

#[test]
#[ignore] // Requires loopback hardware
fn test_real_port_loopback_large_data() {
    let mut fixture = match PortTestFixture::setup() {
        Some(f) => f,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    if !fixture.is_loopback() {
        println!("⏭️  Skipping: TEST_LOOPBACK not set to 1");
        println!("   This test requires a loopback adapter");
        return;
    }

    println!(
        "Testing loopback with large data on: {}",
        fixture.port_name()
    );

    fixture.clear_buffers().expect("Failed to clear buffers");

    // Test with increasing data sizes
    let sizes = vec![16, 64, 256, 1024, 4096];

    for size in sizes {
        println!("  Testing {} bytes...", size);

        let test_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        // Write data
        let written = fixture
            .port_mut()
            .write_bytes(&test_data)
            .expect("Write failed");
        assert_eq!(written, test_data.len());

        // Wait for loopback
        let wait_ms = (size as u64 * 10_000) / (fixture.baud_rate() as u64);
        std::thread::sleep(Duration::from_millis(wait_ms.max(10)));

        // Read back
        let mut buffer = vec![0u8; size + 100]; // Extra space
        let read = fixture
            .port_mut()
            .read_bytes(&mut buffer)
            .expect("Read failed");

        assert!(read > 0, "Should read at least some bytes");
        assert_eq!(
            &buffer[..read],
            &test_data[..read],
            "Data mismatch at size {}",
            size
        );

        println!("    ✅ {} bytes OK", size);

        // Clear for next iteration
        fixture.clear_buffers().expect("Failed to clear buffers");
        std::thread::sleep(Duration::from_millis(50));
    }

    println!("✅ All loopback sizes passed");
}

#[test]
#[ignore] // Requires hardware
fn test_real_port_configuration_variations() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    println!("Testing configuration variations on: {}", config.port_name);

    // Test different data bits
    let data_bits_options = vec![DataBits::Seven, DataBits::Eight];

    // Test different parity options
    let parity_options = vec![Parity::None, Parity::Even, Parity::Odd];

    // Test different stop bits
    let stop_bits_options = vec![StopBits::One, StopBits::Two];

    let mut success_count = 0;
    let mut total_count = 0;

    for data_bits in &data_bits_options {
        for parity in &parity_options {
            for stop_bits in &stop_bits_options {
                total_count += 1;

                let mut port_config = config.to_port_config();
                port_config.data_bits = *data_bits;
                port_config.parity = *parity;
                port_config.stop_bits = *stop_bits;

                println!("  Trying: {:?}, {:?}, {:?}", data_bits, parity, stop_bits);

                match SyncSerialPort::open(&config.port_name, port_config) {
                    Ok(_port) => {
                        success_count += 1;
                        println!("    ✅ OK");
                    }
                    Err(e) => {
                        println!("    ❌ Failed: {}", e);
                    }
                }

                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    println!(
        "\nSuccessfully configured: {}/{} combinations",
        success_count, total_count
    );

    // Should be able to open with at least some configurations
    assert!(
        success_count > 0,
        "Failed to open port with any configuration"
    );
}

#[test]
#[ignore] // Requires hardware
fn test_real_port_timeout_precision() {
    let mut fixture = match PortTestFixture::setup() {
        Some(f) => f,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    println!("Testing timeout precision on: {}", fixture.port_name());

    fixture.clear_buffers().expect("Failed to clear buffers");

    // Test different timeout values
    let timeout_values = vec![50, 100, 250, 500, 1000];

    for timeout_ms in timeout_values {
        println!("  Testing {} ms timeout...", timeout_ms);

        // Reopen with specific timeout
        let config = TestPortConfig::from_env().unwrap();
        let mut port_config = config.to_port_config();
        port_config.timeout = Duration::from_millis(timeout_ms);

        let mut port =
            SyncSerialPort::open(&config.port_name, port_config).expect("Failed to open port");

        port.clear_buffers().expect("Failed to clear buffers");

        // Try to read when no data available
        let mut buffer = [0u8; 100];
        let start = std::time::Instant::now();
        let _ = port.read_bytes(&mut buffer);
        let elapsed = start.elapsed();

        println!(
            "    Expected: {} ms, Actual: {} ms",
            timeout_ms,
            elapsed.as_millis()
        );

        // Allow 50% tolerance due to OS scheduling
        assert_duration_within(
            elapsed,
            Duration::from_millis(timeout_ms),
            Duration::from_millis(timeout_ms / 2),
            &format!("Timeout precision for {} ms", timeout_ms),
        );
    }

    println!("✅ Timeout precision test passed");
}

#[test]
#[ignore] // Requires hardware
fn test_real_port_rapid_open_close() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    println!("Testing rapid open/close on: {}", config.port_name);

    let iterations = 50;
    let port_config = config.to_port_config();

    let timer = TimingHelper::new(&format!("Rapid open/close {} iterations", iterations));

    for i in 0..iterations {
        let port = SyncSerialPort::open(&config.port_name, port_config.clone());

        match port {
            Ok(p) => {
                // Immediately close by dropping
                drop(p);
            }
            Err(e) => {
                println!("❌ Failed on iteration {}: {}", i + 1, e);
                panic!("Port open failed on iteration {}", i + 1);
            }
        }

        // Minimal delay
        std::thread::sleep(Duration::from_millis(10));
    }

    let elapsed = timer.finish();
    let avg_time = elapsed / (iterations as u32);

    println!("Average time per cycle: {:?}", avg_time);

    // Should complete reasonably quickly
    assert!(
        elapsed < Duration::from_secs(10),
        "Rapid open/close took too long"
    );
}

#[test]
#[ignore] // Requires hardware
fn test_real_port_buffer_overflow_handling() {
    let mut fixture = match PortTestFixture::setup() {
        Some(f) => f,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    if !fixture.is_loopback() {
        println!("⏭️  Skipping: TEST_LOOPBACK not set to 1");
        return;
    }

    println!(
        "Testing buffer overflow handling on: {}",
        fixture.port_name()
    );

    fixture.clear_buffers().expect("Failed to clear buffers");

    // Write large amount of data
    let large_data = vec![0xAAu8; 8192];

    println!("  Writing {} bytes...", large_data.len());
    let written = fixture
        .port_mut()
        .write_bytes(&large_data)
        .expect("Write failed");

    println!("  Wrote {} bytes", written);

    // Wait for data to loop back
    std::thread::sleep(Duration::from_millis(100));

    // Try to read with small buffer
    let mut small_buffer = [0u8; 256];
    let mut total_read = 0usize;
    let mut iterations = 0;
    let max_iterations = 50;

    while iterations < max_iterations {
        match fixture.port_mut().read_bytes(&mut small_buffer) {
            Ok(n) if n > 0 => {
                total_read += n;
                println!("    Read {} bytes (total: {})", n, total_read);
                iterations += 1;
            }
            Ok(_) => {
                // No more data
                break;
            }
            Err(e) => {
                println!("    Read error: {}", e);
                break;
            }
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    println!("  Total read: {} bytes", total_read);
    println!(
        "  Data recovery: {:.1}%",
        (total_read as f64 / written as f64) * 100.0
    );

    // Should have read at least some data
    assert!(total_read > 0, "Should read at least some data");

    println!("✅ Buffer overflow handling test completed");
}

#[test]
#[ignore] // Requires hardware
fn test_real_port_flow_control_variations() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    println!("Testing flow control variations on: {}", config.port_name);

    let flow_control_options = vec![
        FlowControl::None,
        FlowControl::Software,
        FlowControl::Hardware,
    ];

    for flow_control in flow_control_options {
        println!("  Testing {:?}...", flow_control);

        let mut port_config = config.to_port_config();
        port_config.flow_control = flow_control;

        match SyncSerialPort::open(&config.port_name, port_config) {
            Ok(port) => {
                println!("    ✅ Opened with {:?}", flow_control);
                drop(port);
            }
            Err(e) => {
                println!("    ⚠️  Failed with {:?}: {}", flow_control, e);
                // Don't fail - some devices don't support all flow control modes
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    println!("✅ Flow control test completed");
}

#[test]
#[ignore] // Requires hardware
fn test_real_port_invalid_baud_rate() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    println!("Testing invalid baud rates on: {}", config.port_name);

    // Invalid/unusual baud rates
    let invalid_bauds = vec![1, 100, 1000000, 9999999];

    for baud in invalid_bauds {
        println!("  Trying invalid baud: {}...", baud);

        let mut port_config = config.to_port_config();
        port_config.baud_rate = baud;

        match SyncSerialPort::open(&config.port_name, port_config) {
            Ok(_) => {
                println!("    ⚠️  Unexpectedly opened with baud {}", baud);
                // Some platforms might allow unusual baud rates
            }
            Err(e) => {
                println!("    ✅ Correctly rejected: {}", e);
            }
        }
    }

    println!("✅ Invalid baud rate test completed");
}

#[tokio::test]
#[ignore] // Requires hardware
async fn test_real_port_stress_concurrent_operations() {
    let config = match TestPortConfig::from_env() {
        Some(c) => c,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    println!(
        "Testing stress with concurrent operations on: {}",
        config.port_name
    );

    let port_config = config.to_port_config();
    let port_name = config.port_name.clone();

    // Spawn multiple tasks trying to open the port
    let mut handles = vec![];

    for i in 0..5 {
        let port_name = port_name.clone();
        let port_config = port_config.clone();

        let handle = tokio::spawn(async move {
            let result = SyncSerialPort::open(&port_name, port_config);
            match result {
                Ok(_port) => {
                    println!("    Task {} opened port", i);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    Ok(())
                }
                Err(e) => {
                    println!("    Task {} failed to open: {}", i, e);
                    Err(e)
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks
    let results = futures::future::join_all(handles).await;

    let success_count = results
        .iter()
        .filter(|r| {
            r.as_ref()
                .ok()
                .and_then(|inner| inner.as_ref().ok())
                .is_some()
        })
        .count();

    println!("\nSuccessful opens: {}/5", success_count);

    // Only one should succeed (serial ports are exclusive)
    // But we don't fail if platform allows multiple opens
    if success_count > 1 {
        println!("⚠️  Platform allows multiple concurrent opens");
    }

    println!("✅ Concurrent stress test completed");
}

#[test]
#[ignore] // Requires hardware
fn test_real_port_clear_buffers_effectiveness() {
    let mut fixture = match PortTestFixture::setup() {
        Some(f) => f,
        None => {
            println!("⏭️  Skipping: TEST_PORT not set");
            return;
        }
    };

    if !fixture.is_loopback() {
        println!("⏭️  Skipping: TEST_LOOPBACK not set to 1");
        return;
    }

    println!(
        "Testing buffer clearing effectiveness on: {}",
        fixture.port_name()
    );

    // Write some data
    let test_data = b"BUFFER CLEAR TEST\r\n";
    fixture
        .port_mut()
        .write_bytes(test_data)
        .expect("Write failed");

    println!("  Wrote {} bytes", test_data.len());

    // Wait for loopback
    std::thread::sleep(Duration::from_millis(100));

    // Clear buffers
    println!("  Clearing buffers...");
    fixture.clear_buffers().expect("Failed to clear buffers");

    // Try to read - should get nothing or very little
    let mut buffer = [0u8; 256];
    let read = fixture.port_mut().read_bytes(&mut buffer).unwrap_or(0);

    println!("  Read after clear: {} bytes", read);

    // Should read 0 bytes if clear was effective
    // Allow some tolerance for race conditions
    assert!(
        read <= test_data.len() / 2,
        "Buffer clear ineffective: read {} bytes after clear",
        read
    );

    println!("✅ Buffer clear test passed");
}
