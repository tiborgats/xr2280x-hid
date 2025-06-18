//! Example demonstrating flexible I2C timeout configuration.
//!
//! This example shows how to use different timeout values for different
//! I2C operations based on device characteristics and use case requirements.

use hidapi::HidApi;
use std::time::Instant;
use xr2280x_hid::{timeouts, Error, Result, Xr2280x};

fn main() -> Result<()> {
    env_logger::init();
    let hid_api = HidApi::new().map_err(Error::Hid)?;
    let device = Xr2280x::device_open_first(&hid_api)?;

    println!("XR2280x I2C Timeout Configuration Examples");
    println!("===========================================\n");

    // Example 1: Fast device scanning with optimized timeouts
    fast_device_scanning(&device)?;

    // Example 2: Working with responsive sensors
    responsive_sensor_example(&device)?;

    // Example 3: Working with slow EEPROMs
    slow_eeprom_example(&device)?;

    // Example 4: Custom timeout for specific requirements
    custom_timeout_example(&device)?;

    // Example 5: Demonstrating stuck bus detection
    stuck_bus_detection_example(&device)?;

    Ok(())
}

/// Example 1: Fast device scanning for responsive devices
fn fast_device_scanning(device: &Xr2280x) -> Result<()> {
    println!("1. Fast Device Scanning");
    println!("   Using {} ms timeout for rapid discovery", timeouts::SCAN);

    let start = Instant::now();

    // Use the default fast scan
    let devices = device.i2c_scan_default()?;

    let duration = start.elapsed();
    println!("   Found {} devices in {:?}", devices.len(), duration);

    for addr in devices {
        println!("     - Device at 0x{:02X}", addr);
    }

    // For even faster scanning of known responsive devices
    println!("\n   Ultra-fast scan (10ms timeout):");
    let start = Instant::now();
    let _devices =
        device.i2c_scan_with_progress_and_timeout(0x48, 0x4F, 10, |addr, found, _, _| {
            if found {
                println!("     Quick response from 0x{:02X}", addr);
            }
        })?;
    let duration = start.elapsed();
    println!("   Scanned sensor range in {:?}", duration);

    println!();
    Ok(())
}

/// Example 2: Working with responsive sensors (temperature, accelerometer, etc.)
fn responsive_sensor_example(device: &Xr2280x) -> Result<()> {
    println!("2. Responsive Sensor Communication");
    println!(
        "   Using {} ms timeout for quick sensor reads",
        timeouts::READ
    );

    // Simulate reading from a temperature sensor at 0x48
    let temp_sensor_addr = 0x48;
    let mut temp_data = [0u8; 2];

    match device.i2c_read_7bit(temp_sensor_addr, &mut temp_data) {
        Ok(_) => {
            println!("   ✓ Temperature sensor read successful");
            println!("     Raw data: 0x{:02X}{:02X}", temp_data[0], temp_data[1]);
        }
        Err(Error::I2cNack { .. }) => {
            println!(
                "   - No temperature sensor found at 0x{:02X}",
                temp_sensor_addr
            );
        }
        Err(e) => {
            println!("   ✗ Error reading temperature sensor: {}", e);
        }
    }

    // For ultra-responsive applications, use custom short timeout
    println!("\n   Ultra-fast sensor polling (25ms timeout):");
    match device.i2c_read_7bit_with_timeout(temp_sensor_addr, &mut temp_data, timeouts::SCAN) {
        Ok(_) => {
            println!("   ✓ Fast sensor read successful");
        }
        Err(Error::I2cNack { .. }) => {
            println!("   - Sensor not responding");
        }
        Err(e) => {
            println!("   ✗ Fast read error: {}", e);
        }
    }

    println!();
    Ok(())
}

