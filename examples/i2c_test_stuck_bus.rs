//! Test program to verify fast stuck bus detection and timeout improvements.
//!
//! This example demonstrates the new robust timeout system that prevents
//! 29+ second hangs when unpowered devices hold I2C lines low.
//!
//! ERROR MEANINGS:
//! - I2cTimeout: Bus stuck (unpowered device) or very slow device  
//! - I2cArbitrationLost: Multiple I2C masters or electrical interference
//! - I2cNack: No device at address (normal during scanning)
//! - I2cRequestError: Invalid parameters sent to firmware

use hidapi::HidApi;
use std::time::Instant;
use xr2280x_hid::{timeouts, Error, Result, Xr2280x};

fn main() -> Result<()> {
    env_logger::init();

    println!("XR2280x Stuck Bus Detection Test");
    println!("================================\n");

    let hid_api = HidApi::new().map_err(Error::Hid)?;
    let device = Xr2280x::open_first(&hid_api)?;

    // Test 1: Normal operation with responsive firmware
    test_normal_operation(&device)?;

    // Test 2: Ultra-fast scanning
    test_ultra_fast_scanning(&device)?;

    // Test 3: Demonstrate timeout variations
    test_timeout_variations(&device)?;

    // Test 4: Stress test with different timeout values
    test_timeout_stress(&device)?;

    println!("All tests completed successfully!");
    println!("\n=== SUMMARY ===");
    println!("If you have an unpowered device connected that's holding");
    println!("I2C lines low, this program should fail quickly (within");
    println!("seconds) instead of hanging for 29+ seconds.");
    println!("\nERROR GUIDE:");
    println!("• I2cTimeout = Stuck bus (unpowered device) - Check device power");
    println!("• I2cArbitrationLost = Bus conflict - Check connections/interference");
    println!("• I2cNack = No device (normal) - Device not present at that address");
    println!("• Fast failure (<3 sec) = GOOD, prevents 29+ second hangs!");

    Ok(())
}

fn test_normal_operation(device: &Xr2280x) -> Result<()> {
    println!("Test 1: Normal Operation");
    println!("========================");

    let start = Instant::now();

    match device.i2c_scan_default() {
        Ok(devices) => {
            let duration = start.elapsed();
            println!("✓ Scan completed in {:?}", duration);
            println!("  Found {} devices:", devices.len());
            for addr in devices {
                println!("    - 0x{:02X}", addr);
            }

            if duration.as_secs() > 5 {
                println!("  WARNING: Scan took longer than expected");
                println!("  This might indicate a slow or problematic I2C bus");
            } else {
                println!("  Scan timing looks good!");
            }
        }
        Err(Error::I2cTimeout { address }) => {
            let duration = start.elapsed();
            println!("✗ Stuck bus detected at {} in {:?}", address, duration);
            println!("  CAUSE: Unpowered device holding I2C lines low, or very slow device");
            println!("  This is EXPECTED if you have unpowered I2C devices connected");

            if duration.as_secs() > 10 {
                println!("  WARNING: Detection took longer than expected!");
                return Err(Error::I2cTimeout { address });
            } else {
                println!("  ✓ Fast failure detection working correctly!");
                println!("  SOLUTION: Power all I2C devices or disconnect problematic ones");
            }
        }
        Err(Error::I2cArbitrationLost { address }) => {
            let duration = start.elapsed();
            println!("✗ Bus arbitration lost at {} in {:?}", address, duration);
            println!("  CAUSE: Multiple I2C masters competing or electrical interference");
            println!("  SOLUTIONS:");
            println!("    - Disconnect other I2C controllers");
            println!("    - Check for loose connections on SDA/SCL");
            println!("    - Reduce I2C speed: device.i2c_set_speed_khz(50)");
            println!("    - Use shorter wires or better shielding");
            return Err(Error::I2cArbitrationLost { address });
        }
        Err(e) => {
            println!("✗ Unexpected error: {}", e);
            return Err(e);
        }
    }

    println!();
    Ok(())
}

