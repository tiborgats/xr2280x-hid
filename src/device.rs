//! Device discovery and management functionality for XR2280x HID devices.

use crate::consts;
use crate::error::{Error, Result};
use hidapi::{HidApi, HidDevice};
use log::{debug, trace, warn};
use std::ffi::CStr;

/// Information about a discovered XR2280x device.
///
/// This struct represents a complete device that may expose multiple
/// USB HID interfaces (I2C and EDGE). This is the recommended structure for
/// device enumeration as it groups logical interfaces by device.
#[derive(Debug, Clone)]
pub struct XrDeviceInfo {
    /// USB vendor ID (0x04E2 for Exar Corporation).
    pub vid: u16,
    /// Device serial number string (used to group interfaces).
    pub serial_number: Option<String>,
    /// Human-readable product name/description.
    pub product_string: Option<String>,
    /// I2C interface information if available.
    pub i2c_interface: Option<InterfaceInfo>,
    /// EDGE (GPIO/PWM/Interrupt) interface information if available.
    pub edge_interface: Option<InterfaceInfo>,
}

/// Finds all XR2280x devices.
/// Returns a vector of device info, with logical interfaces grouped by device.
pub fn device_find_all(hid_api: &HidApi) -> Result<Vec<XrDeviceInfo>> {
    Ok(device_find(hid_api).collect())
}

/// Finds the first XR2280x device.
/// Returns an error if no device is found.
/// **Warning:** Ambiguous if multiple devices exist.
pub fn device_find_first(hid_api: &HidApi) -> Result<XrDeviceInfo> {
    device_find_all(hid_api)?
        .into_iter()
        .next()
        .ok_or(Error::DeviceNotFound)
}

/// Finds XR2280x devices by grouping logical interfaces by serial number.
/// Returns an iterator of devices with deterministic ordering by serial number.
pub fn device_find(hid_api: &HidApi) -> impl Iterator<Item = XrDeviceInfo> + '_ {
    use std::collections::HashMap;

    // First, collect all logical interfaces
    let mut devices_by_serial: HashMap<String, XrDeviceInfo> = HashMap::new();
    let mut devices_without_serial: Vec<XrDeviceInfo> = Vec::new();

    for info in find_logical_devices(hid_api) {
        if let Some(serial) = &info.serial_number {
            let device = devices_by_serial
                .entry(serial.clone())
                .or_insert_with(|| XrDeviceInfo {
                    vid: info.vid,
                    serial_number: info.serial_number.clone(),
                    product_string: info.product_string.clone(),
                    i2c_interface: None,
                    edge_interface: None,
                });

            // Assign to appropriate interface based on PID
            if info.pid == consts::XR2280X_I2C_PID {
                device.i2c_interface = Some(info);
            } else if info.pid == consts::XR2280X_EDGE_PID {
                device.edge_interface = Some(info);
            }
        } else {
            // Handle devices without serial numbers (create separate entries)
            let mut device = XrDeviceInfo {
                vid: info.vid,
                serial_number: None,
                product_string: info.product_string.clone(),
                i2c_interface: None,
                edge_interface: None,
            };

            if info.pid == consts::XR2280X_I2C_PID {
                device.i2c_interface = Some(info);
            } else if info.pid == consts::XR2280X_EDGE_PID {
                device.edge_interface = Some(info);
            }

            devices_without_serial.push(device);
        }
    }

    // Collect and sort devices deterministically
    let mut all_devices: Vec<XrDeviceInfo> = devices_by_serial.into_values().collect();
    all_devices.extend(devices_without_serial);

    // Sort by serial number for deterministic ordering
    all_devices.sort_by(|a, b| {
        match (&a.serial_number, &b.serial_number) {
            (Some(a_serial), Some(b_serial)) => a_serial.cmp(b_serial),
            (Some(_), None) => std::cmp::Ordering::Less, // Devices with serial come first
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    all_devices.into_iter()
}

/// Interface information for a single USB HID interface.
#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub vid: u16,
    pub pid: u16,
    pub path: std::ffi::CString,
    pub serial_number: Option<String>,
    pub product_string: Option<String>,
    pub interface_number: i32,
}

/// Internal helper function for finding logical devices.
/// Used internally by hardware device enumeration.
fn find_logical_devices(hid_api: &HidApi) -> impl Iterator<Item = InterfaceInfo> + '_ {
    hid_api
        .device_list()
        .filter(|info| {
            info.vendor_id() == consts::EXAR_VID
                && (info.product_id() == consts::XR2280X_I2C_PID
                    || info.product_id() == consts::XR2280X_EDGE_PID)
        })
        .map(|info| {
            debug!(
                "Found XR2280x logical device: VID={:04X}, PID={:04X}, Path={:?}, SN={:?}",
                info.vendor_id(),
                info.product_id(),
                info.path(),
                info.serial_number()
            );
            InterfaceInfo {
                vid: info.vendor_id(),
                pid: info.product_id(),
                path: info.path().to_owned(),
                serial_number: info.serial_number().map(|s| s.to_string()),
                product_string: info.product_string().map(|s| s.to_string()),
                interface_number: info.interface_number(),
            }
        })
}

