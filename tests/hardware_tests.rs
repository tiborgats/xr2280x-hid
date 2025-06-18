// tests/hardware_tests.rs
use hidapi::HidApi;
use std::{thread, time::Duration};
use xr2280x_hid::{
    self, I2cAddress, PwmChannel, PwmCommand, Result, flags,
    gpio::{GpioDirection, GpioLevel, GpioPin, GpioPull},
}; // Import the public flags module

// Helper to open the first device, panics on failure for test simplicity
fn open_test_device() -> xr2280x_hid::Xr2280x {
    let hid_api = HidApi::new().expect("Failed to create HID API");
    // Try opening EDGE first, then I2C if EDGE fails (for cases where only one is connected/usable)
    xr2280x_hid::Xr2280x::open_by_vid_pid(
        &hid_api,
        xr2280x_hid::EXAR_VID,
        xr2280x_hid::XR2280X_EDGE_PID,
    )
    .or_else(|_| {
        xr2280x_hid::Xr2280x::open_by_vid_pid(
            &hid_api,
            xr2280x_hid::EXAR_VID,
            xr2280x_hid::XR2280X_I2C_PID,
        )
    })
    .expect("Failed to open any XR2280x device. Is it connected and permissions set?")
}

#[test]
#[ignore] // Ignore by default, requires hardware
fn test_gpio_output_readback() -> Result<()> {
    let device = open_test_device();
    let pin = GpioPin::new(0)?; // Test E0

    // Check capabilities before proceeding
    if pin.number() >= device.get_capabilities().gpio_count {
        println!(
            "Skipping GPIO test: Pin {} not supported on this model.",
            pin.number()
        );
        return Ok(());
    }

    println!("Testing GPIO Output Readback on pin {}", pin.number());
    device.gpio_assign_to_edge(pin)?;
    device.gpio_set_direction(pin, GpioDirection::Output)?;
    device.gpio_set_pull(pin, GpioPull::None)?;

    device.gpio_write(pin, GpioLevel::High)?;
    thread::sleep(Duration::from_millis(5)); // Allow state to settle
    assert_eq!(
        device.gpio_read(pin)?,
        GpioLevel::High,
        "Pin should read HIGH"
    );

    device.gpio_write(pin, GpioLevel::Low)?;
    thread::sleep(Duration::from_millis(5));
    assert_eq!(
        device.gpio_read(pin)?,
        GpioLevel::Low,
        "Pin should read LOW"
    );

    // Cleanup: Set back to input
    device.gpio_set_direction(pin, GpioDirection::Input)?;
    Ok(())
}

#[test]
#[ignore] // Ignore by default, requires hardware
fn test_i2c_presence_check() -> Result<()> {
    let device = open_test_device();
    let known_good_addr = I2cAddress::new_7bit(0x27)?; // CHANGE THIS to an address KNOWN TO BE on your bus
    let known_bad_addr = I2cAddress::new_7bit(0x31)?; // CHANGE THIS to an address KNOWN TO BE EMPTY

    println!("Testing I2C Presence Check");
    device.i2c_set_speed_khz(100)?;

    // Use constants via the re-exported flags module
    let i2c_flags = flags::i2c::START_BIT | flags::i2c::STOP_BIT;

    println!("Checking for device at {}...", known_good_addr);
    match device.i2c_transfer_raw(known_good_addr, None, None, i2c_flags, Some(100)) {
        Ok(_) => println!("Device found at {} (ACK)", known_good_addr),
        Err(xr2280x_hid::Error::I2cNack { .. }) => panic!(
            "Device NOT found at {} (NACK), but expected.",
            known_good_addr
        ),
        Err(e) => return Err(e),
    }

    println!("Checking for device at {}...", known_bad_addr);
    match device.i2c_transfer_raw(known_bad_addr, None, None, i2c_flags, Some(100)) {
        Ok(_) => panic!(
            "Device found at {} (ACK), but NOT expected.",
            known_bad_addr
        ),
        Err(xr2280x_hid::Error::I2cNack { .. }) => {
            println!("Device not found at {} (NACK) as expected.", known_bad_addr)
        }
        Err(e) => return Err(e),
    }

    Ok(())
}

#[test]
#[ignore] // Ignore by default, requires hardware
fn test_pwm_basic_output() -> Result<()> {
    let device = open_test_device();
    let pwm_channel = PwmChannel::Pwm1;
    let pwm_pin = GpioPin::new(20)?; // Test E20

    // Check capabilities before proceeding
    if pwm_pin.number() >= device.get_capabilities().gpio_count {
        println!(
            "Skipping PWM test: Pin {} not supported on this model.",
            pwm_pin.number()
        );
        return Ok(());
    }

    println!("Testing PWM Output on pin {}", pwm_pin.number());
    device.gpio_assign_to_edge(pwm_pin)?;
    device.gpio_set_direction(pwm_pin, GpioDirection::Output)?;
    device.gpio_set_pull(pwm_pin, GpioPull::None)?;

    // Set ~500 Hz, 50% duty cycle
    let high_ns = 1_000_000; // 1ms
    let low_ns = 1_000_000; // 1ms
    device.pwm_set_periods_ns(pwm_channel, high_ns, low_ns)?;
    device.pwm_set_pin(pwm_channel, pwm_pin)?;
    device.pwm_control(pwm_channel, true, PwmCommand::FreeRun)?;

    println!(
        "PWM should be running on pin {} for 3 seconds...",
        pwm_pin.number()
    );
    thread::sleep(Duration::from_secs(3));

    device.pwm_control(pwm_channel, false, PwmCommand::Idle)?;
    println!("PWM stopped.");

    // Set pin back to input
    device.gpio_set_direction(pwm_pin, GpioDirection::Input)?;
    Ok(())
}

// Add more hardware tests:
// - GPIO input reading (requires external signal)
// - GPIO pull resistor verification (measure voltage externally)
// - I2C read/write with a specific device
// - PWM one-shot mode
// - GPIO interrupt configuration + read_gpio_interrupt_report (hard to automate verification)
