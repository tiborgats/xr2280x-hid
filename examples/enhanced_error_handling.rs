//! Enhanced Error Handling Example
//!
//! This example demonstrates the comprehensive, context-aware error handling
//! capabilities of the XR2280x-HID crate. It shows how different error types
//! provide specific context for debugging and enable targeted recovery strategies.

use hidapi::HidApi;
use std::thread;
use std::time::Duration;
use xr2280x_hid::{
    self, Error, Result,
    gpio::{GpioDirection, GpioLevel, GpioPin, GpioPull},
    pwm::{PwmChannel, PwmCommand},
};

fn main() -> Result<()> {
    env_logger::init();

    println!("=== XR2280x Enhanced Error Handling Demo ===\n");

    let hid_api = HidApi::new()?;
    let device = xr2280x_hid::Xr2280x::device_open_first(&hid_api)?;

    println!("Device opened: {:?}\n", device.get_device_info());

    // Demonstrate I2C error handling
    demonstrate_i2c_error_handling(&device)?;

    // Demonstrate GPIO error handling
    demonstrate_gpio_error_handling(&device)?;

    // Demonstrate PWM error handling
    demonstrate_pwm_error_handling(&device)?;

    // Demonstrate error recovery strategies
    demonstrate_error_recovery(&device)?;

    println!("\n=== Error Handling Demo Complete ===");
    Ok(())
}

/// Demonstrates I2C-specific error handling with context-aware responses
fn demonstrate_i2c_error_handling(device: &xr2280x_hid::Xr2280x) -> Result<()> {
    println!("--- I2C Error Handling Examples ---");

    // Example 1: Normal NACK handling (not an error)
    println!("\n1. Handling NACK responses during device scanning:");

    let test_address = 0x50; // Common EEPROM address that might not be present
    let mut buffer = [0u8; 1];

    match device.i2c_read_7bit(test_address, &mut buffer) {
        Ok(_) => println!("   ✓ Device found at 0x{:02X}", test_address),
        Err(Error::I2cNack { address }) => {
            println!("   ℹ No device at address {} (this is normal)", address);
            println!("     This is expected when scanning for devices");
        }
        Err(Error::I2cTimeout { address }) => {
            println!("   ⚠ Hardware issue detected at address {}", address);
            println!("     Troubleshooting steps:");
            println!("       - Check device power supply (3.3V)");
            println!("       - Verify I2C pull-up resistors (4.7kΩ)");
            println!("       - Test with fewer devices connected");
            println!("       - Check for short circuits on SDA/SCL lines");
        }
        Err(Error::I2cArbitrationLost { address }) => {
            println!("   ⚠ Bus contention detected at address {}", address);
            println!("     Possible causes:");
            println!("       - Multiple I2C masters on the bus");
            println!("       - Electrical interference or noise");
            println!("       - Loose connections causing signal corruption");
        }
        Err(e) => println!("   ✗ Unexpected error: {}", e),
    }

    // Example 2: Comprehensive bus scanning with error handling
    println!("\n2. Comprehensive I2C bus scan with error recovery:");

    match device.i2c_scan_default() {
        Ok(devices) => {
            if devices.is_empty() {
                println!("   ℹ No I2C devices found on the bus");
                println!("     This could indicate:");
                println!("       - No devices connected");
                println!("       - Missing pull-up resistors");
                println!("       - Power supply issues");
            } else {
                println!("   ✓ Found {} I2C devices: {:02X?}", devices.len(), devices);
            }
        }
        Err(Error::I2cTimeout { address }) => {
            println!("   ⚠ Bus scan failed with timeout at address {}", address);
            println!("     Hardware diagnostics required - see troubleshooting above");
            return Ok(()); // Don't propagate this error for demo purposes
        }
        Err(e) => return Err(e),
    }

    Ok(())
}

