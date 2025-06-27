//! Demonstration of improved I2C error messages.
//!
//! This example shows what different I2C errors look like and demonstrates
//! the helpful guidance provided by the new error messages.
//!
//! Run this to see the improved error handling in action.

use hidapi::HidApi;
use std::time::Instant;
use xr2280x_hid::{Error, Result, Xr2280x, timeouts};

fn main() -> Result<()> {
    env_logger::init();

    println!("XR2280x I2C Error Message Demonstration");
    println!("=======================================\n");

    let hid_api = HidApi::new().map_err(Error::Hid)?;
    let device = Xr2280x::device_open_first(&hid_api)?;

    println!("This demo shows what different I2C errors look like with");
    println!("the new user-friendly error messages.\n");

    // Demo 1: Normal NACK (device not found)
    demo_nack_error(&device);

    // Demo 2: Timeout error (if present)
    demo_timeout_error(&device);

    // Demo 3: Arbitration lost (if it occurs)
    demo_arbitration_error(&device);

    // Demo 4: Request error (invalid parameters)
    demo_request_error(&device);

    // Demo 5: Quick scan to show normal operation
    demo_normal_scan(&device);

    println!("\n=== ERROR MESSAGE SUMMARY ===");
    println!("✓ I2cNack: Normal during scanning - just means no device at that address");
    println!("⚠ I2cTimeout: Hardware issue - check device power and connections");
    println!("⚠ I2cArbitrationLost: Bus contention - check for interference or multiple masters");
    println!("✗ I2cRequestError: Software bug - fix your code parameters");
    println!("⚠ I2cUnknownError: Firmware issue - power cycle the XR2280x device");
    println!("\nThe new system provides clear guidance for each error type!");

    Ok(())
}

fn demo_nack_error(device: &Xr2280x) {
    println!("=== Demo 1: I2cNack (Normal - No Device Found) ===");

    let test_addr = 0x77; // Try an address that probably has no device
    let mut buffer = [0u8; 1];

    println!("Trying to read from address 0x{test_addr:02X} (probably no device there)....");

    match device.i2c_read_7bit_with_timeout(test_addr, &mut buffer, timeouts::READ) {
        Ok(_) => {
            println!("✓ Unexpected - device found at 0x{test_addr:02X}!");
        }
        Err(Error::I2cNack { address }) => {
            let nack_error = Error::I2cNack { address };
            println!("✓ Expected NACK error:");
            println!("   {nack_error}");
            println!("   → This is NORMAL when scanning - just means no device at this address");
        }
        Err(e) => {
            println!("✗ Different error: {e}");
        }
    }
    println!();
}

fn demo_timeout_error(device: &Xr2280x) {
    println!("=== Demo 2: I2cTimeout (Hardware Issue) ===");

    println!("Testing with ultra-short timeout to potentially trigger timeout...");

    // Try a very short timeout that might cause timeout on slow/stuck devices
    let test_addr = 0x50; // Common EEPROM address
    let mut buffer = [0u8; 1];

    match device.i2c_read_7bit_with_timeout(test_addr, &mut buffer, 1) {
        // 1ms timeout
        Ok(_) => {
            println!("✓ Device responded very quickly at 0x{test_addr:02X}");
        }
        Err(Error::I2cTimeout { address }) => {
            let timeout_error = Error::I2cTimeout { address };
            println!("⚠ Timeout error (this demonstrates the improved message):");
            println!("   {timeout_error}");
            println!("   → This provides clear guidance on what to check!");
        }
        Err(Error::I2cNack { .. }) => {
            println!("- No device at 0x{test_addr:02X} (normal)");
        }
        Err(e) => {
            println!("✗ Different error: {e}");
        }
    }
    println!();
}

fn demo_arbitration_error(_device: &Xr2280x) {
    println!("=== Demo 3: I2cArbitrationLost (Bus Contention) ===");

    println!("Arbitration lost errors occur when there's bus contention.");
    println!("This is hard to trigger artificially, but if it happens, you'll see:");
    println!();
    println!("Example error message:");
    let example_addr = xr2280x_hid::I2cAddress::new_7bit(0x48).unwrap();
    let example_error = Error::I2cArbitrationLost {
        address: example_addr,
    };
    println!("   {example_error}");
    println!("   → Provides specific troubleshooting steps for bus conflicts!");
    println!();
}

fn demo_request_error(_device: &Xr2280x) {
    println!("=== Demo 4: I2cRequestError (Software Bug) ===");

    println!("Request errors happen when you pass invalid parameters to the firmware.");
    println!("Example: trying to send more than 32 bytes at once");
    println!();
    println!("If this occurred, you'd see something like:");
    let example_addr = xr2280x_hid::I2cAddress::new_7bit(0x48).unwrap();
    let example_error = Error::I2cRequestError {
        address: example_addr,
    };
    println!("   {example_error}");
    println!("   → Tells you exactly what parameters to check!");
    println!();
}

fn demo_normal_scan(device: &Xr2280x) {
    println!("=== Demo 5: Normal I2C Bus Scan ===");

    println!("Performing a normal I2C scan to show typical operation...");
    let start = Instant::now();

    match device.i2c_scan_with_progress_and_timeout(
        0x48,
        0x4F,
        timeouts::SCAN,
        |addr, found, idx, _total| {
            if found {
                println!("   Device found at 0x{addr:02X}");
            }
            if idx == 0 {
                println!("   Scanning sensor address range (0x48-0x4F)...");
            }
        },
    ) {
        Ok(devices) => {
            let duration = start.elapsed();
            println!("✓ Scan completed in {duration:?}");
            if devices.is_empty() {
                println!("   No devices found in range 0x48-0x4F (this is normal)");
            } else {
                println!(
                    "   Found {len} device(s): {devices:02X?}",
                    len = devices.len()
                );
            }
        }
        Err(Error::I2cTimeout { address }) => {
            let duration = start.elapsed();
            let timeout_error = Error::I2cTimeout { address };
            println!("⚠ Scan failed with timeout in {duration:?}:");
            println!("   {timeout_error}");
            println!("   → Notice how it provides helpful troubleshooting guidance!");

            if duration.as_secs() < 5 {
                println!("   ✓ GOOD: Failed quickly instead of hanging for 29+ seconds!");
            }
        }
        Err(Error::I2cArbitrationLost { address }) => {
            let arbitration_error = Error::I2cArbitrationLost { address };
            println!("⚠ Scan failed with arbitration lost:");
            println!("   {arbitration_error}");
            println!("   → Specific guidance for bus contention issues!");
        }
        Err(e) => {
            println!("✗ Scan failed: {e}");
        }
    }
    println!();
}
