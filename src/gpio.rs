//! GPIO (General Purpose Input/Output) functionality for XR2280x devices.

use crate::consts;
use crate::device::Xr2280x;
use crate::error::{unsupported_gpio_group1, Error, Result};
use log::{debug, trace};

/// Represents a GPIO group for bulk operations.
/// GPIO Group (0-15 or 16-31) for XR22802/4 multi-group support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioGroup {
    /// GPIO pins 0-15 (supported on all XR2280x models).
    Group0,
    /// GPIO pins 16-31 (only supported on XR22802/XR22804).
    Group1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Direction configuration for a GPIO pin.
pub enum GpioDirection {
    /// Configure pin as input (high impedance).
    Input,
    /// Configure pin as output (can drive high or low).
    Output,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Logic level for GPIO pin state.
pub enum GpioLevel {
    /// Logic low (0V, ground).
    Low,
    /// Logic high (3.3V, VCC).
    High,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Pull resistor configuration for GPIO pins.
pub enum GpioPull {
    /// No pull resistor (floating input).
    None,
    /// Pull-up resistor enabled (weakly pulls to VCC).
    Up,
    /// Pull-down resistor enabled (weakly pulls to ground).
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

impl Xr2280x {
    // --- GPIO Pin Operations ---
    /// Assigns a GPIO pin to the EDGE controller (required before using GPIO functions).
    pub fn gpio_assign_to_edge(&self, pin: GpioPin) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_FUNC_SEL_0
        } else {
            consts::edge::REG_FUNC_SEL_1
        };
        let current = self.read_hid_register(reg)?;
        let new_value = current | pin.mask();
        debug!("Assigning GPIO pin {} to EDGE controller", pin.number());
        self.write_hid_register(reg, new_value)?;
        Ok(())
    }