fn test_ultra_fast_scanning(device: &Xr2280x) -> Result<()> {
    println!("Test 2: Ultra-Fast Scanning");
    println!("===========================");

    // Test with 3ms timeout per address
    println!("Testing with 3ms timeout per address...");
    let start = Instant::now();

    let mut scan_progress = 0;
    let _total_addresses = (0x77u8 - 0x08u8 + 1) as usize;

    match device.i2c_scan_with_progress_and_timeout(
        0x08,
        0x77,
        timeouts::PROBE,
        |addr, found, idx, total| {
            scan_progress = idx + 1;
            if found {
                println!("  Quick device found at 0x{:02X}", addr);
            }
            if idx % 20 == 0 || found {
                println!("  Progress: {}/{} addresses", idx + 1, total);
            }
        },
    ) {
        Ok(devices) => {
            let duration = start.elapsed();
            println!("✓ Ultra-fast scan completed in {:?}", duration);
            println!(
                "  Scanned {} addresses, found {} devices",
                scan_progress,
                devices.len()
            );
            println!(
                "  Average time per address: {:?}",
                duration / scan_progress as u32
            );
        }
        Err(Error::I2cTimeout { address }) => {
            let duration = start.elapsed();
            println!(
                "✗ Stuck bus detected at {} after scanning {} addresses",
                address, scan_progress
            );
            println!("  Detection time: {:?}", duration);
            println!("  MEANING: I2C bus is stuck (device holding lines low)");

            if duration.as_secs() > 3 {
                println!("  WARNING: Should have failed faster!");
            } else {
                println!("  ✓ Excellent - failed very quickly!");
                println!("  This prevents the old 29+ second hangs!");
            }
        }
        Err(Error::I2cArbitrationLost { address }) => {
            let duration = start.elapsed();
            println!("✗ Bus arbitration lost at {} in {:?}", address, duration);
            println!("  MEANING: Multiple masters or electrical interference detected");
            println!("  TRY: Disconnect other I2C devices and check connections");
            return Err(Error::I2cArbitrationLost { address });
        }
        Err(e) => {
            println!("✗ Unexpected error: {}", e);
            return Err(e);
        }
    }

    println!();
    Ok(())
}

fn test_timeout_variations(device: &Xr2280x) -> Result<()> {
    println!("Test 3: Timeout Variations");
    println!("==========================");

    let test_addresses = [0x48, 0x50, 0x68, 0x77]; // Common sensor/EEPROM addresses

    for &addr in &test_addresses {
        println!("Testing address 0x{:02X}:", addr);

        // Test with different timeouts
        let timeouts_to_test = [
            (timeouts::PROBE, "PROBE (3ms)"),
            (timeouts::SCAN, "SCAN (8ms)"),
            (timeouts::READ, "READ (100ms)"),
        ];

        for (timeout_ms, name) in timeouts_to_test {
            let start = Instant::now();
            let mut test_buffer = [0u8; 1];

            match device.i2c_read_7bit_with_timeout(addr, &mut test_buffer, timeout_ms) {
                Ok(_) => {
                    let duration = start.elapsed();
                    println!("  ✓ {} - Device responded in {:?}", name, duration);
                }
                Err(Error::I2cNack { .. }) => {
                    let duration = start.elapsed();
                    println!("  - {} - No device (NACK in {:?})", name, duration);
                }
                Err(Error::I2cTimeout { .. }) => {
                    let duration = start.elapsed();
                    println!("  ✗ {} - Timeout in {:?}", name, duration);
                    println!("    MEANING: Device too slow or bus stuck");

                    // For very short timeouts, we expect quick failures
                    if timeout_ms <= 10 && duration.as_millis() > (timeout_ms as u128 * 3) {
                        println!("    WARNING: Timeout detection slower than expected");
                    }
                }
                Err(Error::I2cArbitrationLost { .. }) => {
                    let duration = start.elapsed();
                    println!("  ✗ {} - Arbitration lost in {:?}", name, duration);
                    println!("    MEANING: Bus contention detected");
                }
                Err(e) => {
                    println!("  ✗ {} - Error: {}", name, e);
                }
            }
        }
        println!();
    }

    Ok(())
}

fn test_timeout_stress(device: &Xr2280x) -> Result<()> {
    println!("Test 4: Timeout Stress Test");
    println!("===========================");

    println!("Performing multiple rapid scans to test consistency...");

    let mut total_time = std::time::Duration::new(0, 0);
    let num_iterations = 5;

    for i in 1..=num_iterations {
        println!("Iteration {}/{}:", i, num_iterations);
        let start = Instant::now();

        match device.i2c_scan_with_progress_and_timeout(
            0x08,
            0x20,
            timeouts::PROBE,
            |_, _, _, _| {},
        ) {
            Ok(devices) => {
                let duration = start.elapsed();
                total_time += duration;
                println!(
                    "  ✓ Completed in {:?}, found {} devices",
                    duration,
                    devices.len()
                );
            }
            Err(Error::I2cTimeout { address }) => {
                let duration = start.elapsed();
                total_time += duration;
                println!("  ✗ Stuck bus at {} in {:?}", address, duration);
                println!("    MEANING: I2C bus stuck (unpowered device holding lines)");

                if duration.as_secs() > 2 {
                    println!("    WARNING: Should have failed faster in stress test!");
                } else {
                    println!("    ✓ Fast failure - much better than 29+ second hangs!");
                }
                break; // Stop stress test if we hit stuck bus
            }
            Err(Error::I2cArbitrationLost { address }) => {
                let duration = start.elapsed();
                println!("  ✗ Arbitration lost at {} in {:?}", address, duration);
                println!("    MEANING: Bus interference or multiple masters");
                break;
            }
            Err(e) => {
                println!("  ✗ Error: {}", e);
                break;
            }
        }
    }

    let avg_time = total_time / num_iterations;
    println!("Average scan time: {:?}", avg_time);

    if avg_time.as_millis() > 500 {
        println!("WARNING: Average scan time seems high");
    } else {
        println!("✓ Scan performance looks good!");
    }

    println!();
    Ok(())
}
