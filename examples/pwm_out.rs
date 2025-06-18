use hidapi::HidApi;
use std::{thread, time::Duration};
use xr2280x_hid::{
    self,
    gpio::{GpioDirection, GpioPin, GpioPull},
    PwmChannel, PwmCommand, Result,
};

const PWM_CHANNEL: PwmChannel = PwmChannel::Pwm0;
const PWM_PIN_NUM: u8 = 15; // E15

fn main() -> Result<()> {
    env_logger::init();
    let hid_api = HidApi::new()?;
    println!("Opening first XR2280x device...");
    let device = xr2280x_hid::Xr2280x::device_open_first(&hid_api)?;
    println!(
        "Device opened. Capabilities: {:?}",
        device.get_capabilities()
    );

    let pwm_pin = GpioPin::new(PWM_PIN_NUM)?;

    // Check if pin is supported for GPIO/PWM on this model
    if pwm_pin.number() >= device.get_capabilities().gpio_count {
        eprintln!(
            "Error: Pin {} is not supported for PWM on this device model (max {} GPIOs).",
            pwm_pin.number(),
            device.get_capabilities().gpio_count
        );
        return Ok(());
    }

    println!(
        "Configuring PWM{:?} on pin {}...",
        PWM_CHANNEL,
        pwm_pin.number()
    );
    device.gpio_assign_to_edge(pwm_pin)?;
    device.gpio_set_direction(pwm_pin, GpioDirection::Output)?;
    device.gpio_set_pull(pwm_pin, GpioPull::None)?;

    // Set ~1 kHz, 75% duty cycle
    let target_freq_hz = 1000.0;
    let duty_cycle = 0.75;
    let period_ns = 1_000_000_000.0 / target_freq_hz;
    let high_ns = (period_ns * duty_cycle) as u64;
    let low_ns = period_ns as u64 - high_ns;

    println!(
        "Target Freq: {:.1} Hz, Duty: {:.1}%",
        target_freq_hz,
        duty_cycle * 100.0
    );
    println!("Calculated High: {} ns, Low: {} ns", high_ns, low_ns);

    device.pwm_set_periods_ns(PWM_CHANNEL, high_ns, low_ns)?;
    let (read_high, read_low) = device.pwm_get_periods_ns(PWM_CHANNEL)?;
    println!("Read back High: {} ns, Low: {} ns", read_high, read_low);

    device.pwm_set_pin(PWM_CHANNEL, pwm_pin)?;
    println!("Starting PWM output...");
    device.pwm_control(PWM_CHANNEL, true, PwmCommand::FreeRun)?;

    println!("PWM running for 5 seconds (Press Ctrl+C to stop early)...");
    thread::sleep(Duration::from_secs(5));

    println!("Stopping PWM output...");
    device.pwm_control(PWM_CHANNEL, false, PwmCommand::Idle)?;

    // Set pin back to input
    device.gpio_set_direction(pwm_pin, GpioDirection::Input)?;
    println!("PWM example finished.");
    Ok(())
}
