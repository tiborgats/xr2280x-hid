//! Device discovery and management functionality for XR2280x HID devices.

use crate::consts;
use crate::error::{Error, Result};
use hidapi::{HidApi, HidDevice};
use log::{debug, trace, warn};
use std::ffi::CStr;

/// Information about a discovered XR2280x HID device.
///
/// This struct contains the essential information needed to identify and
/// connect to a specific XR2280x device found during enumeration.
#[derive(Debug, Clone)]
pub struct XrDeviceDiscoveryInfo {
    /// USB vendor ID (0x04E2 for Exar Corporation).
    pub vid: u16,
    /// USB product ID identifying the device interface type.
    pub pid: u16,
    /// Platform-specific path to the device (e.g., for `HidApi::open_path`).
    pub path: std::ffi::CString,
    /// Device serial number string if available.
    pub serial_number: Option<String>,
    /// Human-readable product name/description if available.
    pub product_string: Option<String>,
    /// USB interface number for this HID interface.
    pub interface_number: i32,
}

/// Finds all XR2280x I2C or EDGE HID devices using default VID/PIDs.
/// Returns a vector of discovery info.
pub fn find_all(hid_api: &HidApi) -> Result<Vec<XrDeviceDiscoveryInfo>> {
    Ok(find_devices(hid_api).collect())
}

/// Finds the first XR2280x I2C or EDGE HID device using default VID/PIDs.
/// Returns an error if no device is found.
/// **Warning:** Ambiguous if multiple devices exist.
pub fn find_first(hid_api: &HidApi) -> Result<XrDeviceDiscoveryInfo> {
    find_all(hid_api)?
        .into_iter()
        .next()
        .ok_or(Error::DeviceNotFound)
}

/// Finds XR2280x devices matching common interface PIDs.
/// This is a more flexible alternative to `open` that allows filtering by VID/PID.
/// Searches for I2C (0x1100) and EDGE (0x1200) interfaces by default.
pub fn find_devices(hid_api: &HidApi) -> impl Iterator<Item = XrDeviceDiscoveryInfo> + '_ {
    hid_api
        .device_list()
        .filter(|info| {
            info.vendor_id() == consts::EXAR_VID
                && (info.product_id() == consts::XR2280X_I2C_PID
                    || info.product_id() == consts::XR2280X_EDGE_PID)
        })
        .map(|info| {
            debug!(
                "Found XR2280x device: VID={:04X}, PID={:04X}, Path={:?}, SN={:?}",
                info.vendor_id(),
                info.product_id(),
                info.path(),
                info.serial_number()
            );
            XrDeviceDiscoveryInfo {
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
pub struct XrDeviceInfo {
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

/// A handle to an opened XR2280x HID device.
/// Provides methods for interacting with the I2C and EDGE (GPIO/PWM/Interrupt) controllers.
/// **Note:** This handle is not thread-safe (`!Send`, `!Sync`).
#[derive(Debug)]
pub struct Xr2280x {
    pub(crate) device: HidDevice,
    pub(crate) info: XrDeviceInfo,
    pub(crate) capabilities: Capabilities,
}

impl Xr2280x {
    // --- Constructors and Info ---

    /// Enumerate all available XR2280x devices.
    /// Returns a Vec of DeviceInfo for all compatible devices found.
    pub fn enumerate_devices(hid_api: &HidApi) -> Result<Vec<&hidapi::DeviceInfo>> {
        let devices: Vec<_> = hid_api
            .device_list()
            .filter(|info| {
                info.vendor_id() == consts::EXAR_VID
                    && (info.product_id() == consts::XR2280X_I2C_PID
                        || info.product_id() == consts::XR2280X_EDGE_PID)
            })
            .collect();
        Ok(devices)
    }

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
        let device = hid_api
            .open_path(path)
            .map_err(|e| Error::DeviceNotFoundByPath {
                path: format!("{:?}", path),
                message: format!("{}", e),
            })?;
        Self::from_hid_device(device)
    }

    /// Opens a device by its serial number.
    /// Searches through all XR2280x devices to find one with the matching serial number.
    pub fn open_by_serial(hid_api: &HidApi, serial: &str) -> Result<Self> {
        let devices = Self::enumerate_devices(hid_api)?;

        for device_info in devices {
            if let Some(device_serial) = device_info.serial_number() {
                if device_serial == serial {
                    let device = hid_api.open_path(device_info.path()).map_err(|e| {
                        Error::DeviceNotFoundBySerial {
                            serial: serial.to_string(),
                            message: format!("Failed to open device: {}", e),
                        }
                    })?;
                    return Self::from_hid_device(device);
                }
            }
        }

        Err(Error::DeviceNotFoundBySerial {
            serial: serial.to_string(),
            message: "No XR2280x device found with this serial number".to_string(),
        })
    }

    /// Opens a device by its index in the enumeration order.
    /// Index is 0-based and corresponds to the order returned by enumerate_devices().
    pub fn open_by_index(hid_api: &HidApi, index: usize) -> Result<Self> {
        let devices = Self::enumerate_devices(hid_api)?;

        if index >= devices.len() {
            return Err(Error::DeviceNotFoundByIndex {
                index,
                message: format!("Index out of range (found {} devices)", devices.len()),
            });
        }

        let device_info = devices[index];
        let device =
            hid_api
                .open_path(device_info.path())
                .map_err(|e| Error::DeviceNotFoundByIndex {
                    index,
                    message: format!("Failed to open device: {}", e),
                })?;

        Self::from_hid_device(device)
    }

    /// Creates an Xr2280x instance from an existing HidDevice.
    /// This is the core method that other constructors use internally.
    ///
    /// # Arguments
    /// * `device` - An already opened HidDevice handle
    ///
    /// # Returns
    /// A configured Xr2280x instance with capabilities detected.
    pub fn from_hid_device(device: HidDevice) -> Result<Self> {
        let device_info_hid = device.get_device_info().map_err(Error::Hid)?;
        let vid = device_info_hid.vendor_id();
        let pid = device_info_hid.product_id();

        debug!(
            "Creating XR2280x from HidDevice: VID={:04X}, PID={:04X}",
            vid, pid
        );

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

    // Internal helper for opening with error conversion
    fn open_internal(
        device_result: hidapi::HidResult<HidDevice>,
        _vid: u16,
        _pid: u16,
    ) -> Result<Self> {
        let device = device_result?;
        Self::from_hid_device(device)
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
    pub(crate) fn write_hid_register(&self, reg_addr: u16, value: u16) -> Result<()> {
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

    pub(crate) fn set_hid_read_address(&self, reg_addr: u16) -> Result<()> {
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

    pub(crate) fn read_hid_register(&self, reg_addr: u16) -> Result<u16> {
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
}
