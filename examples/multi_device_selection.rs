//! Multi-device selection demonstration.
//!
//! This example shows how to use the new multi-device selection features
//! of the xr2280x-hid crate to enumerate, select, and open specific devices
//! when multiple XR2280x devices are connected.

use hidapi::HidApi;
use std::io::{self, Write};
use xr2280x_hid::{Error, Result, Xr2280x};

fn main() -> Result<()> {
    env_logger::init();
    let hid_api = HidApi::new()?;

    println!("=== XR2280x Multi-Device Selection Demo ===\n");

    // Method 1: Enumerate all devices using the new enumerate_devices method
    println!("1. Enumerating all XR2280x devices...");
    let device_infos = Xr2280x::enumerate_devices(&hid_api)?;

    if device_infos.is_empty() {
        println!("No XR2280x devices found. Please connect a device and try again.");
        return Ok(());
    }

    println!("Found {} XR2280x device(s):", device_infos.len());
    for (i, info) in device_infos.iter().enumerate() {
        println!(
            "  [{}] VID: 0x{:04X}, PID: 0x{:04X}, Path: {:?}",
            i,
            info.vendor_id(),
            info.product_id(),
            info.path()
        );
        println!(
            "      Serial: '{}', Product: '{}'",
            info.serial_number().unwrap_or("N/A"),
            info.product_string().unwrap_or("N/A")
        );
    }

    // Method 2: Open by index
    println!("\n2. Opening device by index...");
    match Xr2280x::open_by_index(&hid_api, 0) {
        Ok(device) => {
            println!("✓ Successfully opened device at index 0");
            let info = device.get_device_info()?;
            println!("  Device info: {:?}", info);
            println!("  Capabilities: {:?}", device.get_capabilities());
        }
        Err(e) => {
            println!("✗ Failed to open device at index 0: {}", e);
        }
    }

    // Method 3: Open by serial number (if available)
    if let Some(serial) = device_infos[0].serial_number() {
        println!("\n3. Opening device by serial number '{}'...", serial);
        match Xr2280x::open_by_serial(&hid_api, serial) {
            Ok(device) => {
                println!("✓ Successfully opened device by serial number");
                let info = device.get_device_info()?;
                println!("  Device info: {:?}", info);
            }
            Err(e) => {
                println!("✗ Failed to open device by serial: {}", e);
            }
        }
    } else {
        println!("\n3. Skipping serial number test (device has no serial number)");
    }

    // Method 4: Open by path
    println!("\n4. Opening device by path...");
    let device_path = device_infos[0].path();
    match Xr2280x::open_by_path(&hid_api, device_path) {
        Ok(device) => {
            println!("✓ Successfully opened device by path");
            let info = device.get_device_info()?;
            println!("  Device info: {:?}", info);
        }
        Err(e) => {
            println!("✗ Failed to open device by path: {}", e);
        }
    }

    // Method 5: Interactive device selection
    if device_infos.len() > 1 {
        println!("\n5. Interactive device selection...");
        let selected_device = interactive_device_selection(&hid_api, &device_infos)?;
        if let Some(device) = selected_device {
            println!("✓ Successfully opened selected device");
            let info = device.get_device_info()?;
            println!("  Device info: {:?}", info);

            // Demonstrate a simple operation
            demonstrate_device_operation(&device)?;
        }
    } else {
        println!("\n5. Skipping interactive selection (only one device available)");
    }

    // Method 6: Error handling examples
    println!("\n6. Error handling examples...");

    // Try to open non-existent serial
    match Xr2280x::open_by_serial(&hid_api, "NONEXISTENT_SERIAL") {
        Ok(_) => println!("Unexpected: Found device with fake serial"),
        Err(Error::DeviceNotFoundBySerial { serial, message }) => {
            println!("✓ Expected error for fake serial '{}': {}", serial, message);
        }
        Err(e) => println!("Unexpected error type: {}", e),
    }

    // Try to open out-of-range index
    match Xr2280x::open_by_index(&hid_api, 999) {
        Ok(_) => println!("Unexpected: Found device at index 999"),
        Err(Error::DeviceNotFoundByIndex { index, message }) => {
            println!("✓ Expected error for index {}: {}", index, message);
        }
        Err(e) => println!("Unexpected error type: {}", e),
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}

/// Interactive device selection when multiple devices are available
fn interactive_device_selection(
    hid_api: &HidApi,
    device_infos: &[&hidapi::DeviceInfo],
) -> Result<Option<Xr2280x>> {
    loop {
        println!("\nAvailable devices:");
        for (i, info) in device_infos.iter().enumerate() {
            println!(
                "  [{}] {} (Serial: {}, Interface: {})",
                i,
                info.product_string().unwrap_or("XR2280x"),
                info.serial_number().unwrap_or("N/A"),
                if info.product_id() == xr2280x_hid::XR2280X_I2C_PID {
                    "I2C"
                } else {
                    "EDGE"
                }
            );
        }

        print!(
            "\nEnter device index (0-{}) or 'q' to quit: ",
            device_infos.len() - 1
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("q") {
            return Ok(None);
        }

        match input.parse::<usize>() {
            Ok(index) if index < device_infos.len() => {
                match Xr2280x::open_by_index(hid_api, index) {
                    Ok(device) => return Ok(Some(device)),
                    Err(e) => {
                        println!("Error opening device at index {}: {}", index, e);
                        continue;
                    }
                }
            }
            _ => {
                println!(
                    "Invalid input. Please enter a number between 0 and {} or 'q'.",
                    device_infos.len() - 1
                );
            }
        }
    }
}

/// Demonstrate a simple operation with the opened device
fn demonstrate_device_operation(device: &Xr2280x) -> Result<()> {
    println!("\nDemonstrating device operation...");

    let info = device.get_device_info()?;

    // Check if this is an I2C interface
    if info.product_id == xr2280x_hid::XR2280X_I2C_PID {
        println!("This is an I2C interface. Setting bus speed to 100kHz...");
        match device.i2c_set_speed_khz(100) {
            Ok(()) => println!("✓ I2C speed set successfully"),
            Err(e) => println!("✗ Failed to set I2C speed: {}", e),
        }
    }

    // Check if this is an EDGE interface
    if info.product_id == xr2280x_hid::XR2280X_EDGE_PID {
        println!("This is an EDGE interface. Checking GPIO capabilities...");
        let capabilities = device.get_capabilities();
        println!("✓ Device supports {} GPIO pins", capabilities.gpio_count);

        // Try to read GPIO pin 0 state
        use xr2280x_hid::gpio::GpioPin;
        if let Ok(pin0) = GpioPin::new(0) {
            match device.gpio_read(pin0) {
                Ok(level) => println!("✓ GPIO pin 0 current level: {:?}", level),
                Err(e) => println!(
                    "Note: Could not read GPIO pin 0 (may need configuration): {}",
                    e
                ),
            }
        }
    }

    Ok(())
}
