//! I2C communication functionality for XR2280x devices.

use crate::consts;
use crate::device::Xr2280x;
use crate::error::{Error, Result};
use crate::flags;
use log::{debug, trace, warn};
use std::fmt;
use std::time::Instant;

/// Default timeouts for different I2C operations (in milliseconds).
///
/// These constants provide operation-specific defaults that balance performance with reliability.
/// Unlike traditional I2C where ACK/NACK responses are instantaneous, HID-based I2C communication
/// involves multiple layers (USB, HID, firmware) that introduce latency and potential delays.
///
/// # Why Timeouts Are Necessary
///
/// HID-based I2C communication goes through several layers:
/// ```text
/// Your Code → HID API → USB Stack → XR2280x Firmware → I2C Hardware → Target Device
/// ```
///
/// Each layer can introduce delays due to:
/// - USB packet transmission and scheduling
/// - Firmware processing time
/// - I2C clock stretching by slow devices
/// - USB bus congestion or interference
/// - **Firmware blocking** when I2C bus is stuck (critical issue)
///
/// # Choosing the Right Timeout
///
/// | Operation Type | Recommended Timeout | Use Case |
/// |---------------|-------------------|----------|
/// | [`timeouts::PROBE`] (3ms) | Ultra-fast probing | Detect firmware responsiveness |
/// | [`timeouts::SCAN`] (8ms) | Device discovery | Fast scanning with stuck bus protection |
/// | [`timeouts::READ`] (100ms) | Sensor readings | Most sensor and register reads |
/// | [`timeouts::WRITE`] (200ms) | Configuration | Writing to device registers |
/// | [`timeouts::WRITE_READ`] (250ms) | Register access | Combined write-then-read operations |
/// | [`timeouts::EEPROM_WRITE`] (5000ms) | EEPROM programming | Slow memory operations |
/// | Custom (5-10000ms) | Special cases | Application-specific requirements |
///
/// # Example Usage
/// ```no_run
/// # use xr2280x_hid::*;
/// # use hidapi::HidApi;
/// # fn main() -> Result<()> {
/// # let hid_api = HidApi::new()?;
/// # let device = Xr2280x::device_open_first(&hid_api)?;
/// # let mut buffer = [0u8; 2];
/// # let data = [0x42];
/// // Fast scanning for responsive devices
/// let devices = device.i2c_scan_with_progress_and_timeout(0x08, 0x77, timeouts::SCAN, |_, _, _, _| {})?;
///
/// // Quick sensor read with default timeout
/// device.i2c_read_7bit(0x48, &mut buffer)?;
///
/// // Custom timeout for real-time applications
/// device.i2c_read_7bit_with_timeout(0x48, &mut buffer, 50)?; // 50ms max
///
/// // EEPROM write with extended timeout
/// device.i2c_eeprom_write_7bit(0x50, &data)?;
/// # Ok(())
/// # }
/// ```
///
/// # Robust Stuck Bus Detection
///
/// The scanning functions include multi-layered stuck bus detection that prevents
/// the 29+ second hangs that occur when unpowered devices hold I2C lines low:
///
/// 1. **Ultra-fast probing** (5ms) to detect firmware responsiveness
/// 2. **Application-level timeouts** that don't rely on HID layer
/// 3. **Pattern detection** for consecutive failures
/// 4. **Fast failure** when firmware becomes unresponsive
///
/// This prevents applications from hanging when hardware issues occur.
pub mod timeouts {
    /// Ultra-fast probing to detect firmware responsiveness
    pub const PROBE: i32 = 3;
    /// Fast operations like device scanning - with stuck bus protection
    pub const SCAN: i32 = 8;
    /// Standard read operations - balance of speed and reliability
    pub const READ: i32 = 100;
    /// Standard write operations - slightly longer for device processing
    pub const WRITE: i32 = 200;
    /// Write-then-read operations - combined operation needs more time
    pub const WRITE_READ: i32 = 250;
    /// Maximum time to wait for any single operation before declaring firmware stuck
    pub const FIRMWARE_RESPONSIVENESS: i32 = 100;
    /// EEPROM write operations - can take several seconds for page writes
    pub const EEPROM_WRITE: i32 = 5000;
}

