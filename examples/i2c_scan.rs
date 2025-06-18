use hidapi::HidApi;
use std::time::Instant;
use xr2280x_hid::{self, Error, Result};

fn main() -> Result<()> {
    env_logger::init();
    let hid_api = HidApi::new()?;
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
    println!("Device opened.");

    println!("Setting I2C speed to 100kHz...");
    device.i2c_set_speed_khz(100)?;

    println!("Scanning I2C bus (7-bit addresses 0x08 to 0x77)...");
    let scan_start = Instant::now();

    // Use the fast scan method with default address range (0x08-0x77)
    match device.i2c_scan_default() {
        Ok(found_devices) => {
            let scan_duration = scan_start.elapsed();
            println!("✓ Scan completed in {:?}", scan_duration);

            // Print found devices
            for &addr in &found_devices {
                println!("Device found at 7-bit 0x{:02X}", addr);
            }

            if found_devices.is_empty() {
                println!("No I2C devices found.");
                println!("This is normal if no I2C devices are connected.");
                println!("Try connecting I2C sensors, EEPROMs, or other devices.");
            } else {
                println!(
                    "Scan complete. Found {} device(s) at addresses: {}",
                    found_devices.len(),
                    found_devices
                        .iter()
                        .map(|a| format!("0x{:02X}", a))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
        Err(Error::I2cTimeout { address }) => {
            let scan_duration = scan_start.elapsed();
            eprintln!("✗ I2C bus scan failed after {:?}", scan_duration);
            eprintln!("Stuck bus detected at address {}", address);
            eprintln!();
            eprintln!("TROUBLESHOOTING:");
            eprintln!("• An unpowered device may be holding I2C lines (SDA/SCL) low");
            eprintln!("• Check that all I2C devices have proper power connections");
            eprintln!("• Verify pull-up resistors are present (typically 4.7kΩ to 3.3V)");
            eprintln!("• Try disconnecting I2C devices one by one to isolate the problem");
            eprintln!("• Power cycle all I2C devices and the XR2280x");
            return Err(Error::I2cTimeout { address });
        }
        Err(Error::I2cArbitrationLost { address }) => {
            let scan_duration = scan_start.elapsed();
            eprintln!("✗ I2C bus scan failed after {:?}", scan_duration);
            eprintln!("Bus arbitration lost at address {}", address);
            eprintln!();
            eprintln!("TROUBLESHOOTING:");
            eprintln!("• Multiple I2C masters may be competing for bus control");
            eprintln!("• Check for loose or intermittent connections on SDA/SCL lines");
            eprintln!("• Verify signal integrity - use shorter wires or lower I2C speed");
            eprintln!("• Disconnect other I2C controllers/masters and retry");
            eprintln!("• Check for electrical interference or crosstalk");
            eprintln!("• Try reducing I2C speed: device.i2c_set_speed_khz(50)?;");
            return Err(Error::I2cArbitrationLost { address });
        }
        Err(e) => {
            let scan_duration = scan_start.elapsed();
            eprintln!("✗ I2C bus scan failed after {:?}", scan_duration);
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Try checking I2C connections and device power.");
            return Err(e);
        }
    }

    Ok(())
}