/// Holds basic information about an opened device.
/// Detailed information about an opened XR2280x device.
///
/// This struct provides comprehensive device identification and capability
/// information for an actively connected XR2280x device.
#[derive(Debug, Clone)]
pub struct XrDeviceDetails {
    /// USB vendor ID (0x04E2 for Exar Corporation).
    pub vendor_id: u16,
    /// USB product ID identifying the device interface type.
    pub product_id: u16,
    /// Unique serial number string for this device instance.
    pub serial_number: Option<String>,
    /// Human-readable product name/description.
    pub product_string: Option<String>,
    /// Manufacturer name string (typically "Exar Corporation").
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

/// A handle to an opened XR2280x hardware device.
/// Provides methods for interacting with both I2C and EDGE (GPIO/PWM/Interrupt) controllers.
/// **Note:** This handle is not thread-safe (`!Send`, `!Sync`).
#[derive(Debug)]
pub struct Xr2280x {
    pub(crate) i2c_device: Option<HidDevice>,
    pub(crate) edge_device: Option<HidDevice>,
    pub(crate) info: XrDeviceDetails,
    pub(crate) capabilities: Capabilities,
}

impl Xr2280x {
    // --- Constructors and Info ---

    /// Enumerate all available XR2280x devices.
    /// Returns a Vec of device info with logical interfaces grouped by device.
    pub fn device_enumerate(hid_api: &HidApi) -> Result<Vec<XrDeviceInfo>> {
        device_find_all(hid_api)
    }

    /// Opens a device using its device info. Recommended method.
    /// This opens both I2C and EDGE interfaces if available.
    pub fn device_open(hid_api: &HidApi, info: &XrDeviceInfo) -> Result<Self> {
        let i2c_device =
            if let Some(i2c_info) = &info.i2c_interface {
                Some(hid_api.open_path(&i2c_info.path).map_err(|e| {
                    Error::DeviceNotFoundByPath {
                        path: format!("{:?}", i2c_info.path),
                        message: format!("Failed to open I2C interface: {}", e),
                    }
                })?)
            } else {
                None
            };

        let edge_device =
            if let Some(edge_info) = &info.edge_interface {
                Some(hid_api.open_path(&edge_info.path).map_err(|e| {
                    Error::DeviceNotFoundByPath {
                        path: format!("{:?}", edge_info.path),
                        message: format!("Failed to open EDGE interface: {}", e),
                    }
                })?)
            } else {
                None
            };

        if i2c_device.is_none() && edge_device.is_none() {
            return Err(Error::DeviceNotFound);
        }

        Self::from_hid_devices(i2c_device, edge_device)
    }