    /// Checks if a GPIO pin is assigned to the EDGE controller.
    pub fn gpio_is_assigned_to_edge(&self, pin: GpioPin) -> Result<bool> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_FUNC_SEL_0
        } else {
            consts::edge::REG_FUNC_SEL_1
        };
        let value = self.read_hid_register(reg)?;
        Ok((value & pin.mask()) != 0)
    }

    /// Sets the direction of a GPIO pin (Input or Output).
    pub fn gpio_set_direction(&self, pin: GpioPin, direction: GpioDirection) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_DIR_0
        } else {
            consts::edge::REG_DIR_1
        };
        let current = self.read_hid_register(reg)?;
        let new_value = match direction {
            GpioDirection::Input => current & !pin.mask(), // 0 = Input
            GpioDirection::Output => current | pin.mask(), // 1 = Output
        };
        debug!(
            "Setting GPIO pin {} direction to {:?}",
            pin.number(),
            direction
        );
        self.write_hid_register(reg, new_value)?;
        Ok(())
    }

    /// Gets the direction of a GPIO pin (Input or Output).
    pub fn gpio_get_direction(&self, pin: GpioPin) -> Result<GpioDirection> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_DIR_0
        } else {
            consts::edge::REG_DIR_1
        };
        let value = self.read_hid_register(reg)?;
        Ok(if (value & pin.mask()) != 0 {
            GpioDirection::Output
        } else {
            GpioDirection::Input
        })
    }

    /// Writes a level to a GPIO pin configured as output.
    pub fn gpio_write(&self, pin: GpioPin, level: GpioLevel) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let (reg_set, reg_clear) = if pin.group_index() == 0 {
            (consts::edge::REG_SET_0, consts::edge::REG_CLEAR_0)
        } else {
            (consts::edge::REG_SET_1, consts::edge::REG_CLEAR_1)
        };
        debug!(
            "Writing {:?} to GPIO pin {} (mask=0x{:04X})",
            level,
            pin.number(),
            pin.mask()
        );
        match level {
            GpioLevel::High => self.write_hid_register(reg_set, pin.mask())?,
            GpioLevel::Low => self.write_hid_register(reg_clear, pin.mask())?,
        }
        Ok(())
    }

    /// Reads the current level of a GPIO pin.
    pub fn gpio_read(&self, pin: GpioPin) -> Result<GpioLevel> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_STATE_0
        } else {
            consts::edge::REG_STATE_1
        };
        let value = self.read_hid_register(reg)?;
        let level = if (value & pin.mask()) != 0 {
            GpioLevel::High
        } else {
            GpioLevel::Low
        };
        trace!("Read {:?} from GPIO pin {}", level, pin.number());
        Ok(level)
    }

    /// Sets the pull resistor configuration for a GPIO pin.
    pub fn gpio_set_pull(&self, pin: GpioPin, pull: GpioPull) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let (reg_up, reg_down) = if pin.group_index() == 0 {
            (consts::edge::REG_PULL_UP_0, consts::edge::REG_PULL_DOWN_0)
        } else {
            (consts::edge::REG_PULL_UP_1, consts::edge::REG_PULL_DOWN_1)
        };

        debug!("Setting GPIO pin {} pull to {:?}", pin.number(), pull);

        match pull {
            GpioPull::None => {
                // Clear both pull-up and pull-down
                let up_val = self.read_hid_register(reg_up)?;
                self.write_hid_register(reg_up, up_val & !pin.mask())?;
                let down_val = self.read_hid_register(reg_down)?;
                self.write_hid_register(reg_down, down_val & !pin.mask())?;
            }
            GpioPull::Up => {
                // Set pull-up, clear pull-down
                let up_val = self.read_hid_register(reg_up)?;
                self.write_hid_register(reg_up, up_val | pin.mask())?;
                let down_val = self.read_hid_register(reg_down)?;
                self.write_hid_register(reg_down, down_val & !pin.mask())?;
            }
            GpioPull::Down => {
                // Clear pull-up, set pull-down
                let up_val = self.read_hid_register(reg_up)?;
                self.write_hid_register(reg_up, up_val & !pin.mask())?;
                let down_val = self.read_hid_register(reg_down)?;
                self.write_hid_register(reg_down, down_val | pin.mask())?;
            }
        }
        Ok(())
    }

    /// Gets the pull resistor configuration for a GPIO pin.
    pub fn gpio_get_pull(&self, pin: GpioPin) -> Result<GpioPull> {
        self.check_gpio_pin_support(pin)?;
        let (reg_up, reg_down) = if pin.group_index() == 0 {
            (consts::edge::REG_PULL_UP_0, consts::edge::REG_PULL_DOWN_0)
        } else {
            (consts::edge::REG_PULL_UP_1, consts::edge::REG_PULL_DOWN_1)
        };

        let up_val = self.read_hid_register(reg_up)?;
        let down_val = self.read_hid_register(reg_down)?;

        let has_pull_up = (up_val & pin.mask()) != 0;
        let has_pull_down = (down_val & pin.mask()) != 0;

        Ok(match (has_pull_up, has_pull_down) {
            (true, false) => GpioPull::Up,
            (false, true) => GpioPull::Down,
            _ => GpioPull::None, // Both or neither
        })
    }

    /// Sets the open-drain configuration for a GPIO pin.
    pub fn gpio_set_open_drain(&self, pin: GpioPin, enable: bool) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_OPEN_DRAIN_0
        } else {
            consts::edge::REG_OPEN_DRAIN_1
        };
        let current = self.read_hid_register(reg)?;
        let new_value = if enable {
            current | pin.mask()
        } else {
            current & !pin.mask()
        };
        debug!("Setting GPIO pin {} open-drain to {}", pin.number(), enable);
        self.write_hid_register(reg, new_value)?;
        Ok(())
    }

    /// Checks if a GPIO pin is configured for open-drain output.
    pub fn gpio_is_open_drain(&self, pin: GpioPin) -> Result<bool> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_OPEN_DRAIN_0
        } else {
            consts::edge::REG_OPEN_DRAIN_1
        };
        let value = self.read_hid_register(reg)?;
        Ok((value & pin.mask()) != 0)
    }

    /// Sets the tri-state (high-impedance) configuration for a GPIO pin.
    pub fn gpio_set_tri_state(&self, pin: GpioPin, enable: bool) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_TRI_STATE_0
        } else {
            consts::edge::REG_TRI_STATE_1
        };
        let current = self.read_hid_register(reg)?;
        let new_value = if enable {
            current | pin.mask()
        } else {
            current & !pin.mask()
        };
        debug!("Setting GPIO pin {} tri-state to {}", pin.number(), enable);
        self.write_hid_register(reg, new_value)?;
        Ok(())
    }

    /// Checks if a GPIO pin is in tri-state (high-impedance) mode.
    pub fn gpio_is_tri_stated(&self, pin: GpioPin) -> Result<bool> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_TRI_STATE_0
        } else {
            consts::edge::REG_TRI_STATE_1
        };
        let value = self.read_hid_register(reg)?;
        Ok((value & pin.mask()) != 0)
    }

    // --- GPIO Group Operations (Bulk) ---
    /// Sets the direction of multiple GPIO pins in a group using a mask.
    /// Bit positions in the mask correspond to pins 0-15 within the group.
    pub fn gpio_set_direction_masked(
        &self,
        group: GpioGroup,
        mask: u16,
        direction: GpioDirection,
    ) -> Result<()> {
        self.check_gpio_group_support(group)?;
        let reg_dir = self.get_gpio_reg_for_group(group, consts::edge::REG_DIR_0);

        let current = self.read_hid_register(reg_dir)?;
        let new_value = match direction {
            GpioDirection::Input => current & !mask, // 0 = Input
            GpioDirection::Output => current | mask, // 1 = Output
        };
        debug!(
            "Setting {:?} pins (mask=0x{:04X}) direction to {:?}",
            group, mask, direction
        );
        self.write_hid_register(reg_dir, new_value)?;
        Ok(())
    }

    /// Writes levels to multiple GPIO pins in a group.
    /// The `mask` determines which pins are affected (1 = write, 0 = ignore).
    /// The `values` determine the levels to write (1 = High, 0 = Low).
    pub fn gpio_write_masked(&self, group: GpioGroup, mask: u16, values: u16) -> Result<()> {
        self.check_gpio_group_support(group)?;
        let (reg_set, reg_clear) = self.get_gpio_group_regs(group);

        // Which pins to set high
        let set_mask = mask & values;
        // Which pins to set low
        let clear_mask = mask & !values;

        debug!(
            "Writing to {:?}: set_mask=0x{:04X}, clear_mask=0x{:04X}",
            group, set_mask, clear_mask
        );

        if set_mask != 0 {
            self.write_hid_register(reg_set, set_mask)?;
        }
        if clear_mask != 0 {
            self.write_hid_register(reg_clear, clear_mask)?;
        }
        Ok(())
    }

    /// Reads the current levels of all GPIO pins in a group.
    /// Returns a 16-bit value where each bit represents a pin's state (1 = High, 0 = Low).
    pub fn gpio_read_group(&self, group: GpioGroup) -> Result<u16> {
        self.check_gpio_group_support(group)?;
        let reg_state = self.get_gpio_reg_for_group(group, consts::edge::REG_STATE_0);
        let value = self.read_hid_register(reg_state)?;
        trace!("Read {:?} state: 0x{:04X}", group, value);
        Ok(value)
    }

    /// Sets the pull resistor configuration for multiple GPIO pins in a group.
    pub fn gpio_set_pull_masked(&self, group: GpioGroup, mask: u16, pull: GpioPull) -> Result<()> {
        self.check_gpio_group_support(group)?;
        let reg_up = self.get_gpio_reg_for_group(group, consts::edge::REG_PULL_UP_0);
        let reg_down = self.get_gpio_reg_for_group(group, consts::edge::REG_PULL_DOWN_0);

        debug!(
            "Setting {:?} pins (mask=0x{:04X}) pull to {:?}",
            group, mask, pull
        );

        match pull {
            GpioPull::None => {
                // Clear both pull-up and pull-down for masked pins
                let up_val = self.read_hid_register(reg_up)?;
                self.write_hid_register(reg_up, up_val & !mask)?;
                let down_val = self.read_hid_register(reg_down)?;
                self.write_hid_register(reg_down, down_val & !mask)?;
            }
            GpioPull::Up => {
                // Set pull-up, clear pull-down for masked pins
                let up_val = self.read_hid_register(reg_up)?;
                self.write_hid_register(reg_up, up_val | mask)?;
                let down_val = self.read_hid_register(reg_down)?;
                self.write_hid_register(reg_down, down_val & !mask)?;
            }
            GpioPull::Down => {
                // Clear pull-up, set pull-down for masked pins
                let up_val = self.read_hid_register(reg_up)?;
                self.write_hid_register(reg_up, up_val & !mask)?;
                let down_val = self.read_hid_register(reg_down)?;
                self.write_hid_register(reg_down, down_val | mask)?;
            }
        }
        Ok(())
    }

    /// Sets the open-drain configuration for multiple GPIO pins in a group.
    pub fn gpio_set_open_drain_masked(
        &self,
        group: GpioGroup,
        mask: u16,
        enable: bool,
    ) -> Result<()> {
        self.check_gpio_group_support(group)?;
        let reg_od = self.get_gpio_reg_for_group(group, consts::edge::REG_OPEN_DRAIN_0);

        let current = self.read_hid_register(reg_od)?;
        let new_value = if enable {
            current | mask
        } else {
            current & !mask
        };
        debug!(
            "Setting {:?} pins (mask=0x{:04X}) open-drain to {}",
            group, mask, enable
        );
        self.write_hid_register(reg_od, new_value)?;
        Ok(())
    }

    /// Sets the tri-state configuration for multiple GPIO pins in a group.
    pub fn gpio_set_tri_state_masked(
        &self,
        group: GpioGroup,
        mask: u16,
        enable: bool,
    ) -> Result<()> {
        self.check_gpio_group_support(group)?;
        let reg_ts = self.get_gpio_reg_for_group(group, consts::edge::REG_TRI_STATE_0);

        let current = self.read_hid_register(reg_ts)?;
        let new_value = if enable {
            current | mask
        } else {
            current & !mask
        };
        debug!(
            "Setting {:?} pins (mask=0x{:04X}) tri-state to {}",
            group, mask, enable
        );
        self.write_hid_register(reg_ts, new_value)?;
        Ok(())
    }

    // --- Helper Methods ---
    fn get_gpio_group_regs(&self, group: GpioGroup) -> (u16, u16) {
        match group {
            GpioGroup::Group0 => (consts::edge::REG_SET_0, consts::edge::REG_CLEAR_0),
            GpioGroup::Group1 => (consts::edge::REG_SET_1, consts::edge::REG_CLEAR_1),
        }
    }

    fn get_gpio_reg_for_group(&self, group: GpioGroup, base_reg: u16) -> u16 {
        match group {
            GpioGroup::Group0 => base_reg,
            GpioGroup::Group1 => {
                base_reg + (consts::edge::REG_FUNC_SEL_1 - consts::edge::REG_FUNC_SEL_0)
            }
        }
    }

    pub(crate) fn check_gpio_pin_support(&self, pin: GpioPin) -> Result<()> {
        if self.capabilities.gpio_count == 8 && pin.number() > 7 {
            Err(Error::UnsupportedFeature(format!(
                "GPIO pin {} is not available on this device (only pins 0-7 supported)",
                pin.number()
            )))
        } else {
            Ok(())
        }
    }

    pub(crate) fn check_gpio_group_support(&self, group: GpioGroup) -> Result<()> {
        if self.capabilities.gpio_count == 8 && group == GpioGroup::Group1 {
            Err(unsupported_gpio_group1())
        } else {
            Ok(())
        }
    }
}
