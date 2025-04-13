use hidapi::HidApi;
use std::io::{self, Write};
use xr2280x_hid::{self, gpio::GpioPin, Result}; // Import GpioPin

fn main() -> Result<()> {
    env_logger::init();
    let hid_api = HidApi::new()?;

    // --- Find ALL default XR2280x devices ---
    println!(
        "Searching for default XR2280x HID devices (VID=0x{:04X}, PID=0x{:04X} or 0x{:04X})...",
        xr2280x_hid::EXAR_VID,
        xr2280x_hid::XR2280X_I2C_PID,
        xr2280x_hid::XR2280X_EDGE_PID
    );
    let devices = xr2280x_hid::find_all(&hid_api)?;

    if devices.is_empty() {
        println!("No devices found.");
        return Ok(());
    }

    println!("Found {} device(s):", devices.len());
    for (i, info) in devices.iter().enumerate() {
        println!(
            "  {}: VID=0x{:04X}, PID=0x{:04X}, Interface={}, Path={:?}, Serial='{}', Product='{}'",
            i,
            info.vid,
            info.pid,
            info.interface_number,
            info.path,
            info.serial_number.as_deref().unwrap_or("N/A"),
            info.product_string.as_deref().unwrap_or("N/A"),
        );
    }

    // --- Select Device ---
    let selected_info = if devices.len() == 1 {
        println!("Automatically selecting the only device found.");
        &devices[0]
    } else {
        // Prompt user to select
        loop {
            print!(
                "Enter the number of the device to open (0-{}): ",
                devices.len() - 1
            );
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            match input.trim().parse::<usize>() {
                Ok(index) if index < devices.len() => {
                    break &devices[index];
                }
                _ => {
                    println!(
                        "Invalid input. Please enter a number between 0 and {}.",
                        devices.len() - 1
                    );
                }
            }
        }
    };

    // --- Open Selected Device ---
    println!("Opening device: {:?}", selected_info.path);
    let device = match xr2280x_hid::Xr2280x::open(&hid_api, selected_info) {
        Ok(dev) => dev,
        Err(e) => {
            eprintln!("Error opening selected device: {}", e);
            return Err(e);
        }
    };

    println!("Successfully opened device!");
    let opened_info = device.get_device_info()?;
    println!("Opened Info: {:?}", opened_info);
    println!("Capabilities: {:?}", device.get_capabilities());

    // --- Now you can interact with the 'device' handle ---
    // Example: Read GPIO 0 state if it's an EDGE device
    if selected_info.pid == xr2280x_hid::XR2280X_EDGE_PID {
        let pin0 = GpioPin::new(0)?; // Use typed pin
        match device.gpio_read(pin0) {
            Ok(level) => println!("GPIO Pin 0 current level: {:?}", level),
            Err(e) => eprintln!("Error reading GPIO 0: {}", e),
        }
    } else {
        println!("Opened device is not an EDGE interface, skipping GPIO read example.");
    }

    Ok(())
}
