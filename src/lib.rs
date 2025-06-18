//! # XR2280x HID Driver
//!
//! This crate provides a high-performance Rust driver for the Exar XR2280x family of USB HID to I2C/GPIO bridge chips.
//! These chips provide a convenient way to add I2C, GPIO, and PWM capabilities to any system with USB support.
//!
//! ## Features
//!
//! - **Fast I2C master controller** with support for 7-bit and 10-bit addressing
//!   - Optimized I2C device scanning (112 addresses in ~1 second)
//!   - Configurable bus speeds up to 400kHz
//! - **Flexible GPIO control** with interrupt support
//!   - Individual pin control with direction, pull-up/pull-down, and open-drain modes
//!   - Bulk operations for efficient multi-pin control
//! - **PWM output generation** on any GPIO pin
//!   - Two independent PWM channels with nanosecond precision
//!   - Multiple operating modes (idle, one-shot, free-run)
//! - **Cross-platform support** via hidapi (Linux, Windows, macOS)
//! - **Zero-copy operations** where possible for maximum performance
//!
//! ## Device Support
//!
//! | Model   | GPIO Pins | I2C | PWM | Interrupts |
//! |---------|-----------|-----|-----|------------|
//! | XR22800 | 8         | ✓   | ✓   | ✓          |
//! | XR22801 | 8         | ✓   | ✓   | ✓          |
//! | XR22802 | 32        | ✓   | ✓   | ✓          |
//! | XR22804 | 32        | ✓   | ✓   | ✓          |
//!
//! All devices operate at USB 2.0 Full Speed (12 Mbps) and support 3.3V logic levels.
//!
//! ## Quick Start
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, device_find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize HID API and find the first XR2280x device
//! let hid_api = HidApi::new()?;
//! let device_info = device_find_first(&hid_api)?;
//! let device = Xr2280x::device_open(&hid_api, &device_info)?;
//!
//! // Scan I2C bus for connected devices
//! device.i2c_set_speed_khz(100)?;
//! let devices = device.i2c_scan_default()?;
//! println!("Found {} I2C devices: {:02X?}", devices.len(), devices);
//! # Ok(())
//! # }
//! ```
//!
//! ## Multi-Device Selection
//!
//! When multiple XR2280x devices are connected, you can select specific devices using various methods:
//!
//! ### Enumerate All Hardware Devices
//!
//! ```no_run
//! use xr2280x_hid::Xr2280x;
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//!
//! // Get list of all XR2280x devices
//! let devices = Xr2280x::device_enumerate(&hid_api)?;
//! println!("Found {} XR2280x devices:", devices.len());
//!
//! for (i, info) in devices.iter().enumerate() {
//!     println!("  [{}] Serial: {}, Product: {}",
//!         i,
//!         info.serial_number.as_deref().unwrap_or("N/A"),
//!         info.product_string.as_deref().unwrap_or("Unknown")
//!     );
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Open by Serial Number
//!
//! ```no_run
//! use xr2280x_hid::Xr2280x;
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//!
//! // Open specific device by serial number
//! let device = Xr2280x::open_by_serial(&hid_api, "ABC123456")?;
//! println!("Opened device with serial ABC123456");
//! # Ok(())
//! # }
//! ```
//!
//! ### Open by Index
//!
//! ```no_run
//! use xr2280x_hid::Xr2280x;
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//!
//! // Open the second device found (0-based indexing)
//! let device = Xr2280x::open_by_index(&hid_api, 1)?;
//! println!("Opened device at index 1");
//! # Ok(())
//! # }
//! ```
//!
//! ### Open by Path
//!
//! ```no_run
//! use xr2280x_hid::Xr2280x;
//! use hidapi::HidApi;
//! use std::ffi::CString;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//!
//! // Open device by platform-specific path
//! let path = CString::new("/dev/hidraw0")?;
//! let device = Xr2280x::open_by_path(&hid_api, &path)?;
//! println!("Opened device at path /dev/hidraw0");
//! # Ok(())
//! # }
//! ```
//!
//! ## Example Usage
//!
//! ### I2C Communication
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, device_find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device = Xr2280x::device_open_first(&hid_api)?;
//!
//! // Configure I2C bus speed (supports 100kHz, 400kHz, etc.)
//! device.i2c_set_speed_khz(400)?;
//!
//! // Write to EEPROM at address 0x50
//! let write_data = [0x00, 0x10, 0x48, 0x65, 0x6C, 0x6C, 0x6F]; // Address + "Hello"
//! device.i2c_write_7bit(0x50, &write_data)?;
//!
//! // Read back from EEPROM
//! device.i2c_write_7bit(0x50, &[0x00, 0x10])?; // Set read address
//! let mut read_buffer = [0u8; 5];
//! device.i2c_read_7bit(0x50, &mut read_buffer)?;
//! println!("Read: {:?}", std::str::from_utf8(&read_buffer));
//!
//! // Combined write-read operation
//! let mut buffer = [0u8; 4];
//! device.i2c_write_read_7bit(0x50, &[0x00, 0x00], &mut buffer)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Fast I2C Device Discovery
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, device_find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device = Xr2280x::device_open_first(&hid_api)?;
//!
//! device.i2c_set_speed_khz(100)?;
//!
//! // Fast scan with progress reporting
//! let devices = device.i2c_scan_with_progress(0x08, 0x77, |addr, found, current, total| {
//!     if found {
//!         println!("Found device at 0x{:02X}", addr);
//!     }
//!     if current % 16 == 0 {
//!         println!("Progress: {}/{}", current, total);
//!     }
//! })?;
//!
//! println!("Scan complete! Found {} devices in total", devices.len());
//! # Ok(())
//! # }
//! ```
//!
//! ### GPIO Control
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, GpioPin, GpioDirection, GpioLevel, GpioPull, device_find_first};
//! use hidapi::HidApi;
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device = Xr2280x::device_open_first(&hid_api)?;
//!
//! // Configure GPIO pin 0 as output (LED)
//! let led_pin = GpioPin::new(0)?;
//! device.gpio_assign_to_edge(led_pin)?;
//! device.gpio_set_direction(led_pin, GpioDirection::Output)?;
//!
//! // Configure GPIO pin 1 as input with pull-up (button)
//! let button_pin = GpioPin::new(1)?;
//! device.gpio_assign_to_edge(button_pin)?;
//! device.gpio_set_direction(button_pin, GpioDirection::Input)?;
//! device.gpio_set_pull(button_pin, GpioPull::Up)?;
//!
//! // Blink LED and read button
//! for _ in 0..10 {
//!     device.gpio_write(led_pin, GpioLevel::High)?;
//!     let button_state = device.gpio_read(button_pin)?;
//!     println!("LED ON, Button: {:?}", button_state);
//!
//!     sleep(Duration::from_millis(500));
//!
//!     device.gpio_write(led_pin, GpioLevel::Low)?;
//!     sleep(Duration::from_millis(500));
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Bulk GPIO Operations
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, GpioGroup, GpioDirection, GpioPin, device_find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device = Xr2280x::device_open_first(&hid_api)?;
//!
//! // Configure pins 0-7 as outputs (LED array)
//! let led_mask = 0x00FF; // Pins 0-7
//!
//! // Assign pins to EDGE controller individually
//! for pin_num in 0..8 {
//!     let pin = GpioPin::new(pin_num)?;
//!     device.gpio_assign_to_edge(pin)?;
//! }
//!
//! // Set direction for all pins at once using mask
//! device.gpio_set_direction_masked(GpioGroup::Group0, led_mask, GpioDirection::Output)?;
//!
//! // Create a running light effect
//! for i in 0..8 {
//!     let pattern = 1u16 << i;
//!     device.gpio_write_masked(GpioGroup::Group0, led_mask, pattern)?;
//!     std::thread::sleep(std::time::Duration::from_millis(100));
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### PWM Generation
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, PwmChannel, PwmCommand, GpioPin, device_find_first};
//! use hidapi::HidApi;
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device = Xr2280x::device_open_first(&hid_api)?;
//!
//! // Configure PWM0 on GPIO pin 2 (servo control)
//! let servo_pin = GpioPin::new(2)?;
//! device.pwm_set_pin(PwmChannel::Pwm0, servo_pin)?;
//!
//! // Set servo PWM: 20ms period, variable pulse width
//! let period_ns = 20_000_000; // 20ms = 50Hz
//!
//! // Servo positions: 1ms = 0°, 1.5ms = 90°, 2ms = 180°
//! let positions = [1_000_000, 1_500_000, 2_000_000]; // pulse widths in ns
//!
//! device.pwm_control(PwmChannel::Pwm0, true, PwmCommand::FreeRun)?;
//!
//! // Move servo through positions
//! for &pulse_width in &positions {
//!     let low_time = period_ns - pulse_width;
//!     device.pwm_set_periods_ns(PwmChannel::Pwm0, pulse_width, low_time)?;
//!     sleep(Duration::from_millis(1000));
//! }
//!
//! // Stop PWM
//! device.pwm_control(PwmChannel::Pwm0, false, PwmCommand::Idle)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### LED Brightness Control
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, PwmChannel, PwmCommand, GpioPin, device_find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device = Xr2280x::device_open_first(&hid_api)?;
//!
//! // Configure PWM1 on GPIO pin 5 for LED brightness
//! let led_pin = GpioPin::new(5)?;
//! device.pwm_set_pin(PwmChannel::Pwm1, led_pin)?;
//!
//! // High-frequency PWM for smooth dimming (1kHz)
//! let frequency_hz = 1000;
//! let period_ns = 1_000_000_000 / frequency_hz;
//!
//! device.pwm_control(PwmChannel::Pwm1, true, PwmCommand::FreeRun)?;
//!
//! // Fade from 0% to 100% brightness
//! for brightness in 0..=100 {
//!     let duty_cycle = brightness as f32 / 100.0;
//!     let high_time = (period_ns as f32 * duty_cycle) as u64;
//!     let low_time = period_ns - high_time;
//!
//!     device.pwm_set_periods_ns(PwmChannel::Pwm1, high_time, low_time)?;
//!     std::thread::sleep(std::time::Duration::from_millis(50));
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Performance
//!
//! This driver includes several optimizations for maximum performance:
//!
//! - **Fast I2C scanning**: Complete 112-address scan in ~1 second (500x faster than naive implementations)
//! - **Bulk GPIO operations**: Update multiple pins in a single USB transaction
//! - **Optimized timeouts**: Minimal delays while maintaining reliability
//! - **Zero-copy reads**: Direct buffer access where possible
//!
//! ## Platform Support
//!
//! - **Linux**: Requires udev rules for non-root access
//! - **Windows**: Works with built-in HID drivers
//! - **macOS**: Supported via hidapi
//!
//! ### Linux Setup
//!
//! Create `/etc/udev/rules.d/99-xr2280x.rules`:
//! ```text
//! # XR2280x I2C Interface
//! SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04e2", ATTRS{idProduct}=="1100", MODE="0666"
//! # XR2280x EDGE Interface
//! SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04e2", ATTRS{idProduct}=="1200", MODE="0666"
//! ```
//!
//! Then reload udev rules: `sudo udevadm control --reload-rules`
//!
//! ## Architecture
//!
//! The XR2280x chips expose multiple USB HID interfaces as separate logical devices:
//! - **I2C Interface** (PID 0x1100): I2C master controller with configurable speeds
//! - **EDGE Interface** (PID 0x1200): GPIO, PWM, and interrupt controller
//!
//! This driver groups these logical interfaces by hardware device (using serial number)
//! and automatically opens both interfaces to present a unified API for complete device access.
//! The device approach eliminates the need to manage separate logical device connections.
//!
//! ## Error Handling
//!
//! All operations return `Result<T, Error>` with detailed error information:
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, Error, device_find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device = Xr2280x::device_open_first(&hid_api)?;
//!
//! match device.i2c_write_7bit(0x50, &[0x00, 0x01]) {
//!     Ok(()) => println!("Write successful"),
//!     Err(Error::I2cNack { address }) => {
//!         println!("Device at {:?} did not acknowledge", address);
//!     },
//!     Err(Error::DeviceNotFound) => {
//!         println!("XR2280x device not connected");
//!     },
//!     Err(e) => println!("Other error: {}", e),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Hardware Device Selection Errors
//!
//! The hardware device selection methods provide specific error types for better error handling:
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, Error};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//!
//! // Handle specific hardware device selection errors
//! match Xr2280x::open_by_serial(&hid_api, "NONEXISTENT") {
//!     Ok(device) => println!("Hardware device opened successfully"),
//!     Err(Error::DeviceNotFoundBySerial { serial, message }) => {
//!         println!("No hardware device found with serial '{}': {}", serial, message);
//!     },
//!     Err(Error::DeviceNotFoundByIndex { index, message }) => {
//!         println!("No hardware device found at index {}: {}", index, message);
//!     },
//!     Err(Error::MultipleDevicesFound { count, message }) => {
//!         println!("Found {} devices when expecting one: {}", count, message);
//!     },
//!     Err(e) => println!("Other error: {}", e),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Safety and Limitations
//!
//! - GPIO pins operate at 3.3V logic levels
//! - Maximum I2C speed is device-dependent (typically 400kHz)
//! - PWM resolution depends on frequency (higher frequency = lower resolution)
//! - No electrical isolation - use appropriate level shifters for 5V systems
//!
//! ## Troubleshooting
//!
//! **Device not found**: Check USB connection and permissions (udev rules on Linux)
//!
//! **I2C timeouts**: Verify I2C device connections and pull-up resistors
//!
//! **GPIO not working**: Ensure pins are assigned to EDGE interface before use
//!
//! **PWM frequency limits**: PWM resolution decreases at higher frequencies due to hardware constraints
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
pub use device::{
    Capabilities, Xr2280x, XrDeviceDetails, XrDeviceInfo, device_find, device_find_all,
    device_find_first,
};
pub use error::{Error, Result};
pub use gpio::{GpioDirection, GpioGroup, GpioLevel, GpioPin, GpioPull};
pub use i2c::{I2cAddress, timeouts};
pub use interrupt::{GpioInterruptReport, ParsedGpioInterruptReport};
pub use pwm::{PwmChannel, PwmCommand};

