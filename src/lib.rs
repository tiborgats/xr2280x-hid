//! # XR2280x HID Driver
//!
//! This crate provides a Rust driver for the Exar XR2280x USB HID to I2C/GPIO bridge chips.
//!
//! ## Features
//!
//! - I2C master controller with support for 7-bit and 10-bit addressing
//! - GPIO control with interrupt support
//! - PWM output generation
//! - Cross-platform support via hidapi
//!
//! ## Device Support
//!
//! - XR22800: 8 GPIO pins
//! - XR22801: 8 GPIO pins
//! - XR22802: 32 GPIO pins
//! - XR22804: 32 GPIO pins
//!
//! ## Example Usage
//!
//! ### Basic I2C Communication
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device_info = find_first(&hid_api)?;
//! let mut device = Xr2280x::open(&hid_api, &device_info)?;
//!
//! // Configure I2C speed
//! device.i2c_set_speed_khz(100)?;
//!
//! // Write data to I2C device at address 0x50
//! let data = [0x00, 0x01, 0x02, 0x03];
//! device.i2c_write_7bit(0x50, &data)?;
//!
//! // Read data from I2C device
//! let mut buffer = [0u8; 4];
//! device.i2c_read_7bit(0x50, &mut buffer)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### GPIO Control
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, GpioPin, GpioDirection, GpioLevel, find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device_info = find_first(&hid_api)?;
//! let mut device = Xr2280x::open(&hid_api, &device_info)?;
//!
//! // Configure GPIO pin 0 as output
//! let pin = GpioPin::new(0)?;
//! device.gpio_assign_to_edge(pin)?;
//! device.gpio_set_direction(pin, GpioDirection::Output)?;
//!
//! // Set pin high
//! device.gpio_write(pin, GpioLevel::High)?;
//!
//! // Read pin state
//! let level = device.gpio_read(pin)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### PWM Generation
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, PwmChannel, PwmCommand, GpioPin, find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device_info = find_first(&hid_api)?;
//! let mut device = Xr2280x::open(&hid_api, &device_info)?;
//!
//! // Configure PWM0 on GPIO pin 2
//! let pin = GpioPin::new(2)?;
//! device.pwm_set_pin(PwmChannel::Pwm0, pin)?;
//!
//! // Set PWM period (50% duty cycle, 1 kHz)
//! device.pwm_set_periods_ns(PwmChannel::Pwm0, 500_000, 500_000)?;
//!
//! // Start PWM in free-run mode
//! device.pwm_control(PwmChannel::Pwm0, true, PwmCommand::FreeRun)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! The XR2280x chips expose multiple USB HID interfaces:
//! - **I2C Interface** (PID 0x1100): I2C master controller
//! - **EDGE Interface** (PID 0x1200): GPIO, PWM, and interrupt controller
//!
//! This driver supports both interfaces through a unified API.
//!
//! ## Thread Safety
//!
//! The `Xr2280x` handle is not thread-safe (`!Send`, `!Sync`) due to the underlying hidapi
//! device handle. For concurrent access, use external synchronization or create separate
//! handles for each thread.
//!
//! ## Error Handling
//!
//! All operations return a `Result<T, Error>` where `Error` provides detailed information
//! about the failure, including:
//! - HID communication errors
//! - I2C bus errors (NACK, arbitration lost, timeout)
//! - Invalid arguments or unsupported operations
//! - Device-specific limitations
//!
//! ## Logging
//!
//! This crate uses the `log` crate for debugging output. Enable logging in your application
//! to see detailed communication traces:
//!
//! ```no_run
//! env_logger::init();
//! ```
//!
//! ## Platform Support
//!
//! Supported on Windows, Linux, and macOS through the hidapi library.
//! Requires appropriate permissions for USB device access on Linux.
//!
//! ## References
//!
//! - [XR2280x Datasheet](https://www.maxlinear.com/product/interface/uarts/usb-uarts/xr22804)
//! - [Application Note AN365](https://www.maxlinear.com/appnote/AN365.pdf)

// Re-export hidapi for convenience
pub use hidapi;

// Internal modules
mod consts;
mod error;

// Public modules
pub mod device;
pub mod gpio;
pub mod i2c;
pub mod interrupt;
pub mod pwm;

