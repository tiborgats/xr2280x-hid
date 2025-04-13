use hidapi::HidApi;
use std::time::Duration;
use xr2280x_hid::{self, flags, I2cAddress, Result}; // Import the public flags module

fn main() -> Result<()> {
    env_logger::init();
    let hid_api = HidApi::new()?;
    println!("Opening first XR2280x device...");
    let device = match xr2280x_hid::Xr2280x::open_first(&hid_api) {
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
    let mut found_devices = Vec::new();

    // Use constants via the re-exported flags module
    let i2c_flags = flags::i2c::START_BIT | flags::i2c::STOP_BIT;

    for addr_7bit in 0x08..=0x77 {
        let address = I2cAddress::new_7bit(addr_7bit)?; // Create typed address
                                                        // Use a short write (0 bytes) to check for ACK
        match device.i2c_transfer_raw(address, None, None, i2c_flags, Some(50)) {
            // Use shorter timeout
            Ok(_) => {
                println!("Device found at {}", address); // Display uses Display trait
                found_devices.push(addr_7bit);
            }
            Err(xr2280x_hid::Error::I2cNack { .. }) => {
                // No device at this address, continue silently
            }
            Err(xr2280x_hid::Error::I2cTimeout { .. }) => {
                // Timeout might indicate a stuck bus or very slow device, log it
                eprintln!("Timeout checking address {}", address);
            }
            Err(e) => {
                // Other error (ArbitrationLost, etc.)
                eprintln!("Error checking address {}: {}", address, e);
            }
        }
        // Small delay between probes might be necessary on some buses
        std::thread::sleep(Duration::from_millis(2));
    }

    if found_devices.is_empty() {
        println!("No I2C devices found.");
    } else {
        println!(
            "Scan complete. Found 7-bit addresses: {:?}",
            found_devices
                .iter()
                .map(|a| format!("0x{:02X}", a))
                .collect::<Vec<_>>()
        );
    }

    Ok(())
}