/// Demonstrates GPIO-specific error handling with pin context
fn demonstrate_gpio_error_handling(device: &xr2280x_hid::Xr2280x) -> Result<()> {
    println!("\n--- GPIO Error Handling Examples ---");

    // Example 1: Pin validation errors
    println!("\n1. Pin validation and range checking:");

    // Try to use an invalid pin number
    match GpioPin::new(99) {
        Ok(_) => println!("   Unexpected: Pin 99 should be invalid"),
        Err(Error::PinArgumentOutOfRange { pin, message }) => {
            println!("   ✓ Correctly rejected invalid pin {}: {}", pin, message);
            println!("     Application can validate pins before use");
        }
        Err(e) => println!("   Unexpected error type: {}", e),
    }

    // Example 2: Device capability checking
    println!("\n2. Device capability validation:");

    let gpio_count = device.get_capabilities().gpio_count;
    println!("   Device has {} GPIO pins available", gpio_count);

    // Try to use a pin that might not exist on this device
    let test_pin = if gpio_count == 8 { 15 } else { 0 }; // Pin 15 doesn't exist on 8-pin devices

    if let Ok(pin) = GpioPin::new(test_pin) {
        match device.gpio_set_direction(pin, GpioDirection::Output) {
            Ok(_) => println!("   ✓ Successfully configured pin {}", test_pin),
            Err(Error::UnsupportedFeature(msg)) => {
                println!("   ℹ Feature limitation detected: {}", msg);
                println!("     Application can check device capabilities first");
            }
            Err(Error::GpioRegisterWriteError {
                pin,
                register,
                message,
            }) => {
                println!(
                    "   ⚠ GPIO hardware error on pin {} register 0x{:04X}: {}",
                    pin, register, message
                );
                println!("     Hardware diagnostics:");
                println!("       - Check device connection and power");
                println!("       - Verify USB cable and hub functionality");
                println!("       - Try power cycling the XR2280x device");
            }
            Err(e) => println!("   Unexpected error: {}", e),
        }
    }

    // Example 3: Successful GPIO operation with error context
    println!("\n3. Normal GPIO operations with error awareness:");

    if let Ok(pin) = GpioPin::new(0) {
        // Pin 0 should always exist
        match device.gpio_setup_output(pin, GpioLevel::Low, GpioPull::None) {
            Ok(_) => {
                println!("   ✓ Successfully configured pin 0 as output");

                // Demonstrate read operation with error handling
                match device.gpio_read(pin) {
                    Ok(level) => println!("   ✓ Pin 0 current level: {:?}", level),
                    Err(Error::GpioRegisterReadError {
                        pin,
                        register,
                        message,
                    }) => {
                        println!(
                            "   ⚠ Failed to read pin {} register 0x{:04X}: {}",
                            pin, register, message
                        );
                    }
                    Err(e) => println!("   Unexpected read error: {}", e),
                }
            }
            Err(Error::GpioRegisterWriteError {
                pin,
                register,
                message,
            }) => {
                println!(
                    "   ⚠ Failed to configure pin {} register 0x{:04X}: {}",
                    pin, register, message
                );
            }
            Err(e) => println!("   Configuration error: {}", e),
        }
    }

    Ok(())
}

/// Demonstrates PWM-specific error handling with channel context
fn demonstrate_pwm_error_handling(device: &xr2280x_hid::Xr2280x) -> Result<()> {
    println!("\n--- PWM Error Handling Examples ---");

    // Example 1: Parameter validation
    println!("\n1. PWM parameter validation:");

    // Try invalid PWM timing parameters
    match device.ns_to_pwm_units(0) {
        Ok(_) => println!("   Unexpected: 0ns should be invalid"),
        Err(Error::PwmParameterError { channel, message }) => {
            println!(
                "   ✓ Correctly rejected invalid timing parameter: {}",
                message
            );
            println!("     Channel context: {}", channel);
        }
        Err(e) => println!("   Unexpected error type: {}", e),
    }

    // Example 2: PWM configuration with error handling
    println!("\n2. PWM channel configuration:");

    let channel = PwmChannel::Pwm0;

    // Try to set valid PWM periods
    match device.pwm_set_periods_ns(channel, 1000000, 1000000) {
        // 1ms high, 1ms low
        Ok(_) => {
            println!("   ✓ Successfully set PWM periods for {:?}", channel);

            // Try to assign to a pin
            if let Ok(pin) = GpioPin::new(0) {
                match device.pwm_set_pin(channel, pin) {
                    Ok(_) => {
                        println!(
                            "   ✓ Successfully assigned {:?} to pin {}",
                            channel,
                            pin.number()
                        );

                        // Try to enable PWM
                        match device.pwm_control(channel, true, PwmCommand::FreeRun) {
                            Ok(_) => {
                                println!(
                                    "   ✓ Successfully enabled {:?} in free-run mode",
                                    channel
                                );

                                // Let it run briefly
                                thread::sleep(Duration::from_millis(100));

                                // Disable PWM
                                let _ = device.pwm_control(channel, false, PwmCommand::Idle);
                                println!("   ✓ PWM disabled");
                            }
                            Err(Error::PwmHardwareError { channel, message }) => {
                                println!(
                                    "   ⚠ PWM hardware error on channel {}: {}",
                                    channel, message
                                );
                                println!("     Check device capabilities and pin assignments");
                            }
                            Err(e) => println!("   PWM control error: {}", e),
                        }
                    }
                    Err(Error::UnsupportedFeature(msg)) => {
                        println!("   ℹ PWM pin assignment limitation: {}", msg);
                    }
                    Err(Error::PwmHardwareError { channel, message }) => {
                        println!(
                            "   ⚠ PWM pin assignment failed for channel {}: {}",
                            channel, message
                        );
                    }
                    Err(e) => println!("   PWM pin assignment error: {}", e),
                }
            }
        }
        Err(Error::PwmParameterError { channel, message }) => {
            println!(
                "   ⚠ PWM parameter error on channel {}: {}",
                channel, message
            );
        }
        Err(e) => println!("   PWM periods error: {}", e),
    }

    Ok(())
}

