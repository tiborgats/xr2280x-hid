use hidapi::HidApi;
use xr2280x_hid::{self, Result};

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

    // Use the fast scan method with default address range (0x08-0x77)
    let found_devices = device.i2c_scan_default()?;

    // Print found devices
    for &addr in &found_devices {
        println!("Device found at 7-bit 0x{:02X}", addr);
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