    /// Opens the first device found. Convenient but ambiguous if multiple devices exist.
    pub fn device_open_first(hid_api: &HidApi) -> Result<Self> {
        let info = device_find_first(hid_api)?;
        Self::device_open(hid_api, &info)
    }

    /// Opens a device by its Vendor ID and Product ID. **Warning:** Ambiguous if multiple devices match.
    pub fn open_by_vid_pid(hid_api: &HidApi, vid: u16, pid: u16) -> Result<Self> {
        let device = hid_api.open(vid, pid)?;

        // Determine which interface this is and assign appropriately
        if pid == consts::XR2280X_I2C_PID {
            Self::from_hid_devices(Some(device), None)
        } else if pid == consts::XR2280X_EDGE_PID {
            Self::from_hid_devices(None, Some(device))
        } else {
            Self::from_hid_devices(Some(device), None) // Default to I2C for unknown PIDs
        }
    }

    /// Opens a device by its platform-specific path.
    pub fn open_by_path(hid_api: &HidApi, path: &CStr) -> Result<Self> {
        let device = hid_api
            .open_path(path)
            .map_err(|e| Error::DeviceNotFoundByPath {
                path: format!("{:?}", path),
                message: format!("{}", e),
            })?;

        // Get device info to determine which interface this is
        let device_info_hid = device.get_device_info().map_err(Error::Hid)?;
        let pid = device_info_hid.product_id();

        // Determine which interface this is and assign appropriately
        if pid == consts::XR2280X_I2C_PID {
            Self::from_hid_devices(Some(device), None)
        } else if pid == consts::XR2280X_EDGE_PID {
            Self::from_hid_devices(None, Some(device))
        } else {
            Self::from_hid_devices(Some(device), None) // Default to I2C for unknown PIDs
        }
    }

    /// Opens a device by its serial number.
    /// Searches through all XR2280x devices to find one with the matching serial number.
    pub fn open_by_serial(hid_api: &HidApi, serial: &str) -> Result<Self> {
        let devices = Self::device_enumerate(hid_api)?;

        for device_info in devices {
            if let Some(device_serial) = &device_info.serial_number {
                if device_serial == serial {
                    return Self::device_open(hid_api, &device_info);
                }
            }
        }

        Err(Error::DeviceNotFoundBySerial {
            serial: serial.to_string(),
            message: "No XR2280x device found with this serial number".to_string(),
        })
    }

    /// Opens a device by its index in the enumeration order.
    /// Index is 0-based and corresponds to the order returned by device_enumerate().
    pub fn open_by_index(hid_api: &HidApi, index: usize) -> Result<Self> {
        let devices = Self::device_enumerate(hid_api)?;

        if index >= devices.len() {
            return Err(Error::DeviceNotFoundByIndex {
                index,
                message: format!("Index out of range (found {} devices)", devices.len()),
            });
        }

        Self::device_open(hid_api, &devices[index])
    }

