//! # xr2280x-hid
//!
//! A Rust crate for controlling the I²C and GPIO (EDGE) functions of
//! MaxLinear/Exar XR22800, XR22801, XR22802, and XR22804 USB bridge chips
//! via their USB HID interfaces.
//!
//! This crate uses the `hidapi` crate for cross-platform USB HID communication.
//!
//! ## Features
//!
//! *   Device discovery (`find_all`, `find_first`, `find_devices`).
//! *   Flexible device opening (`open`, `open_first`, `open_by_vid_pid`, `open_by_path`).
//! *   Querying device info from handle (`get_device_info`).
//! *   Querying detected capabilities (`get_capabilities`).
//! *   I²C communication:
//!     *   Speed setting (`i2c_set_speed_khz`).
//!     *   7-bit and 10-bit addressing support.
//!     *   Basic transfers (`i2c_write_7bit`, `i2c_read_7bit`, `i2c_write_read_7bit`, etc.).
//!     *   Raw transfers with custom flags and timeouts (`i2c_transfer_raw`).
//! *   GPIO (EDGE) control (Pins 0-31 mapped from E0-E31, model dependent):
//!     *   Strongly-typed `GpioPin` struct.
//!     *   Single pin and bulk (masked) operations.
//!     *   Assigning pins between UART/GPIO and EDGE functions.
//!     *   Setting/getting pin direction.
//!     *   Reading/Writing pin levels.
//!     *   Setting/getting pull-up/pull-down resistors.
//!     *   Setting/getting open-drain/tri-state output modes.
//!     *   Checking pin assignment.
//! *   GPIO Interrupt configuration (`gpio_configure_interrupt`).
//! *   Reading raw GPIO interrupt reports (`read_gpio_interrupt_report`).
//! *   Speculative parsing of GPIO interrupt reports (`parse_gpio_interrupt_report` - **Format Unverified**).
//! *   PWM Output configuration:
//!     *   Setting/getting periods using device units or nanoseconds.
//!     *   Setting/getting assigned output pin.
//!     *   Setting/getting control mode and enable state.
//!
//! ## Chip Support & Limitations
//!
//! This crate aims to support the HID interfaces common across the XR2280x family.
//!
//! *   **I²C:** Fully supported on all models (XR22800/1/2/4). Includes 7-bit and 10-bit addressing.
//! *   **EDGE (GPIO/PWM/Interrupts):**
//!     *   **XR22802/XR22804:** Support 32 GPIOs (E0-E31), mapped to pins 0-31. PWM can be assigned to any of these pins (if configured as output).
//!     *   **XR22800/XR22801:** Support **only 8 GPIOs (E0-E7)**, mapped to pins 0-7, via the HID interface. Attempts to access pins 8-31 will return an `Error::UnsupportedFeature`. PWM output can only be assigned to pins 0-7 on these models.
//! *   **Interrupt Parsing:** Reading raw interrupt reports is supported, but parsing (`parse_gpio_interrupt_report`) is speculative due to lack of documentation and requires hardware verification.
//!
//! The crate attempts to detect the GPIO capability (8 vs 32 pins) when the device is opened by checking for the presence of higher-group registers. Use `get_capabilities()` on the device handle to check.
//!
//! ## Installation
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! xr2280x-hid = "0.1.0" # Replace with the latest version
//! hidapi = "2.0"       # Or latest compatible version
//! log = "0.4"          # Optional, for logging
//!
//! [dev-dependencies]   # For examples/tests
//! env_logger = "0.10"
//! approx = "0.5"
//! ```
//!
//! You also need the `hidapi` library installed on your system. See the [`hidapi` crate documentation](https://docs.rs/hidapi/) for details.
//!
//! ## Basic Usage
//!
//! ```no_run
//! // Use items directly from the crate being tested, NO 'self'
//! use xr2280x_hid::{
//!     gpio::{GpioDirection, GpioLevel, GpioPin, GpioPull},
//!     Result,
//!     Xr2280x // Import the main struct directly
//! };
//! use hidapi::HidApi;
//! use std::{thread, time::Duration};
//!
//! fn main() -> Result<()> {
//!     // Optional: Initialize logging
//!     // env_logger::init();
//!
//!     let hid_api = HidApi::new()?;
//!
//!     // --- Option 1: Open the first default device found ---
//!     println!("Opening first default XR2280x device...");
//!     // Refer to items directly by their imported name
//!     let device = match Xr2280x::open_first(&hid_api) {
//!         Ok(dev) => dev,
//!         Err(e) => {
//!             eprintln!("Error opening device: {}", e);
//!             eprintln!("Ensure device is connected and permissions are set (e.g., udev rules on Linux).");
//!             // Use the imported Error type (via Result)
//!             return Err(e);
//!         }
//!     };
//!     println!("Device opened (first found). Info: {:?}", device.get_device_info()?);
//!     println!("Detected capabilities: {:?}", device.get_capabilities());
//!
//!
//!     // --- Use the opened device ---
//!     println!("\nSetting I2C speed...");
//!     device.i2c_set_speed_khz(100)?;
//!
//!     // --- GPIO Example (Pin E0 / GPIO 0) ---
//!     let gpio_pin = GpioPin::new(0)?; // Use imported GpioPin
//!     println!("\n--- GPIO Example (Pin {}) ---", gpio_pin.number());
//!     device.gpio_assign_to_edge(gpio_pin, true)?;
//!     device.gpio_set_direction(gpio_pin, GpioDirection::Output)?;
//!     // Use imported GpioPull
//!     device.gpio_set_pull(gpio_pin, GpioPull::None)?;
//!
//!     println!("Blinking pin {}...", gpio_pin.number());
//!     device.gpio_write(gpio_pin, GpioLevel::High)?;
//!     thread::sleep(Duration::from_millis(200));
//!     device.gpio_write(gpio_pin, GpioLevel::Low)?;
//!
//!     // Set back to input
//!     device.gpio_set_direction(gpio_pin, GpioDirection::Input)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Working with Multiple Devices / Custom IDs
//!
//! If you have multiple XR2280x devices connected, or devices programmed with custom Vendor/Product IDs, use the `find_devices` function and the `open` method for reliable selection:
//!
//! 1.  **Find Devices:** Call `xr2280x_hid::find_devices(&hid_api, vid, pid_option)`.
//! 2.  **Select Device:** Iterate the returned `Vec<XrDeviceDiscoveryInfo>` and choose based on `serial_number`, `path`, etc.
//! 3.  **Open Device:** Call `xr2280x_hid::Xr2280x::open(&hid_api, &selected_device_info)`.
//!
//! See the `examples/list_and_select.rs` example.
//!
//! ## Hardware Setup Notes
//!
//! *   **I²C Pull-up Resistors:** Required externally (e.g., 4.7kΩ to 3.3V).
//! *   **Linux udev Rules:** Grant user permission to the HID devices. Create `/etc/udev/rules.d/99-xr2280x.rules`:
//!     ```udev
//!     # Rule for Exar/MaxLinear XR2280x HID Interfaces (Default PIDs: I2C=1100, EDGE=1200)
//!     SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04e2", ATTRS{idProduct}=="1100", MODE="0666", GROUP="plugdev"
//!     SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04e2", ATTRS{idProduct}=="1200", MODE="0666", GROUP="plugdev"
//!
//!     # Add similar rules if using custom VID/PIDs
//!     # SUBSYSTEM=="hidraw", ATTRS{idVendor}=="YOUR_VID", ATTRS{idProduct}=="YOUR_PID", MODE="0666", GROUP="plugdev"
//!     ```
//!     *(Adjust `GROUP` if needed)*. Reload: `sudo udevadm control --reload-rules && sudo udevadm trigger`
//! *   **GPIO Voltage Levels:** Typically 3.3V logic.
//!
//! ## Pin Mapping
//!
//! *   Pins E0-E7 map to `GpioPin(0)`-`GpioPin(7)` (Group 0). Supported on all models.
//! *   Pins E8-E15 map to `GpioPin(8)`-`GpioPin(15)` (Group 0). Supported on XR22802/4 only.
//! *   Pins E16-E31 map to `GpioPin(16)`-`GpioPin(31)` (Group 1). Supported on XR22802/4 only.
//!
//! ## License
//!
//! This project is licensed under the WTFPL - see the [LICENSE](LICENSE) file for details.
//!
//! ## Contributing
//!
//! Contributions are welcome! Please feel free to submit pull requests or open issues on the [repository](https://github.com/your-username/xr2280x-hid) <!-- <-- CHANGE THIS -->.