/// Represents a 7-bit or 10-bit I2C slave address.
/// Use `I2cAddress::new_7bit(addr)` or `I2cAddress::new_10bit(addr)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum I2cAddress {
    /// Standard 7-bit address (0x00 - 0x7F).
    Bit7(u8),
    /// Extended 10-bit address (0x0000 - 0x03FF).
    Bit10(u16),
}

impl I2cAddress {
    /// Creates a 7-bit address, checking validity (0-127).
    pub fn new_7bit(addr: u8) -> Result<Self> {
        if addr <= 0x7F {
            Ok(I2cAddress::Bit7(addr))
        } else {
            Err(Error::ArgumentOutOfRange(
                "7-bit I2C address must be 0-127".to_string(),
            ))
        }
    }

    /// Creates a 10-bit address, checking validity (0-1023).
    pub fn new_10bit(addr: u16) -> Result<Self> {
        if addr <= 0x03FF {
            Ok(I2cAddress::Bit10(addr))
        } else {
            Err(Error::InvalidI2c10BitAddress(addr))
        }
    }
}

impl fmt::Display for I2cAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            I2cAddress::Bit7(a) => write!(f, "7-bit 0x{:02X}", a),
            I2cAddress::Bit10(a) => write!(f, "10-bit 0x{:03X}", a),
        }
    }
}

