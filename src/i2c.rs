//! I2C communication functionality for XR2280x devices.

use crate::consts;
use crate::device::Xr2280x;
use crate::error::{Error, Result};
use crate::flags;
use log::{debug, trace, warn};
use std::fmt;

/// Default I2C timeout in milliseconds.
const DEFAULT_I2C_TIMEOUT_MS: i32 = 500;

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

    // Internal I2C transfer implementation
    fn i2c_transfer(
        &self,
        slave_addr: I2cAddress,
        write_data: &[u8],
        mut read_buffer: Option<&mut [u8]>,
        flags: u8,
        timeout_ms: Option<i32>,
    ) -> Result<()> {
        let timeout = timeout_ms.unwrap_or(DEFAULT_I2C_TIMEOUT_MS);
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
            I2cAddress::Bit7(addr) => out_buf[3] = addr,
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
        if let I2cAddress::Bit7(_) = slave_addr {
            if write_len > 0 {
                out_buf[4..4 + write_len].copy_from_slice(write_data);
            }
        }

        debug!(
            "I2C transfer to {}: write {} bytes, read {} bytes, flags=0x{:02X}",
            slave_addr, write_len, read_len, flags
        );
        trace!("I2C OUT buffer: {:02X?}", &out_buf);

        // Send the OUT report
        match self.device.write(&out_buf) {
            Ok(written) if written == out_buf.len() => {
                trace!("Sent {} bytes to device", written);
            }
            Ok(written) => {
                warn!(
                    "Partial write: sent {} of {} bytes",
                    written,
                    out_buf.len()
                );
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Partial HID write",
                )));
            }
            Err(e) => return Err(Error::Hid(e)),
        }

        // If reading, get the IN report
        if read_len > 0 {
            let mut in_buf = vec![0u8; consts::i2c::IN_REPORT_READ_BUF_SIZE];
            match self.device.read_timeout(&mut in_buf, timeout) {
                Ok(received) => {
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

                    // Extract read data
                    let reported_read_len = in_buf[2] as usize;
                    if reported_read_len != read_len {
                        warn!(
                            "I2C read length mismatch: expected {}, got {}",
                            read_len, reported_read_len
                        );
                    }
                    let actual_read_len = reported_read_len.min(read_len).min(received.saturating_sub(4));

                    if let Some(ref mut read_buf) = read_buffer {
                        read_buf[..actual_read_len].copy_from_slice(&in_buf[4..4 + actual_read_len]);
                    }
                }
                Err(e) => {
                    warn!("I2C read timeout or error: {}", e);
                    return Err(Error::Hid(e));
                }
            }
        }

        Ok(())
    }
}