use hidapi::{HidApi, HidDevice};
use log::{debug, trace, warn};
use std::convert::TryInto;
use std::ffi::{CStr, CString};

// Make internal modules private, re-export public types
mod consts;
mod error;
pub mod gpio; // Keep gpio public for its enums/structs
pub mod i2c; // Keep i2c public for its enums/structs

pub use error::{Error, Result};
pub use gpio::{GpioDirection, GpioLevel, GpioPin, GpioPull};
pub use i2c::I2cAddress;
// Re-export only essential public constants
pub use consts::{EXAR_VID, XR2280X_EDGE_PID, XR2280X_I2C_PID};

// --- Re-export necessary constants for public API use ---
/// Publicly accessible flags for controlling device features.
pub mod flags {
    /// Flags for use with [`Xr2280x::i2c_transfer_raw`].
    pub mod i2c {
        // Re-export flags needed for i2c_transfer_raw
        pub use crate::consts::i2c::out_flags::{ACK_LAST_READ, START_BIT, STOP_BIT};
    }
    // Add other flags here if needed (e.g., for interrupts if a parsing API is added)
}

// Default Timeouts
const DEFAULT_I2C_TIMEOUT_MS: i32 = 500;
const DEFAULT_INTERRUPT_TIMEOUT_MS: i32 = 1000;

// --- Device Discovery ---

/// Information about a discovered XR2280x HID device (I2C or EDGE interface).
/// Can be used with `Xr2280x::open` to connect to a specific device.
#[derive(Debug, Clone)]
pub struct XrDeviceDiscoveryInfo {
    pub vid: u16,
    pub pid: u16,
    /// The unique, platform-specific path to the HID device. Use this for reliable opening.
    pub path: CString,
    pub serial_number: Option<String>,
    pub product_string: Option<String>,
    pub interface_number: i32,
}

/// Find all connected XR2280x I2C or EDGE HID devices matching the default Exar VID.
/// Returns a list of devices. Use `Xr2280x::open` with an item from this list
/// to connect to a specific device if multiple are found.
pub fn find_all(hid_api: &HidApi) -> Result<Vec<XrDeviceDiscoveryInfo>> {
    find_devices(hid_api, consts::EXAR_VID, None)
}

/// Find the first connected XR2280x I2C or EDGE HID device matching the default Exar VID.
///
/// **Warning:** If multiple matching devices are connected, which one is considered "first"
/// is determined by the OS and `hidapi`, and may not be consistent. Use `find_all` or
/// `find_devices` followed by `Xr2280x::open` for reliable selection.
pub fn find_first(hid_api: &HidApi) -> Result<XrDeviceDiscoveryInfo> {
    find_all(hid_api)?
        .into_iter()
        .next()
        .ok_or(Error::DeviceNotFound)
}

/// Find devices matching a specific VID and optional PID.
///
/// *   If `pid` is `Some(p)`, searches for devices with the exact `vid` and `pid`.
/// *   If `pid` is `None`, searches for devices with the given `vid` and either the
///     default I2C (`0x1100`) or EDGE (`0x1200`) PID for this family.
///
/// Returns a list of matching devices. Use `Xr2280x::open` with an item from this list
/// to connect to a specific device if multiple are found.
pub fn find_devices(
    hid_api: &HidApi,
    vid: u16,
    pid: Option<u16>,
) -> Result<Vec<XrDeviceDiscoveryInfo>> {
    let mut devices = Vec::new();
    // hid_api.refresh_devices()?; // Consider if needed, might be slow
    for device_info in hid_api.device_list() {
        if device_info.vendor_id() == vid {
            let is_target_pid = match pid {
                Some(p) => device_info.product_id() == p,
                None => {
                    device_info.product_id() == consts::XR2280X_I2C_PID
                        || device_info.product_id() == consts::XR2280X_EDGE_PID
                }
            };

            if is_target_pid {
                let path = device_info.path().to_owned();
                debug!(
                    "Found matching device: VID={:04X}, PID={:04X}, Path={:?}, Interface={}",
                    device_info.vendor_id(),
                    device_info.product_id(),
                    path,
                    device_info.interface_number()
                );
                devices.push(XrDeviceDiscoveryInfo {
                    vid: device_info.vendor_id(),
                    pid: device_info.product_id(),
                    path,
                    // Corrected method name
                    serial_number: device_info.serial_number().map(String::from),
                    product_string: device_info.product_string().map(String::from),
                    interface_number: device_info.interface_number(),
                });
            }
        }
    }
    Ok(devices)
}

// --- Enums/Structs for Features ---

/// Represents the two PWM channels available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwmChannel {
    Pwm0,
    Pwm1,
}

/// Represents the operating mode/command for a PWM channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwmCommand {
    Idle,
    AssertLow,
    OneShot,
    FreeRun,
    Undefined(u16),
}

/// Represents the data received in a GPIO interrupt report.
/// **Note:** The exact format and interpretation of this data is currently unknown.
#[derive(Debug, Clone)]
pub struct GpioInterruptReport {
    pub raw_data: Vec<u8>,
}

/// Represents the data potentially parsed from a raw GPIO interrupt report.
/// **Note:** This structure is speculative and based on common HID interrupt patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGpioInterruptReport {
    pub trigger_mask_group0: u16,  // Speculative
    pub trigger_mask_group1: u16,  // Speculative
    pub current_state_group0: u16, // Assumed from first 2 bytes
    pub current_state_group1: u16, // Assumed from next 2 bytes
}

/// Holds basic information about an opened device.
#[derive(Debug, Clone)]
pub struct XrDeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: Option<String>,
    pub product_string: Option<String>,
    pub manufacturer_string: Option<String>,
}

/// Detected capabilities of the connected XR2280x device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    /// Number of GPIO pins controllable via the EDGE HID interface (8 or 32).
    pub gpio_count: u8,
}

impl Default for Capabilities {
    fn default() -> Self {
        Capabilities { gpio_count: 8 }
    }
}

/// Represents a GPIO group for bulk operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioGroup {
    Group0,
    Group1,
}

// --- Device Handle ---
/// A handle to an opened XR2280x HID device.
/// Provides methods for interacting with the I2C and EDGE (GPIO/PWM/Interrupt) controllers.
/// **Note:** This handle is not thread-safe (`!Send`, `!Sync`).
#[derive(Debug)]
pub struct Xr2280x {
    device: HidDevice,
    info: XrDeviceInfo,
    capabilities: Capabilities,
}

impl Xr2280x {
    // --- Constructors and Info ---
    /// Opens a device using its discovery info. Recommended method.
    pub fn open(hid_api: &HidApi, info: &XrDeviceDiscoveryInfo) -> Result<Self> {
        Self::open_internal(hid_api.open_path(&info.path), info.vid, info.pid)
    }

    /// Opens the first discovered default XR2280x device. **Warning:** Ambiguous if multiple devices exist.
    pub fn open_first(hid_api: &HidApi) -> Result<Self> {
        let info = find_first(hid_api)?;
        Self::open(hid_api, &info)
    }

    /// Opens a device by its Vendor ID and Product ID. **Warning:** Ambiguous if multiple devices match.
    pub fn open_by_vid_pid(hid_api: &HidApi, vid: u16, pid: u16) -> Result<Self> {
        Self::open_internal(hid_api.open(vid, pid), vid, pid)
    }

