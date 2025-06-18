//! Example demonstrating hardware device enumeration for XR2280x devices.
//!
//! This example shows how to enumerate XR2280x hardware devices, which groups
//! logical USB interfaces by serial number to present a unified view of each
//! physical device.
//!
//! Run with: cargo run --example enumerate_hardware

use hidapi::HidApi;
use xr2280x_hid::{device_find_all, Xr2280x};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("XR2280x Device Enumeration Example");
    println!("===================================\n");

    let hid_api = HidApi::new()?;

    println!("1. Device Enumeration:");
    println!("----------------------");
    let hardware_devices = device_find_all(&hid_api)?;
    if hardware_devices.is_empty() {
        println!("  No XR2280x hardware devices found.");
    } else {
        for (i, device_info) in hardware_devices.iter().enumerate() {
            println!("  Hardware Device [{}]:", i);
            println!(
                "    Serial Number: {}",
                device_info.serial_number.as_deref().unwrap_or("N/A")
            );
            println!(
                "    Product: {}",
                device_info.product_string.as_deref().unwrap_or("Unknown")
            );

            if let Some(ref i2c_interface) = device_info.i2c_interface {
                println!(
                    "    I2C Interface: Available (PID: 0x{:04X})",
                    i2c_interface.pid
                );
                println!("      Path: {:?}", i2c_interface.path);
            } else {
                println!("    I2C Interface: Not available");
            }

            if let Some(ref edge_interface) = device_info.edge_interface {
                println!(
                    "    EDGE Interface: Available (PID: 0x{:04X})",
                    edge_interface.pid
                );
                println!("      Path: {:?}", edge_interface.path);
            } else {
                println!("    EDGE Interface: Not available");
            }
            println!();
        }
    }

    // Demonstrate opening a device
    println!("2. Opening Device:");
    println!("------------------");
    if !hardware_devices.is_empty() {
        let first_device = &hardware_devices[0];
        println!("Attempting to open first device...");

        match Xr2280x::device_open(&hid_api, first_device) {
            Ok(device) => {
                let info = device.get_device_info();
                let capabilities = device.get_capabilities();

                println!("✓ Successfully opened device:");
                println!(
                    "  Serial: {}",
                    info.serial_number.as_deref().unwrap_or("N/A")
                );
                println!(
                    "  Manufacturer: {}",
                    info.manufacturer_string.as_deref().unwrap_or("N/A")
                );
                println!("  GPIO Count: {}", capabilities.gpio_count);

                // Test basic functionality
                println!("\n4. Testing Device Functionality:");
                println!("--------------------------------");

                // Test I2C functionality if available
                if first_device.i2c_interface.is_some() {
                    println!("Testing I2C interface...");
                    match device.i2c_set_speed_khz(100) {
                        Ok(()) => println!("✓ I2C speed set to 100kHz"),
                        Err(e) => println!("✗ I2C speed setting failed: {}", e),
                    }
                } else {
                    println!("I2C interface not available on this device");
                }

                // Test GPIO functionality if available
                if first_device.edge_interface.is_some() {
                    println!("Testing EDGE (GPIO) interface...");
                    // We can't safely test GPIO without knowing the hardware setup,
                    // so just verify the interface is accessible
                    println!(
                        "✓ EDGE interface accessible (GPIO count: {})",
                        capabilities.gpio_count
                    );
                } else {
                    println!("EDGE interface not available on this device");
                }
            }
            Err(e) => {
                println!("✗ Failed to open device: {}", e);
            }
        }
    } else {
        println!("No hardware devices available to open.");
    }

    // Demonstrate different opening methods
    println!("\n3. Alternative Opening Methods:");
    println!("------------------------------");

    // Open by index
    println!("Opening by index (0)...");
    match Xr2280x::open_by_index(&hid_api, 0) {
        Ok(_device) => println!("✓ Successfully opened device by index"),
        Err(e) => println!("✗ Failed to open by index: {}", e),
    }

    // Open by serial (if we have one)
    if let Some(first_device) = hardware_devices.first() {
        if let Some(ref serial) = first_device.serial_number {
            println!("Opening by serial number '{}'...", serial);
            match Xr2280x::open_by_serial(&hid_api, serial) {
                Ok(_device) => println!("✓ Successfully opened device by serial number"),
                Err(e) => println!("✗ Failed to open by serial: {}", e),
            }
        }
    }

    println!("\n4. Summary:");
    println!("-----------");
    println!("Found {} hardware devices", hardware_devices.len());
    println!("Hardware devices group logical USB interfaces by serial number");
    println!("and provide unified access to both I2C and GPIO/PWM functionality.");
    println!("This approach matches the physical reality of the hardware.");

    Ok(())
}
