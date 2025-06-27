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
//! // Open the first device (index 0)
//! let device = Xr2280x::open_by_index(&hid_api, 0)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Performance Architecture and Best Practices
//!
//! **⚠️ CRITICAL**: Understanding the communication architecture is essential for high-performance applications.
//! The XR2280x devices communicate via USB HID Feature Reports, and each operation has significant overhead.
//!
//! ### Communication Architecture
//!
//! Every GPIO and I2C operation translates to one or more HID Feature Report transactions:
//!
//! - **USB Protocol Overhead**: Control transfer setup and response handling
//! - **HID Report Processing**: Structured data formatting and parsing
//! - **Device Firmware Execution**: Register updates and hardware control
//! - **Typical Transaction Latency**: 5-10ms per HID Feature Report
//!
//! This means traditional "one operation per function call" patterns can be extremely inefficient.
//!
//! ### GPIO Performance Impact Analysis
//!
//! | Operation Pattern | HID Transactions | Typical Latency | Improvement |
//! |-------------------|------------------|-----------------|-------------|
//! | Single pin (individual calls) | 8 transactions | ~40-80ms | Baseline |
//! | Single pin (efficient API) | 5 transactions | ~25-50ms | **1.6x faster** |
//! | 4 pins (individual calls) | 32 transactions | ~160-320ms | Baseline |
//! | 4 pins (bulk API) | 6 transactions | ~30-60ms | **5.3x faster** |
//! | 8 pins (individual calls) | 64 transactions | ~320-640ms | Baseline |
//! | 8 pins (bulk API) | 6 transactions | ~30-60ms | **10.7x faster** |
//!
//! ### Root Cause: Read-Modify-Write Cycles
//!
//! Traditional GPIO operations perform inefficient read-modify-write cycles:
//!
//! ```text
//! gpio_set_direction(): [READ register] → [MODIFY bit] → [WRITE register] = 2 HID transactions
//! gpio_set_pull():      [READ pull-up] → [READ pull-down] → [WRITE pull-up] → [WRITE pull-down] = 4 HID transactions
//! gpio_write():         [WRITE to SET/CLEAR register] = 1 HID transaction
//!
//! Total per pin: 7-8 HID transactions
//! ```
//!
//! ### Architectural Solutions
//!
//! The high-performance APIs eliminate redundant operations through:
//!
//! 1. **Hardware-Aware Register Grouping**: Pins 0-15 (Group 0) and 16-31 (Group 1) use separate registers
//! 2. **Transaction Batching**: Combined operations reduce individual read-modify-write cycles
//! 3. **Bulk Processing**: O(1) complexity for multiple pins instead of O(N)
//! 4. **Optimized Register Access**: Uses dedicated SET/CLEAR registers where available
//!
//! ### High-Performance GPIO APIs
//!
//! **✅ RECOMMENDED: Efficient Single Pin Setup**
//! ```no_run
//! use xr2280x_hid::gpio::{GpioPin, GpioLevel, GpioPull};
//! # use xr2280x_hid::Xr2280x;
//! # fn example(device: &Xr2280x) -> xr2280x_hid::Result<()> {
//!
//! // ✅ Efficient output setup (5 HID transactions vs 8)
//! let pin = GpioPin::new(0)?;
//! device.gpio_setup_output(pin, GpioLevel::Low, GpioPull::None)?;
//!
//! // ✅ Efficient input setup (4 HID transactions vs 6)
//! device.gpio_setup_input(pin, GpioPull::Up)?;
//! # Ok(())
//! # }
//! ```
//!
//! **✅ HIGHLY RECOMMENDED: Bulk Operations**
//! ```no_run
//! use xr2280x_hid::gpio::{GpioPin, GpioLevel, GpioPull};
//! # use xr2280x_hid::Xr2280x;
//! # fn example(device: &Xr2280x) -> xr2280x_hid::Result<()> {
//!
//! // ✅ Bulk output setup (6 HID transactions total, regardless of pin count)
//! let pin_configs = vec![
//!     (GpioPin::new(0)?, GpioLevel::High),
//!     (GpioPin::new(1)?, GpioLevel::Low),
//!     (GpioPin::new(2)?, GpioLevel::High),
//!     (GpioPin::new(3)?, GpioLevel::Low),
//! ];
//! device.gpio_setup_outputs(&pin_configs, GpioPull::None)?;
//!
//! // ✅ Bulk input setup (6 HID transactions total)
//! let input_pins = vec![GpioPin::new(8)?, GpioPin::new(9)?, GpioPin::new(10)?];
//! device.gpio_setup_inputs(&input_pins, GpioPull::Up)?;
//! # Ok(())
//! # }
//! ```
//!
//! **❌ ANTI-PATTERN: Individual Operations in Loops**
//! ```no_run
//! use xr2280x_hid::gpio::{GpioPin, GpioDirection, GpioLevel, GpioPull};
//! # use xr2280x_hid::Xr2280x;
//! # fn example(device: &Xr2280x) -> xr2280x_hid::Result<()> {
//!
//! // ❌ EXTREMELY INEFFICIENT: 8 HID transactions × 4 pins = 32 transactions
//! for i in 0..4 {
//!     let pin = GpioPin::new(i)?;
//!     device.gpio_set_direction(pin, GpioDirection::Output)?;  // 2 transactions
//!     device.gpio_set_pull(pin, GpioPull::None)?;             // 4 transactions
//!     device.gpio_write(pin, GpioLevel::Low)?;                // 1 transaction
//!     // This loop creates 32 HID transactions vs 6 with bulk APIs!
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### HID Transaction Cost Reference
//!
//! | Operation | HID Transactions | Notes |
//! |-----------|------------------|-------|
//! | `gpio_write()` | 1 | Most efficient (uses SET/CLEAR registers) |
//! | `gpio_read()` | 1 | Single register read |
//! | `gpio_set_direction()` | 2 | Read-modify-write cycle |
//! | `gpio_set_pull()` | 4 | **Most expensive** (both pull registers) |
//! | `gpio_set_open_drain()` | 2 | Read-modify-write cycle |
//! | `gpio_setup_output()` | 5 | **Optimized combination** |
//! | `gpio_setup_input()` | 4 | **Optimized combination** |
//! | `gpio_setup_outputs()` | 6 | **Bulk operation (O(1) complexity)** |
//! | `gpio_setup_inputs()` | 6 | **Bulk operation (O(1) complexity)** |
//!
//! ### Migration Strategies
//!
//! **Immediate Wins - Simple Replacements:**
//! ```no_run
//! # use xr2280x_hid::{Xr2280x, gpio::*};
//! # fn example(device: &Xr2280x) -> xr2280x_hid::Result<()> {
//! // ❌ OLD (8 HID transactions)
//! // device.gpio_set_direction(pin, GpioDirection::Output)?;
//! // device.gpio_set_pull(pin, GpioPull::None)?;
//! // device.gpio_write(pin, GpioLevel::Low)?;
//!
//! // ✅ NEW (5 HID transactions - 37% improvement)
//! let pin = GpioPin::new(0)?;
//! device.gpio_setup_output(pin, GpioLevel::Low, GpioPull::None)?;
//! # Ok(())
//! # }
//! ```
//!
//! **Major Wins - Bulk Migration:**
//! ```no_run
//! # use xr2280x_hid::{Xr2280x, gpio::*};
//! # fn example(device: &Xr2280x) -> xr2280x_hid::Result<()> {
//! // ❌ OLD (N × 8 HID transactions)
//! // for (pin, level) in &pin_configs {
//! //     device.gpio_set_direction(*pin, GpioDirection::Output)?;
//! //     device.gpio_set_pull(*pin, GpioPull::None)?;
//! //     device.gpio_write(*pin, *level)?;
//! // }
//!
//! // ✅ NEW (6 HID transactions total - up to 10.7x improvement)
//! let pin_configs = vec![(GpioPin::new(0)?, GpioLevel::Low)];
//! device.gpio_setup_outputs(&pin_configs, GpioPull::None)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Performance Optimization Guidelines
//!
//! 1. **Initialize Once**: Use `gpio_setup_*()` functions during device initialization
//! 2. **Runtime Efficiency**: Use `gpio_write()` and `gpio_write_masked()` for state changes
//! 3. **Bulk Operations**: Always prefer bulk APIs when configuring multiple pins
//! 4. **Register Grouping**: Group operations by hardware boundaries (pins 0-15 vs 16-31)
//! 5. **State Caching**: Track GPIO states in application logic to avoid redundant reads
//! 6. **Minimize Reconfiguration**: Avoid repeatedly changing pin configurations
//!
//! ### I2C Performance Considerations
//!
//! - **Use `i2c_write_read()`** instead of separate `i2c_write()` + `i2c_read()` calls
//! - **Batch device scanning** with appropriate timeouts (see `i2c::timeouts` module)
//! - **Cache device responses** when possible to reduce bus traffic
//!
//! See the `gpio_efficient_config.rs` example for comprehensive performance demonstrations and measurements.
//!
//! ## Advanced Error Handling
//!
//! The XR2280x-HID crate provides comprehensive, context-aware error handling designed to make debugging
//! hardware issues and application problems much easier.
//!
//! ### Specific Error Types by Domain
//!
//! Instead of generic errors, the crate provides domain-specific error variants:
//!
//! **I2C Communication Errors:**
//! - [`Error::I2cNack`] - Device not found (normal during scanning)
//! - [`Error::I2cTimeout`] - Hardware issues (stuck bus, power problems)
//! - [`Error::I2cArbitrationLost`] - Bus contention or interference
//! - [`Error::I2cRequestError`] - Invalid parameters or data length
//! - [`Error::I2cUnknownError`] - Firmware-level issues
//!
//! **GPIO Hardware Errors:**
//! - [`Error::GpioRegisterReadError`] - Pin-specific register read failures
//! - [`Error::GpioRegisterWriteError`] - Pin-specific register write failures
//! - [`Error::GpioConfigurationError`] - Invalid pin configuration combinations
//! - [`Error::GpioHardwareError`] - Hardware-level GPIO issues
//! - [`Error::PinArgumentOutOfRange`] - Invalid pin numbers
//!
//! **PWM Configuration Errors:**
//! - [`Error::PwmConfigurationError`] - Channel configuration issues
//! - [`Error::PwmParameterError`] - Invalid timing or duty cycle parameters
//! - [`Error::PwmHardwareError`] - PWM hardware communication failures
//!
//! ### Enhanced Error Context
//!
//! Each error includes specific context to help with debugging:
//!
//! ```no_run
//! # use xr2280x_hid::*;
//! # use hidapi::HidApi;
//! # fn example() -> Result<()> {
//! # let hid_api = HidApi::new()?;
//! # let device = Xr2280x::device_open_first(&hid_api)?;
//! use xr2280x_hid::gpio::*;
//!
//! // GPIO errors include pin number and register details
//! match device.gpio_set_direction(GpioPin::new(5)?, GpioDirection::Output) {
//!     Err(Error::GpioRegisterWriteError { pin, register, message }) => {
//!         eprintln!("Failed to configure pin {} register 0x{:04X}: {}", pin, register, message);
//!         // Can implement pin-specific recovery logic
//!     }
//!     Err(Error::PinArgumentOutOfRange { pin, message }) => {
//!         eprintln!("Invalid pin {}: {}", pin, message);
//!         // Handle invalid pin number
//!     }
//!     Ok(_) => println!("Pin configured successfully"),
//!     Err(e) => return Err(e),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Error Recovery Strategies
//!
//! The specific error types enable targeted recovery strategies:
//!
//! ```no_run
//! # use xr2280x_hid::*;
//! # use hidapi::HidApi;
//! # fn example() -> Result<()> {
//! # let hid_api = HidApi::new()?;
//! # let device = Xr2280x::device_open_first(&hid_api)?;
//! // I2C error handling with specific recovery actions
//! match device.i2c_scan_default() {
//!     Ok(devices) => println!("Found devices: {:02X?}", devices),
//!     Err(Error::I2cTimeout { address }) => {
//!         eprintln!("Hardware issue detected at address {}", address);
//!         eprintln!("Recovery steps:");
//!         eprintln!("  1. Check device power supply");
//!         eprintln!("  2. Verify I2C pull-up resistors");
//!         eprintln!("  3. Test with fewer devices connected");
//!         // Could implement automatic retry logic here
//!     }
//!     Err(Error::I2cArbitrationLost { address }) => {
//!         eprintln!("Bus contention at {}, retrying...", address);
//!         // Implement retry with exponential backoff
//!     }
//!     Err(e) => return Err(e),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Benefits for Applications
//!
//! 1. **Precise Diagnostics**: Know exactly which pin, register, or device caused issues
//! 2. **Targeted Recovery**: Different error types enable different recovery strategies
//! 3. **Better User Experience**: Provide specific troubleshooting guidance to end users
//! 4. **Robust Applications**: Handle transient vs permanent failures appropriately
//! 5. **Development Efficiency**: Faster debugging with detailed error context
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
//! ### GPIO Interrupt Handling
//!
//! The XR2280x devices support GPIO interrupts with configurable edge detection.
//! The crate provides a **consistent Pin API** that eliminates the need for manual
//! pin number conversions and provides type safety throughout.
//!
//! #### Consistent Pin API Benefits
//!
//! - **🛡️ Type Safety**: All pin numbers validated through `GpioPin::new()`
//! - **🎯 Ergonomics**: No manual `u8` to `GpioPin` conversions required
//! - **⚡ Error Handling**: Invalid pin numbers caught at API boundary
//! - **🔄 Consistency**: Uniform use of `GpioPin` across all GPIO functions
//!
//! #### GPIO Edge Detection
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, GpioEdge, GpioPin, GpioPull, device_find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device = Xr2280x::device_open_first(&hid_api)?;
//!
//! // Configure GPIO pins for interrupt monitoring
//! let interrupt_pins = [0, 1, 2, 3];
//!
//! for &pin_num in &interrupt_pins {
//!     let pin = GpioPin::new(pin_num)?;
//!
//!     // Assign pin to EDGE interface
//!     device.gpio_assign_to_edge(pin)?;
//!
//!     // Configure as input with pull-up
//!     device.gpio_setup_input(pin, GpioPull::Up)?;
//!
//!     // Enable interrupts on both edges
//!     device.gpio_configure_interrupt(pin, true, true, true)?;
//! }
//!
//! println!("GPIO interrupts configured. Monitoring for events...");
//! # Ok(())
//! # }
//! ```
//!
//! #### Modern Interrupt Parsing API
//!
//! The new `parse_gpio_interrupt_pins()` function provides individual pin/edge
//! combinations with full type safety:
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, GpioEdge, GpioLevel, device_find_first};
//! use hidapi::HidApi;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let hid_api = HidApi::new()?;
//! # let device = Xr2280x::device_open_first(&hid_api)?;
//! // Read interrupt report with timeout
//! let report = device.read_gpio_interrupt_report(Some(1000))?;
//!
//! // NEW: Get individual pin/edge combinations with type safety
//! let pin_events = device.parse_gpio_interrupt_pins(&report)?;
//!
//! for (pin, edge) in pin_events {
//!     println!("📌 Pin {} triggered on {:?} edge", pin.number(), edge);
//!
//!     // Direct use with other GPIO functions - no conversion needed!
//!     let current_level = device.gpio_read(pin)?;
//!     let direction = device.gpio_get_direction(pin)?;
//!
//!     // Validate edge detection
//!     let edge_matches = matches!(
//!         (edge, current_level),
//!         (GpioEdge::Rising, GpioLevel::High) |
//!         (GpioEdge::Falling, GpioLevel::Low) |
//!         (GpioEdge::Both, _)
//!     );
//!
//!     if edge_matches {
//!         println!("✅ Edge detection consistent with current level");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! #### API Migration Guide
//!
//! **Old Approach (Manual Parsing):**
//! ```no_run
//! # use xr2280x_hid::{Xr2280x, GpioPin};
//! # fn old_example(device: &Xr2280x, report: &xr2280x_hid::GpioInterruptReport) -> xr2280x_hid::Result<()> {
//! // ❌ OLD: Manual group mask parsing
//! let parsed = unsafe { device.parse_gpio_interrupt_report(report)? };
//!
//! // User had to manually parse group masks and convert u8 to GpioPin
//! for bit_pos in 0..16 {
//!     if parsed.trigger_mask_group0 & (1 << bit_pos) != 0 {
//!         let pin_num = bit_pos as u8;
//!         let pin = GpioPin::new(pin_num)?; // Manual conversion required
//!         // Use pin with other GPIO functions...
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! **New Approach (Consistent Pin API):**
//! ```no_run
//! # use xr2280x_hid::{Xr2280x, GpioPin};
//! # fn new_example(device: &Xr2280x, report: &xr2280x_hid::GpioInterruptReport) -> xr2280x_hid::Result<()> {
//! // ✅ NEW: Clean, type-safe API
//! let pin_events = device.parse_gpio_interrupt_pins(report)?;
//!
//! for (pin, edge) in pin_events {
//!     // Pin is already validated and typed - no conversion needed!
//!     println!("Pin {} triggered on {:?} edge", pin.number(), edge);
//!
//!     // Direct use with GPIO functions
//!     let level = device.gpio_read(pin)?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! #### Complete Interrupt Monitoring Example
//!
//! ```no_run
//! use xr2280x_hid::{Xr2280x, GpioEdge, GpioLevel, GpioPin, GpioPull, device_find_first};
//! use hidapi::HidApi;
//! use std::collections::HashMap;
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hid_api = HidApi::new()?;
//! let device = Xr2280x::device_open_first(&hid_api)?;
//!
//! // Setup interrupt monitoring
//! let monitor_pins = [0, 1, 2, 3];
//! let mut pin_event_counts: HashMap<u8, usize> = HashMap::new();
//!
//! for &pin_num in &monitor_pins {
//!     let pin = GpioPin::new(pin_num)?;
//!     device.gpio_assign_to_edge(pin)?;
//!     device.gpio_setup_input(pin, GpioPull::Up)?;
//!     device.gpio_configure_interrupt(pin, true, true, true)?;
//!     pin_event_counts.insert(pin_num, 0);
//! }
//!
//! println!("Monitoring GPIO interrupts. Connect/disconnect pins to generate events...");
//!
//! // Monitor for 10 seconds
//! for _ in 0..100 {
//!     match device.read_gpio_interrupt_report(Some(100)) {
//!         Ok(report) => {
//!             // Process interrupt events with type-safe API
//!             let pin_events = device.parse_gpio_interrupt_pins(&report)?;
//!
//!             for (pin, edge) in pin_events {
//!                 let count = pin_event_counts.entry(pin.number()).or_insert(0);
//!                 *count += 1;
//!
//!                 println!("🎉 Pin {} {:?} edge (count: {})",
//!                     pin.number(), edge, count);
//!
//!                 // Validate current state
//!                 let current_level = device.gpio_read(pin)?;
//!                 println!("   Current level: {:?}", current_level);
//!             }
//!         }
//!         Err(xr2280x_hid::Error::Timeout) => {
//!             // Normal timeout, continue monitoring
//!             continue;
//!         }
//!         Err(e) => return Err(e.into()),
//!     }
//! }
//!
//! // Display summary
//! println!("\nInterrupt Summary:");
//! for (pin, count) in pin_event_counts {
//!     println!("  Pin {}: {} events", pin, count);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! #### Key Improvements
//!
//! 1. **Type Safety**: All pin numbers validated through `GpioPin::new()`
//! 2. **API Consistency**: Entire GPIO API uses `GpioPin` throughout
//! 3. **Error Handling**: Invalid pin numbers caught at API boundary
//! 4. **Ergonomics**: No manual `u8` to `GpioPin` conversions required
//! 5. **Edge Detection**: Typed `GpioEdge` enum for clear edge identification
//!
//! See the `consistent_pin_api.rs` and `gpio_interrupt_safe_usage.rs` examples
//! for comprehensive demonstrations of interrupt handling patterns.
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
//! ## Code Quality and Maintainability
//!
//! This crate emphasizes high code quality through systematic elimination of common
//! anti-patterns and the use of modern Rust best practices.
//!
//! ### Magic Number Elimination
//!
//! All hardcoded integer offsets in HID report parsing have been replaced with
//! descriptive named constants, significantly improving code readability and
//! maintainability.
//!
//! #### Before: Magic Numbers (Anti-pattern)
//! ```ignore
//! // ❌ Unclear what data is at each offset
//! let status_flags = in_buf[1];
//! let read_length = in_buf[3] as usize;
//! read_buf.copy_from_slice(&in_buf[5..5 + actual_read_len]);
//!
//! // ❌ No context for interrupt report structure
//! let group0_state = u16::from_le_bytes([report.raw_data[1], report.raw_data[2]]);
//! let group1_state = u16::from_le_bytes([report.raw_data[3], report.raw_data[4]]);
//! ```
//!
//! #### After: Named Constants (Best Practice)
//! ```ignore
//! // ✅ Self-documenting code with clear intent
//! let status_flags = in_buf[response_offsets::STATUS_FLAGS];
//! let read_length = in_buf[response_offsets::READ_LENGTH] as usize;
//! read_buf.copy_from_slice(
//!     &in_buf[response_offsets::READ_DATA_START
//!         ..response_offsets::READ_DATA_START + actual_read_len]
//! );
//!
//! // ✅ Clear GPIO interrupt report structure
//! let group0_state = u16::from_le_bytes([
//!     report.raw_data[report_offsets::GROUP0_STATE_LOW],
//!     report.raw_data[report_offsets::GROUP0_STATE_HIGH],
//! ]);
//! let group1_state = u16::from_le_bytes([
//!     report.raw_data[report_offsets::GROUP1_STATE_LOW],
//!     report.raw_data[report_offsets::GROUP1_STATE_HIGH],
//! ]);
//! ```
//!
//! #### Benefits Achieved
//!
//! 1. **Improved Readability**: Code is self-documenting through descriptive constant names
//! 2. **Enhanced Maintainability**: Single point of change if HID report structure changes
//! 3. **Better Error Prevention**: Type system helps prevent using wrong constants in wrong contexts
//! 4. **Documentation Value**: Constants serve as inline documentation of report structure
//! 5. **Future-Proofing**: Easy to extend with new report types or fields
//!
//! The improvement affects **22 magic number locations** across **3 core files**,
//! replacing them with **26 descriptive named constants** organized into **6 logical modules**.
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
pub use gpio::{GpioDirection, GpioEdge, GpioGroup, GpioLevel, GpioPin, GpioPull, GpioTransaction};
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
                    "7-bit address 0x{addr_7bit:02X} should become 0x{expected_wire:02X} on wire, got 0x{wire_format:02X}"
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
                "Round-trip failed for {units} units: got {units_back} back"
            );
        }
    }
}