/// Demonstrates error recovery strategies based on specific error types
fn demonstrate_error_recovery(device: &xr2280x_hid::Xr2280x) -> Result<()> {
    println!("\n--- Error Recovery Strategies ---");

    // Example 1: Retry with exponential backoff for transient errors
    println!("\n1. Transient error recovery with retry logic:");

    let mut retry_count = 0;
    let max_retries = 3;

    loop {
        match device.i2c_scan_default() {
            Ok(devices) => {
                println!("   ✓ I2C scan successful after {} retries", retry_count);
                println!("     Found devices: {:02X?}", devices);
                break;
            }
            Err(Error::I2cTimeout { address }) if retry_count < max_retries => {
                retry_count += 1;
                let delay_ms = 100 * 2_u64.pow(retry_count - 1); // Exponential backoff
                println!(
                    "   ⟳ Retry {} after timeout at {} (waiting {}ms)",
                    retry_count, address, delay_ms
                );
                thread::sleep(Duration::from_millis(delay_ms));
            }
            Err(Error::I2cTimeout { address }) => {
                println!(
                    "   ✗ Persistent I2C timeout at {} after {} retries",
                    address, max_retries
                );
                println!("     Hardware intervention required");
                break;
            }
            Err(Error::I2cArbitrationLost { address }) if retry_count < max_retries => {
                retry_count += 1;
                println!(
                    "   ⟳ Retry {} after arbitration lost at {} (brief delay)",
                    retry_count, address
                );
                thread::sleep(Duration::from_millis(10)); // Short delay for arbitration
            }
            Err(e) => {
                println!("   ✗ Non-recoverable error: {}", e);
                break;
            }
        }
    }

    // Example 2: Graceful degradation based on capability errors
    println!("\n2. Graceful degradation for capability limitations:");

    let test_pins = [0, 8, 16]; // Test different pin ranges

    for &pin_num in &test_pins {
        if let Ok(pin) = GpioPin::new(pin_num) {
            match device.gpio_setup_output(pin, GpioLevel::Low, GpioPull::None) {
                Ok(_) => println!("   ✓ Pin {} configured successfully", pin_num),
                Err(Error::UnsupportedFeature(_)) => {
                    println!(
                        "   ℹ Pin {} not supported, skipping (graceful degradation)",
                        pin_num
                    );
                    // Application continues with supported pins only
                }
                Err(Error::GpioRegisterWriteError {
                    pin,
                    register,
                    message,
                }) => {
                    println!(
                        "   ⚠ Hardware error on pin {} register 0x{:04X}: {}",
                        pin, register, message
                    );
                    println!("     Marking pin as unavailable for this session");
                    // Application could maintain a list of failed pins
                }
                Err(e) => println!("   ✗ Unexpected error on pin {}: {}", pin_num, e),
            }
        }
    }

    // Example 3: Context-aware user feedback
    println!("\n3. User-friendly error reporting:");

    // Simulate a complex operation that might fail at different points
    if let Ok(pin) = GpioPin::new(0) {
        let operation_result = perform_complex_gpio_operation(device, pin);

        match operation_result {
            Ok(_) => println!("   ✓ Complex operation completed successfully"),
            Err(Error::GpioRegisterReadError { pin, register, .. }) => {
                println!("   ✗ User Message: Failed to read GPIO pin {} status", pin);
                println!(
                    "     Technical Details: Register 0x{:04X} read failure",
                    register
                );
                println!("     User Action: Check device connection and try again");
            }
            Err(Error::GpioRegisterWriteError { pin, register, .. }) => {
                println!("   ✗ User Message: Failed to configure GPIO pin {}", pin);
                println!(
                    "     Technical Details: Register 0x{:04X} write failure",
                    register
                );
                println!("     User Action: Verify device power and USB connection");
            }
            Err(Error::PwmParameterError { channel, message }) => {
                println!(
                    "   ✗ User Message: Invalid PWM settings for channel {}",
                    channel
                );
                println!("     Technical Details: {}", message);
                println!("     User Action: Adjust PWM frequency or duty cycle settings");
            }
            Err(e) => println!("   ✗ Operation failed: {}", e),
        }
    }

    Ok(())
}

/// A complex operation that might fail at different points to demonstrate error handling
fn perform_complex_gpio_operation(device: &xr2280x_hid::Xr2280x, pin: GpioPin) -> Result<()> {
    // Step 1: Configure pin as output
    device.gpio_setup_output(pin, GpioLevel::Low, GpioPull::None)?;

    // Step 2: Read back the configuration
    let current_level = device.gpio_read(pin)?;

    // Step 3: Toggle the pin
    let new_level = match current_level {
        GpioLevel::High => GpioLevel::Low,
        GpioLevel::Low => GpioLevel::High,
    };
    device.gpio_write(pin, new_level)?;

    // Step 4: If we have PWM capability, try to set up PWM
    if device.get_capabilities().gpio_count > 8 {
        let channel = PwmChannel::Pwm0;
        device.pwm_set_periods_ns(channel, 500000, 500000)?; // 500μs high, 500μs low
        device.pwm_set_pin(channel, pin)?;
    }

    Ok(())
}