/// Example 3: Working with slow EEPROMs that need long timeouts
fn slow_eeprom_example(device: &Xr2280x) -> Result<()> {
    println!("3. Slow EEPROM Communication");
    println!(
        "   Using {} ms timeout for EEPROM operations",
        timeouts::EEPROM_WRITE
    );

    let eeprom_addr = 0x50;
    let test_data = [0x12, 0x34, 0x56, 0x78];

    // Use the dedicated EEPROM write method with long timeout
    println!("   Writing to EEPROM with extended timeout...");
    let start = Instant::now();

    match device.i2c_eeprom_write_7bit(eeprom_addr, &test_data) {
        Ok(_) => {
            let duration = start.elapsed();
            println!("   ✓ EEPROM write completed in {:?}", duration);
        }
        Err(Error::I2cNack { .. }) => {
            println!("   - No EEPROM found at 0x{:02X}", eeprom_addr);
        }
        Err(Error::I2cTimeout { .. }) => {
            println!("   ✗ EEPROM write timeout - device may be busy or stuck");
        }
        Err(e) => {
            println!("   ✗ EEPROM write error: {}", e);
        }
    }

    // Reading from EEPROM (faster operation)
    let mut read_data = [0u8; 4];
    match device.i2c_read_7bit(eeprom_addr, &mut read_data) {
        Ok(_) => {
            println!("   ✓ EEPROM read: {:02X?}", read_data);
        }
        Err(Error::I2cNack { .. }) => {
            println!("   - EEPROM read: device not responding");
        }
        Err(e) => {
            println!("   ✗ EEPROM read error: {}", e);
        }
    }

    println!();
    Ok(())
}

/// Example 4: Custom timeout for specific application requirements
fn custom_timeout_example(device: &Xr2280x) -> Result<()> {
    println!("4. Custom Timeout Configuration");

    // Example: Real-time application requiring guaranteed response within 50ms
    println!("   Real-time constraint (50ms max):");
    let rt_timeout = 50;
    let sensor_addr = 0x68; // IMU/gyroscope address
    let mut imu_data = [0u8; 6];

    match device.i2c_read_7bit_with_timeout(sensor_addr, &mut imu_data, rt_timeout) {
        Ok(_) => {
            println!("   ✓ Real-time IMU read within {}ms", rt_timeout);
        }
        Err(Error::I2cTimeout { .. }) => {
            println!("   ✗ IMU failed real-time constraint (>{}ms)", rt_timeout);
        }
        Err(Error::I2cNack { .. }) => {
            println!("   - No IMU found at 0x{:02X}", sensor_addr);
        }
        Err(e) => {
            println!("   ✗ IMU error: {}", e);
        }
    }

    // Example: Tolerant application allowing up to 2 seconds
    println!("\n   Tolerant application (2000ms max):");
    let tolerant_timeout = 2000;
    let slow_device_addr = 0x60;
    let test_data = [0xFF];

    match device.i2c_write_7bit_with_timeout(slow_device_addr, &test_data, tolerant_timeout) {
        Ok(_) => {
            println!("   ✓ Slow device responded within {}ms", tolerant_timeout);
        }
        Err(Error::I2cTimeout { .. }) => {
            println!("   ✗ Device too slow (>{}ms)", tolerant_timeout);
        }
        Err(Error::I2cNack { .. }) => {
            println!("   - No device found at 0x{:02X}", slow_device_addr);
        }
        Err(e) => {
            println!("   ✗ Slow device error: {}", e);
        }
    }

    println!();
    Ok(())
}

/// Example 5: Demonstrating stuck bus detection
fn stuck_bus_detection_example(device: &Xr2280x) -> Result<()> {
    println!("5. Stuck Bus Detection");
    println!("   The scan function includes automatic stuck bus detection");
    println!("   If multiple consecutive timeouts occur, it will:");
    println!("   - Detect potential stuck bus condition");
    println!("   - Try extended timeout to confirm");
    println!("   - Return error if bus is truly stuck");

    // Perform a scan that would detect stuck bus conditions
    println!("\n   Performing scan with stuck bus detection...");
    let start = Instant::now();

    match device.i2c_scan_with_progress_and_timeout(
        0x08,
        0x77,
        timeouts::SCAN,
        |addr, found, idx, total| {
            if idx % 20 == 0 {
                println!("   Progress: {}/{} addresses scanned", idx, total);
            }
            if found {
                println!("   Found device at 0x{:02X}", addr);
            }
        },
    ) {
        Ok(devices) => {
            let duration = start.elapsed();
            println!("   ✓ Scan completed in {:?}", duration);
            println!("     Found {} devices total", devices.len());
        }
        Err(Error::I2cTimeout { address }) => {
            println!("   ✗ Stuck bus detected at address {}", address);
            println!("     This typically means:");
            println!("     - A device is holding SDA/SCL low");
            println!("     - Power supply issues");
            println!("     - Hardware malfunction");
            println!("   Recommendation: Check hardware connections and power");
        }
        Err(e) => {
            println!("   ✗ Scan error: {}", e);
        }
    }

    println!();
    Ok(())
}
