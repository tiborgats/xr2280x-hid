use crate::error::{Error, Result};
use std::fmt;

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
