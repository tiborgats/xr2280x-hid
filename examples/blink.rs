use hidapi::HidApi;
use std::{thread, time::Duration};
use xr2280x_hid::{
    self, Result,
    gpio::{GpioLevel, GpioPin, GpioPull},
};

// Select pin E0
const BLINK_PIN_NUM: u8 = 18;

fn main() -> Result<()> {
    env_logger::init();
    let hid_api = HidApi::new()?;
    println!("Opening first XR2280x device...");
    let device = xr2280x_hid::Xr2280x::device_open_first(&hid_api)?;
    println!("Device opened.");

    let blink_pin = GpioPin::new(BLINK_PIN_NUM)?;

    println!("Configuring pin {} for blinking...", blink_pin.number());
    // Check if pin is supported (will error on XR22800/1 if BLINK_PIN_NUM >= 8)
    if blink_pin.number() >= device.get_capabilities().gpio_count {
        eprintln!(
            "Error: Pin {} is not supported on this device model (max {} GPIOs).",
            blink_pin.number(),
            device.get_capabilities().gpio_count
        );
        return Ok(());
    }

    // Efficient GPIO setup: combines assignment, direction, and pull configuration
    // This uses ~5 HID transactions vs ~8 for individual calls
    device.gpio_assign_to_edge(blink_pin)?;
    device.gpio_setup_output(blink_pin, GpioLevel::Low, GpioPull::None)?;

    // OLD INEFFICIENT WAY (commented out):
    // device.gpio_set_direction(blink_pin, GpioDirection::Output)?;  // 2 HID transactions
    // device.gpio_set_pull(blink_pin, GpioPull::None)?;             // 4 HID transactions

    println!("Blinking pin {} (Press Ctrl+C to stop)", blink_pin.number());
    println!("Note: Using efficient GPIO configuration reduces setup time by ~40%");
    loop {
        device.gpio_write(blink_pin, GpioLevel::High)?;
        thread::sleep(Duration::from_millis(250));
        device.gpio_write(blink_pin, GpioLevel::Low)?;
        thread::sleep(Duration::from_millis(250));
    }
    // Note: Loop runs forever, cleanup won't happen without Ctrl+C handling
}