impl Xr2280x {
    // --- I2C Methods ---
    //
    // The I2C implementation provides three categories of methods:
    //
    // 1. **Default timeout methods** (e.g., `i2c_read_7bit`):
    //    - Use operation-specific default timeouts
    //    - Suitable for most applications
    //    - Balance performance with reliability
    //
    // 2. **Custom timeout methods** (e.g., `i2c_read_7bit_with_timeout`):
    //    - Allow precise timeout control
    //    - Essential for real-time or special applications
    //    - Enable optimization for specific device characteristics
    //
    // 3. **Specialized methods** (e.g., `i2c_eeprom_write_7bit`):
    //    - Pre-configured for specific device types
    //    - Use appropriate timeouts for the target device class
    //    - Simplify common use cases
    //
    // All methods include automatic error handling for common I2C conditions:
    // - NACK responses (device not present/busy)
    // - Arbitration loss (multi-master conflicts)
    // - Timeouts (stuck bus or slow devices)
    // - Protocol errors (malformed responses)

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
    ///
    /// Uses a [`timeouts::WRITE`] (200ms) timeout, suitable for most device register writes.
    /// For EEPROM operations, consider using [`Self::i2c_eeprom_write_7bit`] instead.
    ///
    /// # Arguments
    /// * `slave_addr` - 7-bit I2C address (0x00-0x7F)
    /// * `data` - Data bytes to write (max 32 bytes)
    ///
    /// # Example
    /// ```no_run
    /// # use xr2280x_hid::*;
    /// # use hidapi::HidApi;
    /// # fn main() -> Result<()> {
    /// # let hid_api = HidApi::new()?;
    /// # let device = Xr2280x::device_open_first(&hid_api)?;
    /// // Write configuration to a sensor
    /// device.i2c_write_7bit(0x48, &[0x01, 0x60])?; // Set config register
    /// # Ok(())
    /// # }
    /// ```
    pub fn i2c_write_7bit(&self, slave_addr: u8, data: &[u8]) -> Result<()> {
        let addr = I2cAddress::new_7bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(data),
            None,
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeouts::WRITE),
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
            Some(timeouts::WRITE),
        )
    }

    /// Performs a 7-bit I2C read operation with default timeout.
    ///
    /// Uses a [`timeouts::READ`] (100ms) timeout, optimized for sensor readings and register access.
    ///
    /// # Arguments
    /// * `slave_addr` - 7-bit I2C address (0x00-0x7F)
    /// * `buffer` - Buffer to receive data (max 32 bytes)
    ///
    /// # Example
    /// ```no_run
    /// # use xr2280x_hid::*;
    /// # use hidapi::HidApi;
    /// # fn main() -> Result<()> {
    /// # let hid_api = HidApi::new()?;
    /// # let device = Xr2280x::device_open_first(&hid_api)?;
    /// // Read temperature from sensor
    /// let mut temp_data = [0u8; 2];
    /// device.i2c_read_7bit(0x48, &mut temp_data)?;
    /// let temperature = i16::from_be_bytes(temp_data) as f32 / 256.0;
    /// # Ok(())
    /// # }
    /// ```
    pub fn i2c_read_7bit(&self, slave_addr: u8, buffer: &mut [u8]) -> Result<()> {
        let addr = I2cAddress::new_7bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            None,
            Some(buffer),
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeouts::READ),
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
            Some(timeouts::READ),
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
            Some(timeouts::WRITE_READ),
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
            Some(timeouts::WRITE_READ),
        )
    }

    /// Performs a 7-bit I2C write operation with custom timeout.
    /// Use this for slow devices (like EEPROMs) that need longer timeouts.
    pub fn i2c_write_7bit_with_timeout(
        &self,
        slave_addr: u8,
        data: &[u8],
        timeout_ms: i32,
    ) -> Result<()> {
        let addr = I2cAddress::new_7bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(data),
            None,
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeout_ms),
        )
    }

    /// Performs a 10-bit I2C write operation with custom timeout.
    /// Use this for slow devices (like EEPROMs) that need longer timeouts.
    pub fn i2c_write_10bit_with_timeout(
        &self,
        slave_addr: u16,
        data: &[u8],
        timeout_ms: i32,
    ) -> Result<()> {
        let addr = I2cAddress::new_10bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(data),
            None,
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeout_ms),
        )
    }

    /// Performs a 7-bit I2C read operation with custom timeout.
    /// Use this for slow devices or when you need faster response times.
    pub fn i2c_read_7bit_with_timeout(
        &self,
        slave_addr: u8,
        buffer: &mut [u8],
        timeout_ms: i32,
    ) -> Result<()> {
        let addr = I2cAddress::new_7bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            None,
            Some(buffer),
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeout_ms),
        )
    }

    /// Performs a 10-bit I2C read operation with custom timeout.
    /// Use this for slow devices or when you need faster response times.
    pub fn i2c_read_10bit_with_timeout(
        &self,
        slave_addr: u16,
        buffer: &mut [u8],
        timeout_ms: i32,
    ) -> Result<()> {
        let addr = I2cAddress::new_10bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            None,
            Some(buffer),
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeout_ms),
        )
    }

    /// Performs a 7-bit I2C write-then-read operation with custom timeout.
    /// Use this for slow devices or when you need faster response times.
    pub fn i2c_write_read_7bit_with_timeout(
        &self,
        slave_addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
        timeout_ms: i32,
    ) -> Result<()> {
        let addr = I2cAddress::new_7bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(write_data),
            Some(read_buffer),
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeout_ms),
        )
    }

    /// Performs a 10-bit I2C write-then-read operation with custom timeout.
    /// Use this for slow devices or when you need faster response times.
    pub fn i2c_write_read_10bit_with_timeout(
        &self,
        slave_addr: u16,
        write_data: &[u8],
        read_buffer: &mut [u8],
        timeout_ms: i32,
    ) -> Result<()> {
        let addr = I2cAddress::new_10bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(write_data),
            Some(read_buffer),
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeout_ms),
        )
    }

    /// Performs a 7-bit I2C EEPROM write operation with extended timeout.
    /// EEPROMs can take several seconds to complete internal write cycles.
    /// This method uses a 5-second default timeout suitable for most EEPROMs.
    pub fn i2c_eeprom_write_7bit(&self, slave_addr: u8, data: &[u8]) -> Result<()> {
        let addr = I2cAddress::new_7bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(data),
            None,
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeouts::EEPROM_WRITE),
        )
    }

    /// Performs a 10-bit I2C EEPROM write operation with extended timeout.
    /// EEPROMs can take several seconds to complete internal write cycles.
    /// This method uses a 5-second default timeout suitable for most EEPROMs.
    pub fn i2c_eeprom_write_10bit(&self, slave_addr: u16, data: &[u8]) -> Result<()> {
        let addr = I2cAddress::new_10bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(data),
            None,
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeouts::EEPROM_WRITE),
        )
    }

    /// Performs a 7-bit I2C EEPROM write operation with custom timeout.
    /// Use this when you know the specific timing requirements of your EEPROM.
    pub fn i2c_eeprom_write_7bit_with_timeout(
        &self,
        slave_addr: u8,
        data: &[u8],
        timeout_ms: i32,
    ) -> Result<()> {
        let addr = I2cAddress::new_7bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(data),
            None,
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeout_ms),
        )
    }

    /// Performs a 10-bit I2C EEPROM write operation with custom timeout.
    /// Use this when you know the specific timing requirements of your EEPROM.
    pub fn i2c_eeprom_write_10bit_with_timeout(
        &self,
        slave_addr: u16,
        data: &[u8],
        timeout_ms: i32,
    ) -> Result<()> {
        let addr = I2cAddress::new_10bit(slave_addr)?;
        self.i2c_transfer_raw(
            addr,
            Some(data),
            None,
            flags::i2c::START_BIT | flags::i2c::STOP_BIT,
            Some(timeout_ms),
        )
    }

    /// Low-level I2C transfer with full control over flags and timeout.
    /// Can perform writes, reads, or write-then-read operations.
    /// See [`crate::flags::i2c`] for available flag constants.
    pub fn i2c_transfer_raw(
        &self,
        slave_addr: I2cAddress,
        write_data: Option<&[u8]>,
        read_buffer: Option<&mut [u8]>,
        flags: u8,
        timeout_ms: Option<i32>,
    ) -> Result<()> {
        self.i2c_transfer(
            slave_addr,
            write_data.unwrap_or(&[]),
            read_buffer,
            flags,
            timeout_ms,
        )
    }

    /// Fast I2C bus scan for device discovery.
    /// Scans the specified range of 7-bit addresses using optimized timeouts.
    /// Returns a vector of addresses where devices responded with ACK.
    ///
    /// # Arguments
    /// * `start_addr` - First 7-bit address to scan (typically 0x08)
    /// * `end_addr` - Last 7-bit address to scan (typically 0x77)
    ///
    /// # Example
    /// ```no_run
    /// # use xr2280x_hid::*;
    /// # use hidapi::HidApi;
    /// # fn main() -> Result<()> {
    /// # let hid_api = HidApi::new()?;
    /// # let device = Xr2280x::device_open_first(&hid_api)?;
    /// let found_devices = device.i2c_scan(0x08, 0x77)?;
    /// for addr in found_devices {
    ///     println!("Found device at 0x{:02X}", addr);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn i2c_scan(&self, start_addr: u8, end_addr: u8) -> Result<Vec<u8>> {
        self.i2c_scan_with_progress(start_addr, end_addr, |_, _, _, _| {})
    }

    /// Fast I2C bus scan using the standard address range (0x08 to 0x77).
    /// This is a convenience method that scans the most commonly used I2C address space,
    /// avoiding reserved addresses at the low and high ends.
    ///
    /// # Example
    /// ```no_run
    /// # use xr2280x_hid::*;
    /// # use hidapi::HidApi;
    /// # fn main() -> Result<()> {
    /// # let hid_api = HidApi::new()?;
    /// # let device = Xr2280x::device_open_first(&hid_api)?;
    /// let found_devices = device.i2c_scan_default()?;
    /// for addr in found_devices {
    ///     println!("Found device at 0x{:02X}", addr);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn i2c_scan_default(&self) -> Result<Vec<u8>> {
        self.i2c_scan(0x08, 0x77)
    }

    /// Fast I2C bus scan with progress callback for device discovery.
    ///
    /// Scans the specified range of 7-bit addresses using optimized [`timeouts::SCAN`] (25ms) timeouts.
    /// Includes automatic stuck bus detection to prevent hanging when hardware issues occur.
    ///
    /// # Stuck Bus Detection
    ///
    /// The scan includes built-in protection against stuck I2C buses:
    /// - Pre-scan test using reserved address 0x00
    /// - Monitoring for consecutive timeout patterns
    /// - Extended timeout verification when stuck bus suspected
    /// - Early termination with clear error message
    ///
    /// This prevents indefinite hanging when devices drive bus lines low (e.g., unpowered devices).
    ///
    /// # Arguments
    /// * `start_addr` - First 7-bit address to scan (typically 0x08)
    /// * `end_addr` - Last 7-bit address to scan (typically 0x77)
    /// * `progress_callback` - Called for each address: (addr, found, current_idx, total)
    ///
    /// # Example
    /// ```no_run
    /// # use xr2280x_hid::*;
    /// # use hidapi::HidApi;
    /// # fn main() -> Result<()> {
    /// # let hid_api = HidApi::new()?;
    /// # let device = Xr2280x::device_open_first(&hid_api)?;
    /// let found_devices = device.i2c_scan_with_progress(0x08, 0x77, |addr, found, idx, total| {
    ///     if found {
    ///         println!("Device found at 0x{:02X}", addr);
    ///     }
    ///     if idx % 16 == 0 {
    ///         println!("Progress: {}/{}", idx, total);
    ///     }
    /// })?;
    ///
    /// // Handle potential stuck bus error
    /// match device.i2c_scan_default() {
    ///     Ok(devices) => println!("Found {} devices", devices.len()),
    ///     Err(Error::I2cTimeout { address }) => {
    ///         eprintln!("Stuck I2C bus detected at {}", address);
    ///         eprintln!("Check hardware connections and device power");
    ///     }
    ///     Err(e) => eprintln!("Scan error: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn i2c_scan_with_progress<F>(
        &self,
        start_addr: u8,
        end_addr: u8,
        progress_callback: F,
    ) -> Result<Vec<u8>>
    where
        F: FnMut(u8, bool, usize, usize),
    {
        self.i2c_scan_with_progress_and_timeout(
            start_addr,
            end_addr,
            timeouts::SCAN,
            progress_callback,
        )
    }

    /// Fast I2C bus scan with custom timeout and progress callback.
    /// Allows fine-tuning of scan timeout for different scenarios.
    /// Includes robust stuck bus detection that prevents 29+ second hangs.
    ///
    /// # Arguments
    /// * `start_addr` - First 7-bit address to scan (typically 0x08)
    /// * `end_addr` - Last 7-bit address to scan (typically 0x77)
    /// * `scan_timeout_ms` - Timeout per address in milliseconds (e.g., 5ms for fast scan, 100ms for slow devices)
    /// * `progress_callback` - Called for each address: (addr, found, current_idx, total)
    pub fn i2c_scan_with_progress_and_timeout<F>(
        &self,
        start_addr: u8,
        end_addr: u8,
        scan_timeout_ms: i32,
        mut progress_callback: F,
    ) -> Result<Vec<u8>>
    where
        F: FnMut(u8, bool, usize, usize),
    {
        // Step 1: Quick firmware responsiveness test to catch stuck bus immediately
        debug!("Testing firmware responsiveness with ultra-short timeout...");
        let probe_start = Instant::now();

        match self.test_firmware_responsiveness() {
            Ok(_) => {
                debug!("Firmware responsive");
            }
            Err(e) => {
                warn!("Firmware stuck - aborting scan immediately");
                return Err(e);
            }
        }

        debug!("Responsiveness test passed in {:?}", probe_start.elapsed());

        // Step 2: Perform actual scan with fast failure detection
        let mut found_devices = Vec::new();
        let flags = flags::i2c::START_BIT | flags::i2c::STOP_BIT;
        let total_addresses = (end_addr - start_addr + 1) as usize;
        let mut consecutive_timeouts = 0;
        const MAX_CONSECUTIVE_TIMEOUTS: usize = 1; // Fail immediately on stuck bus

        let scan_start = Instant::now();

        for (idx, addr_7bit) in (start_addr..=end_addr).enumerate() {
            let address = I2cAddress::new_7bit(addr_7bit)?;
            let mut found = false;

            // Use the specified timeout, but fail fast on consecutive timeouts
            match self.i2c_transfer_raw(address, None, None, flags, Some(scan_timeout_ms)) {
                Ok(_) => {
                    found_devices.push(addr_7bit);
                    found = true;
                    consecutive_timeouts = 0;
                }
                Err(Error::I2cNack { .. }) => {
                    // Normal - no device at this address
                    consecutive_timeouts = 0;
                }
                Err(Error::I2cTimeout { .. }) => {
                    consecutive_timeouts += 1;
                    if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                        warn!(
                            "Multiple consecutive timeouts starting at 0x{:02X} - bus likely stuck",
                            addr_7bit - consecutive_timeouts as u8 + 1
                        );
                        return Err(Error::I2cTimeout { address });
                    }
                }
                Err(Error::I2cArbitrationLost { address }) => {
                    warn!(
                        "I2C arbitration lost at address 0x{:02X} - this indicates bus contention",
                        addr_7bit
                    );
                    warn!(
                        "Possible causes: multiple I2C masters, electrical interference, or loose connections"
                    );
                    warn!("Recommendation: Check wiring, disconnect other I2C devices, and retry");
                    return Err(Error::I2cArbitrationLost { address });
                }
                Err(e) => {
                    debug!("Error scanning address 0x{:02X}: {}", addr_7bit, e);
                    // Don't count other errors as timeouts, but still fail fast if too many
                    consecutive_timeouts += 1;
                    if consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS {
                        return Err(e);
                    }
                }
            }

            // Call progress callback
            progress_callback(addr_7bit, found, idx, total_addresses);
        }

        debug!(
            "Scan completed in {:?}, found {} devices",
            scan_start.elapsed(),
            found_devices.len()
        );
        Ok(found_devices)
    }

    /// Tests if the XR2280x firmware is responsive by attempting a quick I2C operation.
    /// This catches firmware hangs before they can cause 29+ second delays.
    /// Uses an ultra-short timeout to fail fast if firmware is stuck.
    fn test_firmware_responsiveness(&self) -> Result<()> {
        let test_address = I2cAddress::new_7bit(0x00)?; // Reserved address
        let flags = flags::i2c::START_BIT | flags::i2c::STOP_BIT;

        debug!("Testing firmware responsiveness with 3ms timeout on reserved address");

        // Use ultra-short timeout - if firmware is going to hang, it hangs immediately
        match self.i2c_transfer_raw(test_address, None, None, flags, Some(timeouts::PROBE)) {
            Ok(_) => {
                // Reserved address shouldn't respond, but firmware is working
                debug!("Reserved address responded - unusual but firmware is responsive");
                Ok(())
            }
            Err(Error::I2cNack { .. }) => {
                // Expected - firmware is responsive and bus is working
                debug!("Firmware responsiveness test passed");
                Ok(())
            }
            Err(Error::I2cTimeout { .. }) => {
                // This indicates firmware or bus is stuck - fail immediately
                warn!("Firmware failed to respond within 3ms - bus likely stuck");
                Err(Error::I2cTimeout {
                    address: test_address,
                })
            }
            Err(e) => {
                // Other error types still indicate firmware is responsive
                debug!("Firmware responsive, other error: {}", e);
                Ok(())
            }
        }
    }

    // Internal I2C transfer implementation
    fn i2c_transfer(
        &self,
        slave_addr: I2cAddress,
        write_data: &[u8],
        read_buffer: Option<&mut [u8]>,
        flags: u8,
        timeout_ms: Option<i32>,
    ) -> Result<()> {
        let timeout = timeout_ms.unwrap_or(timeouts::READ);
        let write_len = write_data.len();
        let read_len = read_buffer.as_ref().map(|b| b.len()).unwrap_or(0);

        // Validate sizes
        if write_len > consts::i2c::REPORT_MAX_DATA_SIZE {
            return Err(Error::OperationTooLarge {
                max: consts::i2c::REPORT_MAX_DATA_SIZE,
                actual: write_len,
            });
        }
        if read_len > consts::i2c::REPORT_MAX_DATA_SIZE {
            return Err(Error::OperationTooLarge {
                max: consts::i2c::REPORT_MAX_DATA_SIZE,
                actual: read_len,
            });
        }

        // Prepare OUT report buffer (no Report ID byte needed for write())
        let mut out_buf = vec![0u8; consts::i2c::OUT_REPORT_WRITE_BUF_SIZE];
        out_buf[0] = flags;
        out_buf[1] = write_len as u8;
        out_buf[2] = read_len as u8;

        // Set slave address based on type
        match slave_addr {
            // For 7-bit addresses, shift left by 1 to create the 8-bit wire format
            // The I2C protocol requires the 7-bit address in bits 7:1, with bit 0 reserved for R/W
            I2cAddress::Bit7(addr) => out_buf[3] = addr << 1,
            I2cAddress::Bit10(addr) => {
                // For 10-bit, use special encoding per datasheet
                // High byte in [3], low byte in first data position [4]
                out_buf[3] = ((addr >> 8) & 0x03) as u8 | 0xF0; // 11110xx0 pattern
                if write_len > 0 {
                    // If writing data, shift it and insert low addr byte
                    out_buf[5..5 + write_len].copy_from_slice(write_data);
                    out_buf[4] = (addr & 0xFF) as u8;
                    out_buf[1] = (write_len + 1) as u8; // Increase write size
                } else {
                    // Read-only, low byte goes in data[0]
                    out_buf[4] = (addr & 0xFF) as u8;
                    out_buf[1] = 1; // Write size = 1 for address
                }
            }
        }

        // Copy write data for 7-bit or adjusted for 10-bit
        if matches!(slave_addr, I2cAddress::Bit7(_)) && write_len > 0 {
            out_buf[4..4 + write_len].copy_from_slice(write_data);
        }

        debug!(
            "I2C transfer to {}: write {} bytes, read {} bytes, flags=0x{:02X}",
            slave_addr, write_len, read_len, flags
        );
        trace!("I2C OUT buffer: {:02X?}", &out_buf);

        // Send the OUT report
        let i2c_device = self.i2c_device.as_ref().ok_or(Error::DeviceNotFound)?;
        let written = i2c_device.write(&out_buf).map_err(Error::Hid)?;

        if written != out_buf.len() {
            warn!("Partial write: sent {} of {} bytes", written, out_buf.len());
            return Err(Error::Io(std::io::Error::other("Partial HID write")));
        }
        trace!("Sent {} bytes to device", written);

        // Always read the status response from device (even for write-only operations)
        let mut in_buf = vec![0u8; consts::i2c::IN_REPORT_READ_BUF_SIZE];
        let received = i2c_device
            .read_timeout(&mut in_buf, timeout)
            .map_err(Error::Hid)?;

        trace!(
            "Received {} bytes from device: {:02X?}",
            received,
            &in_buf[..received]
        );

        if received < 4 {
            return Err(Error::InvalidReport(received));
        }

        // Check status flags
        let status_flags = in_buf[0];
        if status_flags & consts::i2c::in_flags::REQUEST_ERROR != 0 {
            return Err(Error::I2cRequestError {
                address: slave_addr,
            });
        }
        if status_flags & consts::i2c::in_flags::NAK_RECEIVED != 0 {
            return Err(Error::I2cNack {
                address: slave_addr,
            });
        }
        if status_flags & consts::i2c::in_flags::ARBITRATION_LOST != 0 {
            return Err(Error::I2cArbitrationLost {
                address: slave_addr,
            });
        }
        if status_flags & consts::i2c::in_flags::TIMEOUT != 0 {
            return Err(Error::I2cTimeout {
                address: slave_addr,
            });
        }
        if status_flags & 0x0F != 0 {
            // Any other error bits set
            return Err(Error::I2cUnknownError {
                address: slave_addr,
                flags: status_flags,
            });
        }

        // Extract read data only if reading was requested
        if read_len > 0 {
            let reported_read_len = in_buf[2] as usize;
            if reported_read_len != read_len {
                warn!(
                    "I2C read length mismatch: expected {}, got {}",
                    read_len, reported_read_len
                );
            }
            let actual_read_len = reported_read_len
                .min(read_len)
                .min(received.saturating_sub(4));

            if let Some(read_buf) = read_buffer {
                read_buf[..actual_read_len].copy_from_slice(&in_buf[4..4 + actual_read_len]);
            }
        }

        Ok(())
    }
}
