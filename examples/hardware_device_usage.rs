//! Example demonstrating complete XR2280x hardware device usage.
//!
//! This example shows how to use the new hardware device API which provides
//! unified access to both I2C and GPIO/PWM functionality through a single
//! device handle, rather than opening separate logical interfaces.
//!
//! Run with: cargo run --example hardware_device_usage

use hidapi::HidApi;
use std::thread::sleep;
use std::time::Duration;
use xr2280x_hid::{
    GpioDirection, GpioLevel, GpioPin, PwmChannel, PwmCommand, Xr2280x, device_find_first,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("XR2280x Device Usage Example");
    println!("=============================\n");

    let hid_api = HidApi::new()?;

    // 1. Find and open a device (unified I2C + GPIO access)
    println!("1. Finding and Opening Device");
    println!("-----------------------------");

    let hardware_device = match device_find_first(&hid_api) {
        Ok(device_info) => {
            println!("Found device:");
            println!(
                "  Serial: {}",
                device_info.serial_number.as_deref().unwrap_or("N/A")
            );
            println!(
                "  Product: {}",
                device_info.product_string.as_deref().unwrap_or("Unknown")
            );
            println!(
                "  I2C Interface: {}",
                if device_info.i2c_interface.is_some() {
                    "Available"
                } else {
                    "Not Available"
                }
            );
            println!(
                "  GPIO Interface: {}",
                if device_info.edge_interface.is_some() {
                    "Available"
                } else {
                    "Not Available"
                }
            );

            // Open the device (automatically opens both I2C and EDGE interfaces)
            println!("\nOpening device...");
            Xr2280x::device_open(&hid_api, &device_info)?
        }
        Err(_) => {
            println!("No XR2280x devices found.");
            println!("Please connect an XR2280x device and try again.");
            return Ok(());
        }
    };

    let capabilities = hardware_device.get_capabilities();
    println!("✓ Hardware device opened successfully!");
    println!("  GPIO pins available: {}", capabilities.gpio_count);

    // 2. I2C Functionality
    println!("\n2. I2C Functionality");
    println!("--------------------");

    // Set I2C speed
    match hardware_device.i2c_set_speed_khz(100) {
        Ok(()) => println!("✓ I2C speed set to 100kHz"),
        Err(e) => println!("✗ Failed to set I2C speed: {e}"),
    }

    // Scan for I2C devices
    println!("Scanning I2C bus for devices...");
    match hardware_device.i2c_scan_default() {
        Ok(devices) => {
            if devices.is_empty() {
                println!("  No I2C devices found on the bus");
            } else {
                let device_count = devices.len();
                println!("  Found {device_count} I2C device(s):");
                for addr in devices {
                    println!("    0x{addr:02X}");
                }
            }
        }
        Err(e) => println!("✗ I2C scan failed: {e}"),
    }

    // Example I2C communication (with a hypothetical device at 0x50)
    println!("\nTesting I2C communication with address 0x50 (common EEPROM address):");

    // Try to read from the device
    let mut read_buffer = [0u8; 4];
    match hardware_device.i2c_read_7bit(0x50, &mut read_buffer) {
        Ok(()) => {
            println!("✓ Successfully read from 0x50: {read_buffer:02X?}");
        }
        Err(e) => {
            println!("✗ Failed to read from 0x50: {e} (this is normal if no device is connected)");
        }
    }

    // 3. GPIO Functionality
    println!("\n3. GPIO Functionality");
    println!("---------------------");

    // Configure GPIO pins
    let led_pin = GpioPin::new(0)?;
    let button_pin = GpioPin::new(1)?;

    println!("Configuring GPIO pins...");

    // Assign pins to EDGE controller and set directions
    match hardware_device.gpio_assign_to_edge(led_pin) {
        Ok(()) => {
            hardware_device.gpio_set_direction(led_pin, GpioDirection::Output)?;
            println!("✓ Pin 0 configured as output (LED)");
        }
        Err(e) => println!("✗ Failed to configure pin 0: {e}"),
    }

    match hardware_device.gpio_assign_to_edge(button_pin) {
        Ok(()) => {
            hardware_device.gpio_set_direction(button_pin, GpioDirection::Input)?;
            println!("✓ Pin 1 configured as input (button)");
        }
        Err(e) => println!("✗ Failed to configure pin 1: {e}"),
    }

    // GPIO control demonstration
    println!("\nGPIO control demonstration (blinking LED and reading button):");
    for i in 0..5 {
        // Set LED on
        if let Err(e) = hardware_device.gpio_write(led_pin, GpioLevel::High) {
            println!("✗ Failed to set LED high: {e}");
        } else {
            print!("LED ON  ");
        }

        // Read button state
        match hardware_device.gpio_read(button_pin) {
            Ok(level) => print!("Button: {level:?}  "),
            Err(e) => print!("Button read error: {e}  "),
        }

        let iteration = i + 1;
        println!("(iteration {iteration})");
        sleep(Duration::from_millis(500));

        // Set LED off
        if let Err(e) = hardware_device.gpio_write(led_pin, GpioLevel::Low) {
            println!("✗ Failed to set LED low: {e}");
        } else {
            print!("LED OFF ");
        }

        // Read button state again
        match hardware_device.gpio_read(button_pin) {
            Ok(level) => print!("Button: {level:?}  "),
            Err(e) => print!("Button read error: {e}  "),
        }

        let iteration = i + 1;
        println!("(iteration {iteration})");
        sleep(Duration::from_millis(500));
    }

    // 4. PWM Functionality
    println!("\n4. PWM Functionality");
    println!("--------------------");

    let pwm_pin = GpioPin::new(2)?;
    println!("Configuring PWM on pin 2...");

    match hardware_device.pwm_set_pin(PwmChannel::Pwm0, pwm_pin) {
        Ok(()) => {
            println!("✓ PWM0 assigned to pin 2");

            // Generate a 1kHz PWM signal with 50% duty cycle
            let frequency_hz = 1000;
            let period_ns = 1_000_000_000 / frequency_hz;
            let duty_cycle = 0.5; // 50%
            let high_time = (period_ns as f64 * duty_cycle) as u64;
            let low_time = period_ns - high_time;

            println!(
                "Starting PWM: {} Hz, {:.1}% duty cycle",
                frequency_hz,
                duty_cycle * 100.0
            );

            hardware_device.pwm_control(PwmChannel::Pwm0, true, PwmCommand::FreeRun)?;
            hardware_device.pwm_set_periods_ns(PwmChannel::Pwm0, high_time, low_time)?;

            println!("✓ PWM running for 3 seconds...");
            sleep(Duration::from_secs(3));

            // Change duty cycle to demonstrate variable control
            let duty_cycle = 0.25; // 25%
            let high_time = (period_ns as f64 * duty_cycle) as u64;
            let low_time = period_ns - high_time;

            println!("Changing duty cycle to {:.1}%", duty_cycle * 100.0);
            hardware_device.pwm_set_periods_ns(PwmChannel::Pwm0, high_time, low_time)?;
            sleep(Duration::from_secs(2));

            // Stop PWM
            hardware_device.pwm_control(PwmChannel::Pwm0, false, PwmCommand::Idle)?;
            println!("✓ PWM stopped");
        }
        Err(e) => println!("✗ Failed to configure PWM: {e}"),
    }

    // 5. Bulk GPIO Operations
    println!("\n5. Bulk GPIO Operations");
    println!("-----------------------");

    println!("Demonstrating bulk GPIO operations on pins 0-7...");

    // Configure multiple pins as outputs
    let pin_mask = 0x00FF; // Pins 0-7

    // Assign pins to EDGE controller
    for pin_num in 0..8 {
        if let Ok(pin) = GpioPin::new(pin_num) {
            let _ = hardware_device.gpio_assign_to_edge(pin);
        }
    }

    // Set all pins as outputs using bulk operation
    use xr2280x_hid::GpioGroup;
    match hardware_device.gpio_set_direction_masked(
        GpioGroup::Group0,
        pin_mask,
        GpioDirection::Output,
    ) {
        Ok(()) => {
            println!("✓ Pins 0-7 configured as outputs");

            // Create a running light pattern
            println!("Running light pattern on pins 0-7:");
            for i in 0..8 {
                let pattern = 1u16 << i;
                match hardware_device.gpio_write_masked(GpioGroup::Group0, pin_mask, pattern) {
                    Ok(()) => {
                        print!("  Pattern: {:08b}", pattern as u8);
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                        sleep(Duration::from_millis(200));
                        println!(" ✓");
                    }
                    Err(e) => println!("  ✗ Failed to write pattern {pattern}: {e}"),
                }
            }

            // Turn off all LEDs
            hardware_device.gpio_write_masked(GpioGroup::Group0, pin_mask, 0)?;
            println!("✓ All LEDs turned off");
        }
        Err(e) => println!("✗ Failed to configure bulk GPIO: {e}"),
    }

    // 6. Demonstrate unified hardware device benefits
    println!("\n6. Unified Hardware Device Benefits");
    println!("----------------------------------");
    println!("This example demonstrates the key benefits of the device approach:");
    println!("• Single device handle provides access to both I2C and GPIO/PWM");
    println!("• No need to manage separate logical device connections");
    println!("• Deterministic device ordering by serial number");
    println!("• Simplified multi-device scenarios");
    println!("• Hardware-centric view matches physical device reality");

    let device_info = hardware_device.get_device_info();
    println!("\nDevice Summary:");
    println!(
        "  Serial: {}",
        device_info.serial_number.as_deref().unwrap_or("N/A")
    );
    println!(
        "  Manufacturer: {}",
        device_info.manufacturer_string.as_deref().unwrap_or("N/A")
    );
    println!("  GPIO Count: {}", capabilities.gpio_count);
    println!("  I2C: Available");
    println!("  GPIO/PWM: Available");

    println!("\n✓ Example completed successfully!");
    Ok(())
}