    /// Opens a device by its platform-specific path.
    pub fn open_by_path(hid_api: &HidApi, path: &CStr) -> Result<Self> {
        let device = hid_api.open_path(path)?;
        let device_info_hid = device.get_device_info().map_err(Error::Hid)?;
        Self::open_internal(
            Ok(device),
            device_info_hid.vendor_id(),
            device_info_hid.product_id(),
        )
    }

    // Internal helper for opening, storing info, and detecting capabilities
    fn open_internal(
        device_result: hidapi::HidResult<HidDevice>,
        vid: u16,
        pid: u16,
    ) -> Result<Self> {
        let device = device_result?;
        debug!("Opened XR2280x device: VID={:04X}, PID={:04X}", vid, pid);

        let manufacturer_string = device.get_manufacturer_string()?.map(|s| s.to_string());
        let product_string = device.get_product_string()?.map(|s| s.to_string());
        let serial_number = device.get_serial_number_string()?.map(|s| s.to_string());
        let info = XrDeviceInfo {
            vendor_id: vid,
            product_id: pid,
            serial_number,
            product_string,
            manufacturer_string,
        };
        trace!("Device Info: {:?}", info);

        // --- Capability Detection ---
        let temp_handle = Self {
            device,
            info: info.clone(),
            capabilities: Capabilities::default(),
        };
        let capabilities = match temp_handle.read_hid_register(consts::edge::REG_FUNC_SEL_1) {
            Ok(_) => {
                debug!("Detected support for 32 GPIOs");
                Capabilities { gpio_count: 32 }
            }
            Err(Error::FeatureReportError { .. }) => {
                debug!("Detected support for 8 GPIOs");
                Capabilities { gpio_count: 8 }
            }
            Err(e) => {
                warn!("Error during capability detection: {}", e);
                return Err(e);
            }
        };

        Ok(Self {
            device: temp_handle.device,
            info,
            capabilities,
        })
    }

    /// Gets basic information about the opened device.
    pub fn get_device_info(&self) -> Result<XrDeviceInfo> {
        Ok(self.info.clone())
    }

    /// Gets the detected capabilities (e.g., GPIO count) of the connected device.
    pub fn get_capabilities(&self) -> Capabilities {
        self.capabilities
    }