    /// Creates an Xr2280x instance from existing HidDevice handles.
    /// This is the core method that other constructors use internally.
    ///
    /// # Arguments
    /// * `i2c_device` - An already opened HidDevice handle for I2C interface (optional)
    /// * `edge_device` - An already opened HidDevice handle for EDGE interface (optional)
    ///
    /// # Returns
    /// A configured Xr2280x instance with capabilities detected.
    pub fn from_hid_devices(
        i2c_device: Option<HidDevice>,
        edge_device: Option<HidDevice>,
    ) -> Result<Self> {
        // Use the first available device for device info extraction
        let info_device = if let Some(ref device) = edge_device {
            device
        } else if let Some(ref device) = i2c_device {
            device
        } else {
            return Err(Error::DeviceNotFound);
        };

        let device_info_hid = info_device.get_device_info().map_err(Error::Hid)?;
        let vid = device_info_hid.vendor_id();

        debug!("Creating XR2280x from HidDevices: VID={:04X}", vid);

        let manufacturer_string = info_device
            .get_manufacturer_string()?
            .map(|s| s.to_string());
        let product_string = info_device.get_product_string()?.map(|s| s.to_string());
        let serial_number = info_device
            .get_serial_number_string()?
            .map(|s| s.to_string());
        let info = XrDeviceDetails {
            vendor_id: vid,
            product_id: 0, // Not meaningful for hardware device
            serial_number,
            product_string,
            manufacturer_string,
        };
        trace!("Hardware Device Info: {:?}", info);

        // --- Capability Detection ---
        let temp_handle = Self {
            i2c_device,
            edge_device,
            info: info.clone(),
            capabilities: Capabilities::default(),
        };

        let capabilities = if temp_handle.edge_device.is_some() {
            match temp_handle.read_hid_register(consts::edge::REG_FUNC_SEL_1) {
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
            }
        } else {
            debug!("No EDGE interface available, assuming 8 GPIOs");
            Capabilities { gpio_count: 8 }
        };

        Ok(Self {
            i2c_device: temp_handle.i2c_device,
            edge_device: temp_handle.edge_device,
            info,
            capabilities,
        })
    }

    /// Gets basic information about the opened device.
    pub fn get_device_info(&self) -> XrDeviceDetails {
        self.info.clone()
    }

    /// Gets the detected capabilities (e.g., GPIO count) of the connected device.
    pub fn get_capabilities(&self) -> Capabilities {
        self.capabilities
    }

    // --- Register Access ---
    // Wrap HID errors with register context
    pub(crate) fn write_hid_register(&self, reg_addr: u16, value: u16) -> Result<()> {
        // Determine which device to use based on register address
        let device = if (0x0340..=0x0342).contains(&reg_addr) {
            // I2C registers
            self.i2c_device.as_ref().ok_or(Error::DeviceNotFound)?
        } else {
            // EDGE registers (GPIO/PWM/Interrupt)
            self.edge_device.as_ref().ok_or(Error::DeviceNotFound)?
        };

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
        match device.send_feature_report(&buf) {
            Ok(_) => Ok(()), // Treat any Ok as success
            Err(e) => {
                trace!("send_feature_report error: {}", e);
                Err(Error::FeatureReportError { reg_addr })
            }
        }
    }

    pub(crate) fn set_hid_read_address(&self, reg_addr: u16) -> Result<()> {
        // Determine which device to use based on register address
        let device = if (0x0340..=0x0342).contains(&reg_addr) {
            // I2C registers
            self.i2c_device.as_ref().ok_or(Error::DeviceNotFound)?
        } else {
            // EDGE registers (GPIO/PWM/Interrupt)
            self.edge_device.as_ref().ok_or(Error::DeviceNotFound)?
        };

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
        match device.send_feature_report(&buf) {
            Ok(_) => Ok(()), // Treat any Ok as success
            Err(e) => {
                trace!("send_feature_report error: {}", e);
                Err(Error::FeatureReportError { reg_addr: 0xFFFF })
            } // Indicate address setting failed
        }
    }

    pub(crate) fn read_hid_register(&self, reg_addr: u16) -> Result<u16> {
        self.set_hid_read_address(reg_addr)?;

        // Determine which device to use based on register address
        let device = if (0x0340..=0x0342).contains(&reg_addr) {
            // I2C registers
            self.i2c_device.as_ref().ok_or(Error::DeviceNotFound)?
        } else {
            // EDGE registers (GPIO/PWM/Interrupt)
            self.edge_device.as_ref().ok_or(Error::DeviceNotFound)?
        };

        let mut buf = [0u8; 3];
        buf[0] = consts::REPORT_ID_READ_HID_REGISTER;
        trace!("Reading Feature Report (Read Reg Addr {:04X})", reg_addr);
        match device.get_feature_report(&mut buf) {
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
}