// Re-export essential hidapi types for multi-device selection
pub use hidapi::{DeviceInfo, HidApi};

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
    fn test_i2c_address_wire_format() {
        // Test that 7-bit addresses are correctly converted to 8-bit wire format
        // In I2C, a 7-bit address 0x50 becomes 0xA0 on the wire (shifted left)

        // Common I2C device addresses and their expected wire format
        let test_cases = [
            (0x50, 0xA0), // EEPROM
            (0x68, 0xD0), // RTC
            (0x77, 0xEE), // Barometric sensor
            (0x3C, 0x78), // OLED display
            (0x48, 0x90), // Temperature sensor
        ];

        for (addr_7bit, expected_wire) in test_cases {
            let addr = I2cAddress::new_7bit(addr_7bit).unwrap();
            if let I2cAddress::Bit7(a) = addr {
                let wire_format = a << 1;
                assert_eq!(
                    wire_format, expected_wire,
                    "7-bit address 0x{:02X} should become 0x{:02X} on wire, got 0x{:02X}",
                    addr_7bit, expected_wire, wire_format
                );
            }
        }

        // Verify the range is still correct after shifting
        assert_eq!(0x00 << 1, 0x00); // Minimum
        assert_eq!(0x7F << 1, 0xFE); // Maximum (0xFF would include R/W bit)
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
        let units_to_ns = |units: u16| -> u64 { (units as f64 * unit_ns).round() as u64 };

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
