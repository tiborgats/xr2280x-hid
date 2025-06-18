use hidapi::HidApi;
use std::time::Instant;
use xr2280x_hid::{self, Result};

fn main() -> Result<()> {
    env_logger::init();
    let hid_api = HidApi::new()?;

    println!("XR2280x I2C Bus Scanner (Advanced)");
    println!("===================================");

    println!("Opening first XR2280x device...");
    let device = match xr2280x_hid::Xr2280x::device_open_first(&hid_api) {
        Ok(dev) => dev,
        Err(e) => {
            eprintln!("Error opening device: {}", e);
            eprintln!(
                "Ensure device is connected and permissions are set (e.g., udev rules on Linux)."
            );
            return Err(e);
        }
    };

    let device_info = device.get_device_info();
    println!("Device opened successfully!");
    println!(
        "  Product: {:?}",
        device_info
            .product_string
            .unwrap_or_else(|| "Unknown".to_string())
    );
    println!(
        "  Serial:  {:?}",
        device_info
            .serial_number
            .unwrap_or_else(|| "Unknown".to_string())
    );
    println!();

    println!("Setting I2C speed to 100kHz...");
    device.i2c_set_speed_khz(100)?;

    println!();
    println!("Scanning I2C bus (7-bit addresses 0x08 to 0x77)...");
    println!("     0  1  2  3  4  5  6  7  8  9  A  B  C  D  E  F");

    let start_time = Instant::now();

    // Use the advanced scan method with progress reporting
    let found_devices = device.i2c_scan_with_progress(0x08, 0x77, |addr, found, idx, total| {
        // Print address grid
        if addr % 16 == 0 {
            print!("{:02X}: ", addr & 0xF0);
        }

        if found {
            print!("{:02X} ", addr);
        } else {
            print!("-- ");
        }

        // Print newline at end of each row
        if addr % 16 == 15 {
            println!();
        }

        // Show progress bar occasionally
        if idx % 32 == 0 || idx == total - 1 {
            let progress = (idx + 1) as f32 / total as f32 * 100.0;
            eprint!("\rProgress: [{:>3.0}%] ", progress);

            // Simple progress bar
            let bar_width = 30;
            let filled = (progress / 100.0 * bar_width as f32) as usize;
            eprint!("[");
            for i in 0..bar_width {
                if i < filled {
                    eprint!("█");
                } else {
                    eprint!("░");
                }
            }
            eprint!("]");

            if idx == total - 1 {
                eprintln!(); // Final newline
            }
        }
    })?;

    let scan_duration = start_time.elapsed();

    println!();
    println!("Scan Results:");
    println!("=============");

    if found_devices.is_empty() {
        println!("No I2C devices found on the bus.");
    } else {
        println!("Found {} device(s):", found_devices.len());
        for &addr in &found_devices {
            println!("  0x{:02X} ({})", addr, device_name_hint(addr));
        }
    }

    println!();
    println!(
        "Scan completed in {:.2}ms ({:.1} addresses/sec)",
        scan_duration.as_secs_f64() * 1000.0,
        112.0 / scan_duration.as_secs_f64()
    );

    // Suggest next steps if devices were found
    if !found_devices.is_empty() {
        println!();
        println!("Next steps:");
        println!("- Use device.i2c_write_7bit(addr, &data) to write data");
        println!("- Use device.i2c_read_7bit(addr, &mut buffer) to read data");
        println!(
            "- Use device.i2c_write_read_7bit(addr, &write_data, &mut read_buffer) for register access"
        );
    }

    Ok(())
}

/// Provides a hint about what type of device might be at a given I2C address
fn device_name_hint(addr: u8) -> &'static str {
    match addr {
        0x08..=0x0F => "Reserved/Rare",
        0x10..=0x17 => "Audio/Video",
        0x18..=0x1F => "Audio/Sensors",
        0x20..=0x27 => "GPIO Expanders",
        0x28..=0x2F => "Sensors",
        0x30..=0x37 => "Displays/Mixed",
        0x38..=0x3F => "Displays/Sensors",
        0x40..=0x47 => "Current Sensors",
        0x48..=0x4F => "Temperature/ADC",
        0x50..=0x57 => "EEPROM/RTC",
        0x58..=0x5F => "Mixed Sensors",
        0x60..=0x67 => "Displays/Sensors",
        0x68..=0x6F => "RTC/IMU",
        0x70..=0x77 => "Displays/Mux",
        _ => "Unknown",
    }
}