    // --- Register Access ---
    // Wrap HID errors with register context
    fn write_hid_register(&self, reg_addr: u16, value: u16) -> Result<()> {
        let buf: [u8; 5] = [
            consts::REPORT_ID_WRITE_HID_REGISTER,
            (reg_addr & 0xFF) as u8,
            ((reg_addr >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
        ];
        trace!(
            "Writing Feature Report (Write Reg {:04X} = {:04X}): {:02X?}",
            reg_addr,
            value,
            &buf[..]
        );
        match self.device.send_feature_report(&buf) {
            Ok(_) => Ok(()), // Treat any Ok as success
            Err(e) => {
                trace!("send_feature_report error: {}", e);
                Err(Error::FeatureReportError { reg_addr })
            }
        }
    }
    fn set_hid_read_address(&self, reg_addr: u16) -> Result<()> {
        let buf: [u8; 3] = [
            consts::REPORT_ID_SET_HID_READ_ADDRESS,
            (reg_addr & 0xFF) as u8,
            ((reg_addr >> 8) & 0xFF) as u8,
        ];
        trace!(
            "Writing Feature Report (Set Read Addr {:04X}): {:02X?}",
            reg_addr,
            &buf[..]
        );
        match self.device.send_feature_report(&buf) {
            Ok(_) => Ok(()), // Treat any Ok as success
            Err(e) => {
                trace!("send_feature_report error: {}", e);
                Err(Error::FeatureReportError { reg_addr: 0xFFFF })
            } // Indicate address setting failed
        }
    }
    fn read_hid_register(&self, reg_addr: u16) -> Result<u16> {
        self.set_hid_read_address(reg_addr)?;
        let mut buf = [0u8; 3];
        buf[0] = consts::REPORT_ID_READ_HID_REGISTER;
        trace!("Reading Feature Report (Read Reg Addr {:04X})", reg_addr);
        match self.device.get_feature_report(&mut buf) {
            Ok(len) if len == buf.len() => {
                if buf[0] != consts::REPORT_ID_READ_HID_REGISTER {
                    warn!(
                        "get_feature_report returned unexpected report ID: {:02X}",
                        buf[0]
                    );
                    return Err(Error::FeatureReportError { reg_addr });
                }
                let value = u16::from_le_bytes([buf[1], buf[2]]);
                trace!("Read Reg 0x{:04X} = 0x{:04X}", reg_addr, value);
                Ok(value)
            }
            Ok(len) => {
                warn!(
                    "get_feature_report returned unexpected length: {} (expected {})",
                    len,
                    buf.len()
                );
                Err(Error::FeatureReportError { reg_addr })
            }
            Err(e) => {
                trace!("get_feature_report error: {}", e);
                Err(Error::FeatureReportError { reg_addr })
            }
        }
    }

    // --- I2C Methods ---
    /// Sets the I2C bus speed (approximated). Max supported is 400 kHz.
    pub fn i2c_set_speed_khz(&self, speed_khz: u32) -> Result<()> {
        if speed_khz == 0 || speed_khz > 400 {
            return Err(Error::ArgumentOutOfRange(format!(
                "I2C speed {} kHz out of range (1-400)",
                speed_khz
            )));
        }
        let target_total_cycles = 60_000 / speed_khz;
        let low_cycles = target_total_cycles / 2;
        let high_cycles = target_total_cycles - low_cycles;
        let (min_low, min_high) = if speed_khz <= 100 {
            (252, 240)
        } else {
            (78, 36)
        };
        let final_low = low_cycles.max(min_low);
        let final_high = high_cycles.max(min_high);
        debug!(
            "Setting I2C speed ~{}kHz: SCL_LOW=0x{:04X}, SCL_HIGH=0x{:04X}",
            speed_khz, final_low, final_high
        );
        self.write_hid_register(consts::i2c::REG_SCL_LOW, final_low as u16)?;
        self.write_hid_register(consts::i2c::REG_SCL_HIGH, final_high as u16)?;
        Ok(())
    }
    /// Performs a 7-bit I2C write operation with default timeout.
    pub fn i2c_write_7bit(&self, slave_addr: u8, data: &[u8]) -> Result<()> {
        let addr = I2cAddress::new_7bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(data),
            None,
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(DEFAULT_I2C_TIMEOUT_MS),
        )
    }
    /// Performs a 10-bit I2C write operation with default timeout.
    pub fn i2c_write_10bit(&self, slave_addr: u16, data: &[u8]) -> Result<()> {
        let addr = I2cAddress::new_10bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(data),
            None,
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(DEFAULT_I2C_TIMEOUT_MS),
        )
    }
    /// Performs a 7-bit I2C read operation with default timeout.
    pub fn i2c_read_7bit(&self, slave_addr: u8, buffer: &mut [u8]) -> Result<()> {
        let addr = I2cAddress::new_7bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            None,
            Some(buffer),
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(DEFAULT_I2C_TIMEOUT_MS),
        )
    }
    /// Performs a 10-bit I2C read operation with default timeout.
    pub fn i2c_read_10bit(&self, slave_addr: u16, buffer: &mut [u8]) -> Result<()> {
        let addr = I2cAddress::new_10bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            None,
            Some(buffer),
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(DEFAULT_I2C_TIMEOUT_MS),
        )
    }
    /// Performs a 7-bit I2C write-then-read operation with default timeout.
    pub fn i2c_write_read_7bit(
        &self,
        slave_addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<()> {
        let addr = I2cAddress::new_7bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(write_data),
            Some(read_buffer),
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(DEFAULT_I2C_TIMEOUT_MS),
        )
    }
    /// Performs a 10-bit I2C write-then-read operation with default timeout.
    pub fn i2c_write_read_10bit(
        &self,
        slave_addr: u16,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<()> {
        let addr = I2cAddress::new_10bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(write_data),
            Some(read_buffer),
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(DEFAULT_I2C_TIMEOUT_MS),
        )
    }
    /// Performs a raw I2C transfer with custom flags and optional timeout. Handles both 7-bit and 10-bit addresses.
    /// Use constants from `xr2280x_hid::flags::i2c` for the `flags` argument.
    pub fn i2c_transfer_raw(
        &self,
        address: I2cAddress,
        write_data: Option<&[u8]>,
        read_buffer: Option<&mut [u8]>,
        flags: u8,
        timeout_ms: Option<i32>,
    ) -> Result<()> {
        self.i2c_transfer(
            address,
            write_data,
            read_buffer,
            flags,
            timeout_ms.unwrap_or(DEFAULT_I2C_TIMEOUT_MS),
        )
    }
    // Internal I2C transfer implementation
    fn i2c_transfer(
        &self,
        address: I2cAddress,
        write_data: Option<&[u8]>,
        read_buffer: Option<&mut [u8]>,
        flags: u8,
        read_timeout_ms: i32,
    ) -> Result<()> {
        let write_len = write_data.map_or(0, |d| d.len());
        let read_len = read_buffer.as_ref().map_or(0, |b| b.len());
        let (hid_addr_byte, extra_write_byte) = match address {
            I2cAddress::Bit7(addr) => {
                if addr > 0x7F {
                    return Err(Error::ArgumentOutOfRange("7-bit address > 0x7F".into()));
                }
                (addr, None)
            }
            I2cAddress::Bit10(addr) => {
                if addr > 0x03FF {
                    return Err(Error::InvalidI2c10BitAddress(addr));
                }
                let hid_addr = 0b1111_0000 | ((addr >> 8) & 0b11) as u8;
                let first_data_byte = (addr & 0xFF) as u8;
                (hid_addr, Some(first_data_byte))
            }
        };
        let total_write_len = write_len + if extra_write_byte.is_some() { 1 } else { 0 };
        if total_write_len > consts::i2c::REPORT_MAX_DATA_SIZE
            || read_len > consts::i2c::REPORT_MAX_DATA_SIZE
        {
            return Err(Error::OperationTooLarge {
                max: consts::i2c::REPORT_MAX_DATA_SIZE,
                actual: total_write_len.max(read_len),
            });
        }
        let mut out_buf = [0u8; consts::i2c::OUT_REPORT_WRITE_BUF_SIZE];
        out_buf[0] = flags;
        out_buf[1] = total_write_len as u8;
        out_buf[2] = read_len as u8;
        out_buf[3] = hid_addr_byte;
        let mut current_pos = 4;
        if let Some(extra_byte) = extra_write_byte {
            out_buf[current_pos] = extra_byte;
            current_pos += 1;
        }
        if let Some(data) = write_data {
            out_buf[current_pos..current_pos + write_len].copy_from_slice(data);
        }
        trace!(
            "I2C OUT Report Buffer (addr={:?}, flags=0x{:02X}): {:02X?}",
            address,
            flags,
            &out_buf[..4 + total_write_len]
        );
        let bytes_written = self.device.write(&out_buf)?;
        if bytes_written != out_buf.len() {
            warn!(
                "hidapi write returned unexpected length: {} (expected {})",
                bytes_written,
                out_buf.len()
            );
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Incomplete HID write",
            )));
        }
        let mut in_buf = [0u8; consts::i2c::IN_REPORT_READ_BUF_SIZE];
        let bytes_read = self.device.read_timeout(&mut in_buf, read_timeout_ms)?;
        if bytes_read == 0 {
            return Err(Error::I2cTimeout { address });
        }
        trace!(
            "I2C IN Report Buffer ({} bytes): {:02X?}",
            bytes_read,
            &in_buf[..bytes_read]
        );
        if bytes_read < 4 {
            warn!("Received short I2C IN report ({} bytes)", bytes_read);
            return Err(Error::InvalidReport(bytes_read));
        }
        let status_flags = in_buf[0];
        if status_flags & consts::i2c::in_flags::REQUEST_ERROR != 0 {
            return Err(Error::I2cRequestError { address });
        }
        if status_flags & consts::i2c::in_flags::NAK_RECEIVED != 0 {
            return Err(Error::I2cNack { address });
        }
        if status_flags & consts::i2c::in_flags::ARBITRATION_LOST != 0 {
            return Err(Error::I2cArbitrationLost { address });
        }
        if status_flags & consts::i2c::in_flags::TIMEOUT != 0 {
            return Err(Error::I2cTimeout { address });
        }
        if status_flags & 0b0000_1111 != 0
            && (status_flags
                & (consts::i2c::in_flags::REQUEST_ERROR
                    | consts::i2c::in_flags::NAK_RECEIVED
                    | consts::i2c::in_flags::ARBITRATION_LOST
                    | consts::i2c::in_flags::TIMEOUT))
                == 0
        {
            warn!(
                "I2C IN report indicates unknown error: flags=0x{:02X}",
                status_flags
            );
        }
        if let Some(buffer) = read_buffer {
            let actual_read_size = in_buf[2] as usize;
            if actual_read_size > read_len {
                warn!(
                    "Device reported reading {} bytes, but buffer only has space for {}",
                    actual_read_size, read_len
                );
                return Err(Error::BufferTooSmall {
                    expected: actual_read_size,
                    actual: read_len,
                });
            }
            if bytes_read < 4 + actual_read_size {
                warn!(
                    "Received IN report is too short ({}) to contain reported read data ({})",
                    bytes_read, actual_read_size
                );
                return Err(Error::InvalidReport(bytes_read));
            }
            buffer[..actual_read_size].copy_from_slice(&in_buf[4..4 + actual_read_size]);
        }
        Ok(())
    }

    // --- GPIO (EDGE) Methods ---
    // Make helpers private
    #[inline]
    fn get_gpio_group_regs(pin: GpioPin, reg0: u16, reg1: u16) -> (GpioGroup, u16) {
        if pin.group_index() == 0 {
            (GpioGroup::Group0, reg0)
        } else {
            (GpioGroup::Group1, reg1)
        }
    }
    #[inline]
    fn get_gpio_reg_for_group(group: GpioGroup, reg0: u16, reg1: u16) -> u16 {
        match group {
            GpioGroup::Group0 => reg0,
            GpioGroup::Group1 => reg1,
        }
    }
    #[inline]
    fn check_gpio_pin_support(&self, pin: GpioPin) -> Result<()> {
        if pin.number() >= self.capabilities.gpio_count {
            Err(error::unsupported_gpio_group1())
        } else {
            Ok(())
        }
    }
    #[inline]
    fn check_gpio_group_support(&self, group: GpioGroup) -> Result<()> {
        if group == GpioGroup::Group1 && self.capabilities.gpio_count < 16 {
            Err(error::unsupported_gpio_group1())
        } else {
            Ok(())
        }
    }

    // --- Single Pin GPIO ---
    /// Assigns or unassigns a pin to the EDGE controller. **Warning:** May conflict with UART functions.
    pub fn gpio_assign_to_edge(&self, pin: GpioPin, assign_to_edge: bool) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let (_, reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_FUNC_SEL_0,
            consts::edge::REG_FUNC_SEL_1,
        );
        let mask = pin.mask();
        let current_val = self.read_hid_register(reg)?;
        let new_val = if assign_to_edge {
            current_val | mask
        } else {
            current_val & !mask
        };
        if new_val != current_val {
            debug!(
                "Setting EDGE_FUNC_SEL pin {}: {}",
                pin.number(),
                assign_to_edge
            );
            self.write_hid_register(reg, new_val)?;
        } else {
            trace!(
                "EDGE_FUNC_SEL pin {} already set to {}",
                pin.number(),
                assign_to_edge
            );
        }
        Ok(())
    }
    /// Checks if a pin is currently assigned to the EDGE controller.
    pub fn gpio_is_assigned_to_edge(&self, pin: GpioPin) -> Result<bool> {
        self.check_gpio_pin_support(pin)?;
        let (_, reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_FUNC_SEL_0,
            consts::edge::REG_FUNC_SEL_1,
        );
        let reg_val = self.read_hid_register(reg)?;
        Ok((reg_val & pin.mask()) != 0)
    }
    /// Sets the direction (Input or Output) for a single GPIO pin assigned to EDGE.
    pub fn gpio_set_direction(&self, pin: GpioPin, direction: GpioDirection) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let (_, reg) =
            Self::get_gpio_group_regs(pin, consts::edge::REG_DIR_0, consts::edge::REG_DIR_1);
        let mask = pin.mask();
        let current_val = self.read_hid_register(reg)?;
        let new_val = match direction {
            GpioDirection::Output => current_val | mask,
            GpioDirection::Input => current_val & !mask,
        };
        if new_val != current_val {
            debug!("Setting EDGE_DIR pin {}: {:?}", pin.number(), direction);
            self.write_hid_register(reg, new_val)?;
        } else {
            trace!(
                "EDGE_DIR pin {} already set to {:?}",
                pin.number(),
                direction
            );
        }
        Ok(())
    }
    /// Gets the configured direction (Input or Output) for a single GPIO pin assigned to EDGE.
    pub fn gpio_get_direction(&self, pin: GpioPin) -> Result<GpioDirection> {
        self.check_gpio_pin_support(pin)?;
        let (_, reg) =
            Self::get_gpio_group_regs(pin, consts::edge::REG_DIR_0, consts::edge::REG_DIR_1);
        let reg_val = self.read_hid_register(reg)?;
        Ok(if (reg_val & pin.mask()) != 0 {
            GpioDirection::Output
        } else {
            GpioDirection::Input
        })
    }
    /// Sets the output level (High or Low) for a single GPIO pin assigned to EDGE and configured as an output.
    pub fn gpio_write(&self, pin: GpioPin, level: GpioLevel) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let mask = pin.mask();
        match level {
            GpioLevel::High => {
                let (_, set_reg) = Self::get_gpio_group_regs(
                    pin,
                    consts::edge::REG_SET_0,
                    consts::edge::REG_SET_1,
                );
                trace!(
                    "Setting EDGE pin {} HIGH (writing 0x{:04X} to reg 0x{:04X})",
                    pin.number(),
                    mask,
                    set_reg
                );
                self.write_hid_register(set_reg, mask)?;
            }
            GpioLevel::Low => {
                let (_, clear_reg) = Self::get_gpio_group_regs(
                    pin,
                    consts::edge::REG_CLEAR_0,
                    consts::edge::REG_CLEAR_1,
                );
                trace!(
                    "Setting EDGE pin {} LOW (writing 0x{:04X} to reg 0x{:04X})",
                    pin.number(),
                    mask,
                    clear_reg
                );
                self.write_hid_register(clear_reg, mask)?;
            }
        }
        Ok(())
    }
    /// Reads the current level (High or Low) of a single GPIO pin assigned to EDGE.
    pub fn gpio_read(&self, pin: GpioPin) -> Result<GpioLevel> {
        self.check_gpio_pin_support(pin)?;
        let (_, reg) =
            Self::get_gpio_group_regs(pin, consts::edge::REG_STATE_0, consts::edge::REG_STATE_1);
        let reg_val = self.read_hid_register(reg)?;
        trace!(
            "Read EDGE_STATE pin {}: reg=0x{:04X}, mask=0x{:04X}, value=0x{:04X}",
            pin.number(),
            reg,
            pin.mask(),
            reg_val
        );
        Ok(if (reg_val & pin.mask()) != 0 {
            GpioLevel::High
        } else {
            GpioLevel::Low
        })
    }
    /// Configures internal pull resistors (Up, Down, or None) for a GPIO pin assigned to EDGE.
    pub fn gpio_set_pull(&self, pin: GpioPin, pull: GpioPull) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let (_, pull_up_reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_PULL_UP_0,
            consts::edge::REG_PULL_UP_1,
        );
        let (_, pull_down_reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_PULL_DOWN_0,
            consts::edge::REG_PULL_DOWN_1,
        );
        let mask = pin.mask();
        debug!("Setting EDGE Pull pin {}: {:?}", pin.number(), pull);
        match pull {
            GpioPull::Up => {
                let cur_dn = self.read_hid_register(pull_down_reg)?;
                if (cur_dn & mask) != 0 {
                    self.write_hid_register(pull_down_reg, cur_dn & !mask)?;
                }
                let cur_up = self.read_hid_register(pull_up_reg)?;
                if (cur_up & mask) == 0 {
                    self.write_hid_register(pull_up_reg, cur_up | mask)?;
                }
            }
            GpioPull::Down => {
                let cur_up = self.read_hid_register(pull_up_reg)?;
                if (cur_up & mask) != 0 {
                    self.write_hid_register(pull_up_reg, cur_up & !mask)?;
                }
                let cur_dn = self.read_hid_register(pull_down_reg)?;
                if (cur_dn & mask) == 0 {
                    self.write_hid_register(pull_down_reg, cur_dn | mask)?;
                }
            }
            GpioPull::None => {
                let cur_up = self.read_hid_register(pull_up_reg)?;
                if (cur_up & mask) != 0 {
                    self.write_hid_register(pull_up_reg, cur_up & !mask)?;
                }
                let cur_dn = self.read_hid_register(pull_down_reg)?;
                if (cur_dn & mask) != 0 {
                    self.write_hid_register(pull_down_reg, cur_dn & !mask)?;
                }
            }
        }
        Ok(())
    }
    /// Gets the configured pull resistor state (Up, Down, or None) for a GPIO pin.
    pub fn gpio_get_pull(&self, pin: GpioPin) -> Result<GpioPull> {
        self.check_gpio_pin_support(pin)?;
        let (_, pull_up_reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_PULL_UP_0,
            consts::edge::REG_PULL_UP_1,
        );
        let (_, pull_down_reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_PULL_DOWN_0,
            consts::edge::REG_PULL_DOWN_1,
        );
        let mask = pin.mask();
        let up_enabled = (self.read_hid_register(pull_up_reg)? & mask) != 0;
        let down_enabled = (self.read_hid_register(pull_down_reg)? & mask) != 0;
        let state = match (up_enabled, down_enabled) {
            (true, false) => GpioPull::Up,
            (false, true) => GpioPull::Down,
            _ => GpioPull::None,
        };
        trace!("Read EDGE Pull pin {}: {:?}", pin.number(), state);
        Ok(state)
    }
    /// Configures a GPIO pin assigned to EDGE as an open-drain output.
    pub fn gpio_set_open_drain(&self, pin: GpioPin, enable: bool) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let (_, reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_OPEN_DRAIN_0,
            consts::edge::REG_OPEN_DRAIN_1,
        );
        let mask = pin.mask();
        let current_val = self.read_hid_register(reg)?;
        let new_val = if enable {
            current_val | mask
        } else {
            current_val & !mask
        };
        if new_val != current_val {
            debug!("Setting EDGE_OPEN_DRAIN pin {}: {}", pin.number(), enable);
            self.write_hid_register(reg, new_val)?;
        }
        Ok(())
    }
    /// Checks if a GPIO pin assigned to EDGE is configured as open-drain.
    pub fn gpio_is_open_drain(&self, pin: GpioPin) -> Result<bool> {
        self.check_gpio_pin_support(pin)?;
        let (_, reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_OPEN_DRAIN_0,
            consts::edge::REG_OPEN_DRAIN_1,
        );
        let reg_val = self.read_hid_register(reg)?;
        Ok((reg_val & pin.mask()) != 0)
    }
    /// Configures a GPIO pin assigned to EDGE as a tri-stated output (high impedance).
    pub fn gpio_set_tri_state(&self, pin: GpioPin, enable: bool) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let (_, reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_TRI_STATE_0,
            consts::edge::REG_TRI_STATE_1,
        );
        let mask = pin.mask();
        let current_val = self.read_hid_register(reg)?;
        let new_val = if enable {
            current_val | mask
        } else {
            current_val & !mask
        };
        if new_val != current_val {
            debug!("Setting EDGE_TRI_STATE pin {}: {}", pin.number(), enable);
            self.write_hid_register(reg, new_val)?;
        }
        Ok(())
    }
    /// Checks if a GPIO pin assigned to EDGE is configured as tri-stated.
    pub fn gpio_is_tri_stated(&self, pin: GpioPin) -> Result<bool> {
        self.check_gpio_pin_support(pin)?;
        let (_, reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_TRI_STATE_0,
            consts::edge::REG_TRI_STATE_1,
        );
        let reg_val = self.read_hid_register(reg)?;
        Ok((reg_val & pin.mask()) != 0)
    }

    // --- Bulk GPIO Operations ---
    /// Sets the direction for multiple GPIO pins within a group simultaneously.
    pub fn gpio_set_direction_masked(
        &self,
        group: GpioGroup,
        mask: u16,
        directions: u16,
    ) -> Result<()> {
        self.check_gpio_group_support(group)?;
        if mask == 0 {
            return Ok(());
        }
        let reg =
            Self::get_gpio_reg_for_group(group, consts::edge::REG_DIR_0, consts::edge::REG_DIR_1);
        let current_val = self.read_hid_register(reg)?;
        let new_val = (current_val & !mask) | (directions & mask);
        if new_val != current_val {
            debug!(
                "Setting EDGE_DIR group {:?}: mask=0x{:04X}, directions=0x{:04X}",
                group, mask, directions
            );
            self.write_hid_register(reg, new_val)?;
        }
        Ok(())
    }
    /// Writes output levels for multiple GPIO pins within a group simultaneously.
    pub fn gpio_write_masked(&self, group: GpioGroup, mask: u16, levels: u16) -> Result<()> {
        self.check_gpio_group_support(group)?;
        if mask == 0 {
            return Ok(());
        }
        let set_mask = mask & levels;
        let clear_mask = mask & !levels;
        if set_mask != 0 {
            let set_reg = Self::get_gpio_reg_for_group(
                group,
                consts::edge::REG_SET_0,
                consts::edge::REG_SET_1,
            );
            trace!(
                "Setting EDGE group {:?} HIGH: mask=0x{:04X}",
                group,
                set_mask
            );
            self.write_hid_register(set_reg, set_mask)?;
        }
        if clear_mask != 0 {
            let clear_reg = Self::get_gpio_reg_for_group(
                group,
                consts::edge::REG_CLEAR_0,
                consts::edge::REG_CLEAR_1,
            );
            trace!(
                "Setting EDGE group {:?} LOW: mask=0x{:04X}",
                group,
                clear_mask
            );
            self.write_hid_register(clear_reg, clear_mask)?;
        }
        Ok(())
    }
    /// Reads the state of all 16 GPIO pins within a group.
    pub fn gpio_read_group(&self, group: GpioGroup) -> Result<u16> {
        self.check_gpio_group_support(group)?;
        let reg = Self::get_gpio_reg_for_group(
            group,
            consts::edge::REG_STATE_0,
            consts::edge::REG_STATE_1,
        );
        let value = self.read_hid_register(reg)?;
        trace!("Read EDGE_STATE group {:?}: value=0x{:04X}", group, value);
        Ok(value)
    }
    /// Sets the pull resistor configuration for multiple GPIO pins within a group simultaneously.
    pub fn gpio_set_pull_masked(&self, group: GpioGroup, mask: u16, pulls: GpioPull) -> Result<()> {
        self.check_gpio_group_support(group)?;
        if mask == 0 {
            return Ok(());
        }
        let pull_up_reg = Self::get_gpio_reg_for_group(
            group,
            consts::edge::REG_PULL_UP_0,
            consts::edge::REG_PULL_UP_1,
        );
        let pull_down_reg = Self::get_gpio_reg_for_group(
            group,
            consts::edge::REG_PULL_DOWN_0,
            consts::edge::REG_PULL_DOWN_1,
        );
        debug!(
            "Setting EDGE Pull group {:?}: mask=0x{:04X}, state={:?}",
            group, mask, pulls
        );
        match pulls {
            GpioPull::Up => {
                let cur_dn = self.read_hid_register(pull_down_reg)?;
                self.write_hid_register(pull_down_reg, cur_dn & !mask)?;
                let cur_up = self.read_hid_register(pull_up_reg)?;
                self.write_hid_register(pull_up_reg, cur_up | mask)?;
            }
            GpioPull::Down => {
                let cur_up = self.read_hid_register(pull_up_reg)?;
                self.write_hid_register(pull_up_reg, cur_up & !mask)?;
                let cur_dn = self.read_hid_register(pull_down_reg)?;
                self.write_hid_register(pull_down_reg, cur_dn | mask)?;
            }
            GpioPull::None => {
                let cur_up = self.read_hid_register(pull_up_reg)?;
                self.write_hid_register(pull_up_reg, cur_up & !mask)?;
                let cur_dn = self.read_hid_register(pull_down_reg)?;
                self.write_hid_register(pull_down_reg, cur_dn & !mask)?;
            }
        }
        Ok(())
    }
    /// Sets the open-drain configuration for multiple GPIO pins within a group simultaneously.
    pub fn gpio_set_open_drain_masked(
        &self,
        group: GpioGroup,
        mask: u16,
        enables: u16,
    ) -> Result<()> {
        self.check_gpio_group_support(group)?;
        if mask == 0 {
            return Ok(());
        }
        let reg = Self::get_gpio_reg_for_group(
            group,
            consts::edge::REG_OPEN_DRAIN_0,
            consts::edge::REG_OPEN_DRAIN_1,
        );
        let current_val = self.read_hid_register(reg)?;
        let new_val = (current_val & !mask) | (enables & mask);
        if new_val != current_val {
            debug!(
                "Setting EDGE_OPEN_DRAIN group {:?}: mask=0x{:04X}, enables=0x{:04X}",
                group, mask, enables
            );
            self.write_hid_register(reg, new_val)?;
        }
        Ok(())
    }
    /// Sets the tri-state configuration for multiple GPIO pins within a group simultaneously.
    pub fn gpio_set_tri_state_masked(
        &self,
        group: GpioGroup,
        mask: u16,
        enables: u16,
    ) -> Result<()> {
        self.check_gpio_group_support(group)?;
        if mask == 0 {
            return Ok(());
        }
        let reg = Self::get_gpio_reg_for_group(
            group,
            consts::edge::REG_TRI_STATE_0,
            consts::edge::REG_TRI_STATE_1,
        );
        let current_val = self.read_hid_register(reg)?;
        let new_val = (current_val & !mask) | (enables & mask);
        if new_val != current_val {
            debug!(
                "Setting EDGE_TRI_STATE group {:?}: mask=0x{:04X}, enables=0x{:04X}",
                group, mask, enables
            );
            self.write_hid_register(reg, new_val)?;
        }
        Ok(())
    }

    // --- GPIO Interrupt Methods ---
    /// Configures interrupt generation for a single GPIO pin assigned to EDGE.
    pub fn gpio_configure_interrupt(
        &self,
        pin: GpioPin,
        mask_enable: bool,
        trigger_rising: bool,
        trigger_falling: bool,
    ) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        debug!(
            "Configuring EDGE Interrupt pin {}: mask={}, rise={}, fall={}",
            pin.number(),
            mask_enable,
            trigger_rising,
            trigger_falling
        );
        let (_, mask_reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_INTR_MASK_0,
            consts::edge::REG_INTR_MASK_1,
        );
        let (_, pos_reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_INTR_POS_EDGE_0,
            consts::edge::REG_INTR_POS_EDGE_1,
        );
        let (_, neg_reg) = Self::get_gpio_group_regs(
            pin,
            consts::edge::REG_INTR_NEG_EDGE_0,
            consts::edge::REG_INTR_NEG_EDGE_1,
        );
        let bit_mask = pin.mask();
        // Manual RMW logic
        let current_mask_val = self.read_hid_register(mask_reg)?;
        let new_mask_val = if mask_enable {
            current_mask_val | bit_mask
        } else {
            current_mask_val & !bit_mask
        };
        if new_mask_val != current_mask_val {
            trace!(
                "Writing INTR_MASK pin {}: reg=0x{:04X}, val=0x{:04X}",
                pin.number(),
                mask_reg,
                new_mask_val
            );
            self.write_hid_register(mask_reg, new_mask_val)?;
        }
        let current_pos_val = self.read_hid_register(pos_reg)?;
        let new_pos_val = if trigger_rising {
            current_pos_val | bit_mask
        } else {
            current_pos_val & !bit_mask
        };
        if new_pos_val != current_pos_val {
            trace!(
                "Writing INTR_POS_EDGE pin {}: reg=0x{:04X}, val=0x{:04X}",
                pin.number(),
                pos_reg,
                new_pos_val
            );
            self.write_hid_register(pos_reg, new_pos_val)?;
        }
        let current_neg_val = self.read_hid_register(neg_reg)?;
        let new_neg_val = if trigger_falling {
            current_neg_val | bit_mask
        } else {
            current_neg_val & !bit_mask
        };
        if new_neg_val != current_neg_val {
            trace!(
                "Writing INTR_NEG_EDGE pin {}: reg=0x{:04X}, val=0x{:04X}",
                pin.number(),
                neg_reg,
                new_neg_val
            );
            self.write_hid_register(neg_reg, new_neg_val)?;
        }
        Ok(())
    }
    /// Reads a GPIO interrupt report from the EDGE HID interface (blocking) with optional timeout.
    pub fn read_gpio_interrupt_report(
        &self,
        timeout_ms: Option<i32>,
    ) -> Result<GpioInterruptReport> {
        let timeout = timeout_ms.unwrap_or(DEFAULT_INTERRUPT_TIMEOUT_MS);
        let mut buf = [0u8; 64];
        match self.device.read_timeout(&mut buf, timeout) {
            Ok(0) => Err(Error::Timeout), // Use generic Timeout error
            Ok(bytes_read) => {
                trace!(
                    "Read GPIO Interrupt Report ({} bytes): {:02X?}",
                    bytes_read,
                    &buf[..bytes_read]
                );
                Ok(GpioInterruptReport {
                    raw_data: buf[..bytes_read].to_vec(),
                })
            }
            Err(e) => Err(Error::Hid(e)),
        }
    }
    /// Attempts to parse a raw GPIO interrupt report into a structured format. **Warning:** Speculative.
    pub fn parse_gpio_interrupt_report(
        &self,
        report: &GpioInterruptReport,
    ) -> Result<ParsedGpioInterruptReport> {
        if report.raw_data.len() < 4 {
            return Err(Error::InterruptParseError(format!(
                "Report too short ({}), expected at least 4 bytes for state",
                report.raw_data.len()
            )));
        }
        let state0 = u16::from_le_bytes(report.raw_data[0..2].try_into().unwrap());
        let state1 = u16::from_le_bytes(report.raw_data[2..4].try_into().unwrap());
        warn!("GPIO Interrupt report parsing is speculative. Trigger mask is unknown.");
        if self.capabilities.gpio_count == 8 && report.raw_data.len() >= 4 {
            warn!("Parsing interrupt report on 8-GPIO device, assuming only first 2 bytes (Group 0 state) are relevant.");
        }
        Ok(ParsedGpioInterruptReport {
            trigger_mask_group0: 0,
            trigger_mask_group1: 0,
            current_state_group0: state0,
            current_state_group1: state1,
        })
    }

    // --- PWM Methods ---
    /// Converts a duration in nanoseconds to the device's PWM counter units.
    pub fn ns_to_pwm_units(duration_ns: u64) -> Option<u16> {
        if duration_ns == 0 {
            return None;
        }
        let units_f = duration_ns as f64 / consts::edge::PWM_UNIT_TIME_NS;
        let units = units_f.round() as u32;
        if units >= consts::edge::PWM_MIN_UNITS as u32
            && units <= consts::edge::PWM_MAX_UNITS as u32
        {
            Some(units as u16)
        } else {
            warn!(
                "Duration {} ns converts to {} units, outside valid range ({}-{})",
                duration_ns,
                units,
                consts::edge::PWM_MIN_UNITS,
                consts::edge::PWM_MAX_UNITS
            );
            None
        }
    }
    /// Converts PWM counter units to an approximate duration in nanoseconds.
    pub fn pwm_units_to_ns(units: u16) -> u64 {
        (units as f64 * consts::edge::PWM_UNIT_TIME_NS).round() as u64
    }
    /// Sets the high and low period durations for a PWM channel using device units.
    pub fn pwm_set_periods(
        &self,
        channel: PwmChannel,
        high_units: u16,
        low_units: u16,
    ) -> Result<()> {
        if !(consts::edge::PWM_MIN_UNITS..=consts::edge::PWM_MAX_UNITS).contains(&high_units)
            || !(consts::edge::PWM_MIN_UNITS..=consts::edge::PWM_MAX_UNITS).contains(&low_units)
        {
            return Err(Error::ArgumentOutOfRange(
                "PWM high/low periods must be between 1 and 4095".to_string(),
            ));
        }
        let (high_reg, low_reg) = match channel {
            PwmChannel::Pwm0 => (consts::edge::REG_PWM0_HIGH, consts::edge::REG_PWM0_LOW),
            PwmChannel::Pwm1 => (consts::edge::REG_PWM1_HIGH, consts::edge::REG_PWM1_LOW),
        };
        debug!(
            "Setting PWM{:?} periods: high_units={}, low_units={}",
            channel, high_units, low_units
        );
        self.write_hid_register(high_reg, high_units)?;
        self.write_hid_register(low_reg, low_units)?;
        Ok(())
    }
    /// Sets the high and low period durations for a PWM channel using nanoseconds.
    pub fn pwm_set_periods_ns(&self, channel: PwmChannel, high_ns: u64, low_ns: u64) -> Result<()> {
        let high_units = Self::ns_to_pwm_units(high_ns).ok_or_else(|| {
            Error::ArgumentOutOfRange(format!("High duration {} ns is invalid", high_ns))
        })?;
        let low_units = Self::ns_to_pwm_units(low_ns).ok_or_else(|| {
            Error::ArgumentOutOfRange(format!("Low duration {} ns is invalid", low_ns))
        })?;
        self.pwm_set_periods(channel, high_units, low_units)
    }
    /// Gets the configured high and low period durations for a PWM channel in device units.
    pub fn pwm_get_periods(&self, channel: PwmChannel) -> Result<(u16, u16)> {
        let (high_reg, low_reg) = match channel {
            PwmChannel::Pwm0 => (consts::edge::REG_PWM0_HIGH, consts::edge::REG_PWM0_LOW),
            PwmChannel::Pwm1 => (consts::edge::REG_PWM1_HIGH, consts::edge::REG_PWM1_LOW),
        };
        let high_units = self.read_hid_register(high_reg)?;
        let low_units = self.read_hid_register(low_reg)?;
        trace!(
            "Read PWM{:?} periods: high={}, low={}",
            channel,
            high_units,
            low_units
        );
        Ok((high_units, low_units))
    }
    /// Gets the configured high and low period durations for a PWM channel in nanoseconds (approximate).
    pub fn pwm_get_periods_ns(&self, channel: PwmChannel) -> Result<(u64, u64)> {
        let (high_units, low_units) = self.pwm_get_periods(channel)?;
        let high_ns = Self::pwm_units_to_ns(high_units);
        let low_ns = Self::pwm_units_to_ns(low_units);
        trace!(
            "Read PWM{:?} periods: high_ns={}, low_ns={}",
            channel,
            high_ns,
            low_ns
        );
        Ok((high_ns, low_ns))
    }
    /// Assigns a PWM channel output to a specific EDGE GPIO pin.
    pub fn pwm_set_pin(&self, channel: PwmChannel, pin: GpioPin) -> Result<()> {
        if pin.number() >= self.capabilities.gpio_count {
            return Err(error::unsupported_pwm_pin(pin.number()));
        }
        let ctrl_reg = match channel {
            PwmChannel::Pwm0 => consts::edge::REG_PWM0_CTRL,
            PwmChannel::Pwm1 => consts::edge::REG_PWM1_CTRL,
        };
        debug!("Assigning PWM{:?} output to pin {}", channel, pin.number());
        let pin_val: u16 = pin.number().into();
        let value =
            (pin_val << consts::edge::pwm_ctrl::PIN_SHIFT) & consts::edge::pwm_ctrl::PIN_MASK;
        let mask = consts::edge::pwm_ctrl::PIN_MASK;
        let current_ctrl = self.read_hid_register(ctrl_reg)?;
        let new_ctrl = (current_ctrl & !mask) | (value & mask);
        if new_ctrl != current_ctrl {
            self.write_hid_register(ctrl_reg, new_ctrl)?;
        }
        Ok(())
    }
    /// Gets the GPIO pin assigned to a PWM channel output.
    pub fn pwm_get_pin(&self, channel: PwmChannel) -> Result<GpioPin> {
        let ctrl_reg = match channel {
            PwmChannel::Pwm0 => consts::edge::REG_PWM0_CTRL,
            PwmChannel::Pwm1 => consts::edge::REG_PWM1_CTRL,
        };
        let ctrl_val = self.read_hid_register(ctrl_reg)?;
        let pin_num = ((ctrl_val & consts::edge::pwm_ctrl::PIN_MASK)
            >> consts::edge::pwm_ctrl::PIN_SHIFT) as u8;
        trace!("Read PWM{:?} pin assignment: {}", channel, pin_num);
        GpioPin::new(pin_num)
    }
    /// Sets the command mode and enables/disables a PWM channel.
    pub fn pwm_control(
        &self,
        channel: PwmChannel,
        command: PwmCommand,
        enable: bool,
    ) -> Result<()> {
        let ctrl_reg = match channel {
            PwmChannel::Pwm0 => consts::edge::REG_PWM0_CTRL,
            PwmChannel::Pwm1 => consts::edge::REG_PWM1_CTRL,
        };
        let cmd_val = match command {
            PwmCommand::Idle => consts::edge::pwm_ctrl::CMD_IDLE,
            PwmCommand::AssertLow => consts::edge::pwm_ctrl::CMD_ASSERT_LOW,
            PwmCommand::OneShot => consts::edge::pwm_ctrl::CMD_ONE_SHOT,
            PwmCommand::FreeRun => consts::edge::pwm_ctrl::CMD_FREE_RUN,
            PwmCommand::Undefined(_) => {
                return Err(Error::ArgumentOutOfRange(
                    "Cannot set Undefined PWM command".into(),
                ))
            }
        };
        let enable_val = if enable {
            consts::edge::pwm_ctrl::ENABLE_MASK
        } else {
            0
        };
        let mask = consts::edge::pwm_ctrl::CMD_MASK | consts::edge::pwm_ctrl::ENABLE_MASK;
        let value = ((cmd_val << consts::edge::pwm_ctrl::CMD_SHIFT)
            & consts::edge::pwm_ctrl::CMD_MASK)
            | enable_val;
        debug!(
            "Setting PWM{:?} control: command={:?}, enable={}",
            channel, command, enable
        );
        let current_ctrl = self.read_hid_register(ctrl_reg)?;
        let new_ctrl = (current_ctrl & !mask) | (value & mask);
        if new_ctrl != current_ctrl {
            self.write_hid_register(ctrl_reg, new_ctrl)?;
        } else {
            trace!(
                "PWM{:?} control already set to command={:?}, enable={}",
                channel,
                command,
                enable
            );
        }
        Ok(())
    }
    /// Gets the current command mode and enabled state for a PWM channel.
    pub fn pwm_get_control(&self, channel: PwmChannel) -> Result<(PwmCommand, bool)> {
        let ctrl_reg = match channel {
            PwmChannel::Pwm0 => consts::edge::REG_PWM0_CTRL,
            PwmChannel::Pwm1 => consts::edge::REG_PWM1_CTRL,
        };
        let ctrl_val = self.read_hid_register(ctrl_reg)?;
        let enabled = (ctrl_val & consts::edge::pwm_ctrl::ENABLE_MASK) != 0;
        let cmd_bits =
            (ctrl_val & consts::edge::pwm_ctrl::CMD_MASK) >> consts::edge::pwm_ctrl::CMD_SHIFT;
        let command = match cmd_bits {
            consts::edge::pwm_ctrl::CMD_IDLE => PwmCommand::Idle,
            consts::edge::pwm_ctrl::CMD_ASSERT_LOW => PwmCommand::AssertLow,
            consts::edge::pwm_ctrl::CMD_ONE_SHOT => PwmCommand::OneShot,
            consts::edge::pwm_ctrl::CMD_FREE_RUN => PwmCommand::FreeRun,
            _ => PwmCommand::Undefined(cmd_bits),
        };
        trace!(
            "Read PWM{:?} control: command={:?}, enabled={}",
            channel,
            command,
            enabled
        );
        Ok((command, enabled))
    }
} // impl Xr2280x

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_gpio_pin_creation() { /* ... */
    }
    #[test]
    fn test_gpio_pin_helpers() { /* ... */
    }
    #[test]
    fn test_i2c_address_creation() { /* ... */
    }

    #[test]
    fn test_pwm_unit_conversion() {
        let unit_ns = consts::edge::PWM_UNIT_TIME_NS; // ~266.667

        // Test known good value (datasheet period)
        assert_relative_eq!(Xr2280x::pwm_units_to_ns(1) as f64, unit_ns, epsilon = 1.0);
        assert_eq!(Xr2280x::ns_to_pwm_units(unit_ns.round() as u64), Some(1));

        // Test boundaries
        assert_eq!(Xr2280x::ns_to_pwm_units(0), None); // Zero duration

        // Test value just below minimum unit threshold (should map to None or 1 depending on rounding)
        let ns_below_min = (unit_ns * 0.4).round() as u64;
        assert!(
            Xr2280x::ns_to_pwm_units(ns_below_min).is_none_or(|u| u == 1),
            "Value near zero failed"
        ); // Allow None or 1

        // Test value just above minimum unit threshold (should map to 1)
        let ns_at_min_boundary = (unit_ns * 0.6).round() as u64;
        assert_eq!(
            Xr2280x::ns_to_pwm_units(ns_at_min_boundary),
            Some(1),
            "Value near 1 failed"
        );

        // Test maximum value
        let ns_at_max = Xr2280x::pwm_units_to_ns(consts::edge::PWM_MAX_UNITS);
        assert_eq!(
            Xr2280x::ns_to_pwm_units(ns_at_max),
            Some(consts::edge::PWM_MAX_UNITS),
            "Max value failed"
        );

        // Test value just above maximum threshold (should map to None)
        // Calculate ns for 4095 units, add half a unit's duration in ns, then round.
        // This ensures the division inside ns_to_pwm_units results in > 4095.5
        let ns_above_max = (ns_at_max as f64 + unit_ns * 0.6).round() as u64;
        assert!(
            Xr2280x::ns_to_pwm_units(ns_above_max).is_none(),
            "Value above max failed ({} ns)",
            ns_above_max
        );

        // Test intermediate value
        let target_ns = 1_000_000; // 1ms
        let expected_units = (target_ns as f64 / unit_ns).round() as u16; // ~3750
        assert_eq!(Xr2280x::ns_to_pwm_units(target_ns), Some(expected_units));
        assert_relative_eq!(
            Xr2280x::pwm_units_to_ns(expected_units) as f64,
            target_ns as f64,
            epsilon = unit_ns
        ); // Check reverse conversion within one unit tolerance
    }
}
