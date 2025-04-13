use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioDirection {
    Input,
    Output,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioLevel {
    Low,
    High,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioPull {
    None,
    Up,
    Down,
}

/// Represents a valid GPIO Pin number (0-31).
/// Use `GpioPin::new(num)` to create.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpioPin(pub(crate) u8); // Make field private to enforce constructor use

impl GpioPin {
    /// Creates a new GpioPin, returning an error if the number is out of range (0-31).
    pub fn new(pin_num: u8) -> Result<Self> {
        if pin_num <= 31 {
            Ok(GpioPin(pin_num))
        } else {
            Err(Error::PinArgumentOutOfRange {
                pin: pin_num,
                message: "Pin number must be 0-31".to_string(),
            })
        }
    }

    /// Returns the underlying pin number (0-31).
    #[inline]
    pub fn number(&self) -> u8 {
        self.0
    }

    /// Returns the group index (0 or 1) the pin belongs to.
    #[inline]
    pub fn group_index(&self) -> u8 {
        self.0 / 16
    }

    /// Returns the bit index (0-15) within the group's register.
    #[inline]
    pub fn bit_index(&self) -> u8 {
        self.0 % 16
    }

    /// Returns the bit mask (1 << bit_index) for register operations.
    #[inline]
    pub fn mask(&self) -> u16 {
        1u16 << self.bit_index()
    }
}

// Removed unused helper: get_gpio_reg_and_mask