// Re-export main types and functions
pub use device::{find_all, find_devices, find_first, Capabilities, Xr2280x, XrDeviceDiscoveryInfo, XrDeviceInfo};
pub use error::{Error, Result};
pub use gpio::{GpioDirection, GpioGroup, GpioLevel, GpioPin, GpioPull};
pub use i2c::I2cAddress;
pub use interrupt::{GpioInterruptReport, ParsedGpioInterruptReport};
pub use pwm::{PwmChannel, PwmCommand};

// Re-export only essential public constants
pub use consts::{EXAR_VID, XR2280X_EDGE_PID, XR2280X_I2C_PID};

// --- Re-export necessary constants for public API use ---
/// Publicly accessible flags for controlling device features.
pub mod flags {
    /// Flags for use with [`crate::Xr2280x::i2c_transfer_raw`].
    pub mod i2c {
        // Re-export flags needed for i2c_transfer_raw
        pub use crate::consts::i2c::out_flags::{ACK_LAST_READ, START_BIT, STOP_BIT};
    }
    // Add other flags here if needed (e.g., for interrupts if a parsing API is added)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpio_pin_creation() {
        assert!(GpioPin::new(0).is_ok());
        assert!(GpioPin::new(31).is_ok());
        assert!(GpioPin::new(32).is_err());
    }

    #[test]
    fn test_gpio_pin_helpers() {
        let pin = GpioPin::new(17).unwrap();
        assert_eq!(pin.number(), 17);
        assert_eq!(pin.group_index(), 1);
        assert_eq!(pin.bit_index(), 1);
        assert_eq!(pin.mask(), 0x0002);
    }

    #[test]
    fn test_i2c_address_creation() {
        assert!(I2cAddress::new_7bit(0x50).is_ok());
        assert!(I2cAddress::new_7bit(0x7F).is_ok());
        assert!(I2cAddress::new_7bit(0x80).is_err());

        assert!(I2cAddress::new_10bit(0x000).is_ok());
        assert!(I2cAddress::new_10bit(0x3FF).is_ok());
        assert!(I2cAddress::new_10bit(0x400).is_err());
    }

    #[test]
    fn test_pwm_unit_conversion() {
        // Test conversion constants
        let unit_ns = consts::edge::PWM_UNIT_TIME_NS;
        
        // Helper to convert ns to pwm units (matching the implementation)
        let ns_to_units = |nanoseconds: u64| -> Result<u16> {
            if nanoseconds == 0 {
                return Err(Error::ArgumentOutOfRange(
                    "PWM time must be greater than 0 ns".to_string(),
                ));
            }
            let units = (nanoseconds as f64 / unit_ns).round() as u64;
            if units < consts::edge::PWM_MIN_UNITS as u64 {
                Err(Error::ArgumentOutOfRange("too small".to_string()))
            } else if units > consts::edge::PWM_MAX_UNITS as u64 {
                Err(Error::ArgumentOutOfRange("too large".to_string()))
            } else {
                Ok(units as u16)
            }
        };
        
        // Helper to convert pwm units to ns
        let units_to_ns = |units: u16| -> u64 {
            (units as f64 * unit_ns).round() as u64
        };

        // Test basic conversions
        let units = ns_to_units(1000).unwrap();
        assert_eq!(units, 4); // 1000ns / 266.667ns ≈ 3.75, rounds to 4

        let ns = units_to_ns(4);
        assert_eq!(ns, 1067); // 4 * 266.667ns ≈ 1066.67, rounds to 1067

        // Test edge cases
        assert!(ns_to_units(0).is_err());
        assert!(ns_to_units(134).is_ok()); // Minimum that rounds to 1 unit
        assert!(ns_to_units(133).is_err()); // Just below minimum
        assert!(ns_to_units(1_100_000).is_err()); // Too large

        // Test round-trip conversion accuracy
        for units in [1, 10, 100, 1000, 4095] {
            let ns = units_to_ns(units);
            let units_back = ns_to_units(ns).unwrap();
            // Should be exact or within 1 unit due to rounding
            assert!(
                units_back == units || units_back == units - 1 || units_back == units + 1,
                "Round-trip failed for {} units: got {} back",
                units,
                units_back
            );
        }
    }
}