//! GPIO (General Purpose Input/Output) functionality for XR2280x devices.
//!
//! # Performance Considerations
//!
//! **⚠️ IMPORTANT**: Individual GPIO operations are inefficient due to HID communication overhead.
//! Each function call typically requires 2-4 HID Feature Report transactions with the device.
//!
//! ## HID Transaction Costs
//!
//! | Operation | HID Transactions | Notes |
//! |-----------|------------------|-------|
//! | `gpio_set_direction()` | 2 | 1 read + 1 write |
//! | `gpio_write()` | 1 | Uses SET/CLEAR registers |
//! | `gpio_read()` | 1 | Single read |
//! | `gpio_set_pull()` | 4 | 2 reads + 2 writes (both pull registers) |
//! | `gpio_set_open_drain()` | 2 | 1 read + 1 write |
//! | `gpio_set_tri_state()` | 2 | 1 read + 1 write |
//!
//! ## Performance Recommendations
//!
//! **✅ DO:**
//! - Use `gpio_setup_output()` and `gpio_setup_input()` for single pins (5 vs 8 transactions)
//! - Use `gpio_setup_outputs()` and `gpio_setup_inputs()` for multiple pins (6 total vs 8×N)
//! - Use `gpio_write_masked()` for updating multiple pins simultaneously
//! - Batch configuration changes together
//! - Group operations by GPIO group (0-15 vs 16-31) when possible
//!
//! **⚠️ AVOID:**
//! - Calling individual setup functions in loops
//! - Multiple `gpio_write()` calls when `gpio_write_masked()` could be used
//! - Mixing individual and bulk operations unnecessarily
//!
//! ## Example: Efficient vs Inefficient
//!
//! ```rust,no_run
//! # use xr2280x_hid::{Xr2280x, gpio::*};
//! # fn example(device: &Xr2280x) -> xr2280x_hid::Result<()> {
//! let pins = [GpioPin::new(0)?, GpioPin::new(1)?, GpioPin::new(2)?];
//!
//! // ❌ INEFFICIENT: ~24 HID transactions (8 per pin)
//! for pin in &pins {
//!     device.gpio_set_direction(*pin, GpioDirection::Output)?;
//!     device.gpio_set_pull(*pin, GpioPull::None)?;
//!     device.gpio_write(*pin, GpioLevel::Low)?;
//! }
//!
//! // ✅ EFFICIENT: ~6 HID transactions total
//! device.gpio_setup_outputs(
//!     &pins.iter().map(|&p| (p, GpioLevel::Low)).collect::<Vec<_>>(),
//!     GpioPull::None
//! )?;
//! # Ok(())
//! # }
//! ```

use crate::consts;
use crate::device::Xr2280x;
use crate::error::{
    Error, Result, gpio_register_read_error, gpio_register_write_error, unsupported_gpio_group1,
};
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
        let reg = match pin.group_index() {
            0 => consts::edge::REG_FUNC_SEL_0,
            _ => consts::edge::REG_FUNC_SEL_1,
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
        let reg = match pin.group_index() {
            0 => consts::edge::REG_FUNC_SEL_0,
            _ => consts::edge::REG_FUNC_SEL_1,
        };
        let value = self.read_gpio_register(pin, reg)?;
        Ok((value & pin.mask()) != 0)
    }

    /// Sets the direction of a GPIO pin (Input or Output).
    ///
    /// **Performance**: Uses 2 HID transactions (1 read + 1 write).
    /// For better performance with multiple pins, use `gpio_set_direction_masked()` or the
    /// `gpio_setup_*()` functions.
    /// Sets the direction of a GPIO pin (input or output).
    pub fn gpio_set_direction(&self, pin: GpioPin, direction: GpioDirection) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let reg = match pin.group_index() {
            0 => consts::edge::REG_DIR_0,
            _ => consts::edge::REG_DIR_1,
        };
        let current = self.read_gpio_register(pin, reg)?;
        let new_value = match direction {
            GpioDirection::Input => current & !pin.mask(), // 0 = Input
            GpioDirection::Output => current | pin.mask(), // 1 = Output
        };
        debug!(
            "Setting GPIO pin {} direction to {:?}",
            pin.number(),
            direction
        );
        self.write_gpio_register(pin, reg, new_value)?;
        Ok(())
    }

    /// Gets the direction of a GPIO pin (Input or Output).
    pub fn gpio_get_direction(&self, pin: GpioPin) -> Result<GpioDirection> {
        self.check_gpio_pin_support(pin)?;
        let reg = match pin.group_index() {
            0 => consts::edge::REG_DIR_0,
            _ => consts::edge::REG_DIR_1,
        };
        let value = self.read_gpio_register(pin, reg)?;
        Ok(match (value & pin.mask()) != 0 {
            true => GpioDirection::Output,
            false => GpioDirection::Input,
        })
    }

    /// Writes a level to a GPIO pin configured as output.
    ///
    /// **Performance**: Uses 1 HID transaction. For multiple pins, use `gpio_write_masked()`
    /// to write several pins in the same group with just 1-2 transactions total.
    pub fn gpio_write(&self, pin: GpioPin, level: GpioLevel) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let (reg_set, reg_clear) = match pin.group_index() {
            0 => (consts::edge::REG_SET_0, consts::edge::REG_CLEAR_0),
            _ => (consts::edge::REG_SET_1, consts::edge::REG_CLEAR_1),
        };
        debug!(
            "Writing {:?} to GPIO pin {} (mask=0x{:04X})",
            level,
            pin.number(),
            pin.mask()
        );
        match level {
            GpioLevel::High => self.write_gpio_register(pin, reg_set, pin.mask())?,
            GpioLevel::Low => self.write_gpio_register(pin, reg_clear, pin.mask())?,
        }
        Ok(())
    }

    /// Reads the current level of a GPIO pin.
    pub fn gpio_read(&self, pin: GpioPin) -> Result<GpioLevel> {
        self.check_gpio_pin_support(pin)?;
        let reg = match pin.group_index() {
            0 => consts::edge::REG_STATE_0,
            _ => consts::edge::REG_STATE_1,
        };
        let value = self.read_gpio_register(pin, reg)?;
        let level = match (value & pin.mask()) != 0 {
            true => GpioLevel::High,
            false => GpioLevel::Low,
        };
        trace!("GPIO pin {} read as {:?}", pin.number(), level);
        Ok(level)
    }

    /// Sets the pull resistor configuration for a GPIO pin.
    ///
    /// **Performance**: Uses 4 HID transactions (2 reads + 2 writes for pull-up/pull-down registers).
    /// This is the most expensive individual GPIO operation. For better performance, use
    /// `gpio_set_pull_masked()` or the `gpio_setup_*()` functions.
    pub fn gpio_set_pull(&self, pin: GpioPin, pull: GpioPull) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let (reg_up, reg_down) = match pin.group_index() {
            0 => (consts::edge::REG_PULL_UP_0, consts::edge::REG_PULL_DOWN_0),
            _ => (consts::edge::REG_PULL_UP_1, consts::edge::REG_PULL_DOWN_1),
        };

        debug!("Setting GPIO pin {} pull to {:?}", pin.number(), pull);

        match pull {
            GpioPull::None => {
                // Clear both pull-up and pull-down
                let up_val = self.read_gpio_register(pin, reg_up)?;
                self.write_gpio_register(pin, reg_up, up_val & !pin.mask())?;
                let down_val = self.read_gpio_register(pin, reg_down)?;
                self.write_gpio_register(pin, reg_down, down_val & !pin.mask())?;
            }
            GpioPull::Up => {
                // Set pull-up, clear pull-down
                let up_val = self.read_gpio_register(pin, reg_up)?;
                self.write_gpio_register(pin, reg_up, up_val | pin.mask())?;
                let down_val = self.read_gpio_register(pin, reg_down)?;
                self.write_gpio_register(pin, reg_down, down_val & !pin.mask())?;
            }
            GpioPull::Down => {
                // Clear pull-up, set pull-down
                let up_val = self.read_gpio_register(pin, reg_up)?;
                self.write_gpio_register(pin, reg_up, up_val & !pin.mask())?;
                let down_val = self.read_gpio_register(pin, reg_down)?;
                self.write_gpio_register(pin, reg_down, down_val | pin.mask())?;
            }
        }
        Ok(())
    }

    /// Gets the pull resistor configuration for a GPIO pin.
    pub fn gpio_get_pull(&self, pin: GpioPin) -> Result<GpioPull> {
        self.check_gpio_pin_support(pin)?;
        let (reg_up, reg_down) = match pin.group_index() {
            0 => (consts::edge::REG_PULL_UP_0, consts::edge::REG_PULL_DOWN_0),
            _ => (consts::edge::REG_PULL_UP_1, consts::edge::REG_PULL_DOWN_1),
        };

        let up_val = self.read_gpio_register(pin, reg_up)?;
        let down_val = self.read_gpio_register(pin, reg_down)?;

        let has_pull_up = (up_val & pin.mask()) != 0;
        let has_pull_down = (down_val & pin.mask()) != 0;

        Ok(match (has_pull_up, has_pull_down) {
            (true, false) => GpioPull::Up,
            (false, true) => GpioPull::Down,
            _ => GpioPull::None, // Both or neither
        })
    }

    /// Sets the open-drain configuration for a GPIO pin.
    ///
    /// **Performance**: Uses 2 HID transactions (1 read + 1 write).
    /// For multiple pins, use `gpio_set_open_drain_masked()`.
    pub fn gpio_set_open_drain(&self, pin: GpioPin, enable: bool) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_OPEN_DRAIN_0
        } else {
            consts::edge::REG_OPEN_DRAIN_1
        };
        let current = self.read_gpio_register(pin, reg)?;
        let new_value = if enable {
            current | pin.mask()
        } else {
            current & !pin.mask()
        };
        debug!("Setting GPIO pin {} open-drain to {}", pin.number(), enable);
        self.write_gpio_register(pin, reg, new_value)?;
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
        let value = self.read_gpio_register(pin, reg)?;
        Ok((value & pin.mask()) != 0)
    }

    /// Sets the tri-state (high-impedance) configuration for a GPIO pin.
    ///
    /// **Performance**: Uses 2 HID transactions (1 read + 1 write).
    /// For multiple pins, use `gpio_set_tri_state_masked()`.
    pub fn gpio_set_tri_state(&self, pin: GpioPin, enable: bool) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let reg = if pin.group_index() == 0 {
            consts::edge::REG_TRI_STATE_0
        } else {
            consts::edge::REG_TRI_STATE_1
        };
        let current = self.read_gpio_register(pin, reg)?;
        let new_value = if enable {
            current | pin.mask()
        } else {
            current & !pin.mask()
        };
        debug!("Setting GPIO pin {} tri-state to {}", pin.number(), enable);
        self.write_gpio_register(pin, reg, new_value)?;
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
        let value = self.read_gpio_register(pin, reg)?;
        Ok((value & pin.mask()) != 0)
    }

    // --- Efficient GPIO Configuration (Minimal HID Transactions) ---
    //
    // These functions are designed to minimize HID communication overhead by using
    // the bulk/masked operations internally and combining related configuration steps.
    /// Efficiently configure a GPIO pin for output with minimal HID transactions.
    /// This combines direction, pull, and initial level setting into optimized operations.
    ///
    /// **Performance**: Uses only 2-3 HID transactions vs 6-8 for individual calls.
    pub fn gpio_setup_output(
        &self,
        pin: GpioPin,
        initial_level: GpioLevel,
        pull: GpioPull,
    ) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let group = if pin.group_index() == 0 {
            GpioGroup::Group0
        } else {
            GpioGroup::Group1
        };

        // 1. Set pull configuration (2 HID transactions)
        self.gpio_set_pull_masked(group, pin.mask(), pull)?;

        // 2. Set direction to output (2 HID transactions)
        self.gpio_set_direction_masked(group, pin.mask(), GpioDirection::Output)?;

        // 3. Set initial level (1 HID transaction)
        self.gpio_write(pin, initial_level)?;

        debug!(
            "Efficiently configured GPIO pin {} as output: level={:?}, pull={:?}",
            pin.number(),
            initial_level,
            pull
        );
        Ok(())
    }

    /// Efficiently configure a GPIO pin for input with minimal HID transactions.
    /// This combines direction and pull setting into optimized operations.
    ///
    /// **Performance**: Uses only 4 HID transactions vs 6 for individual calls.
    pub fn gpio_setup_input(&self, pin: GpioPin, pull: GpioPull) -> Result<()> {
        self.check_gpio_pin_support(pin)?;
        let group = if pin.group_index() == 0 {
            GpioGroup::Group0
        } else {
            GpioGroup::Group1
        };

        // 1. Set pull configuration (4 HID transactions)
        self.gpio_set_pull_masked(group, pin.mask(), pull)?;

        // 2. Set direction to input (2 HID transactions)
        self.gpio_set_direction_masked(group, pin.mask(), GpioDirection::Input)?;

        debug!(
            "Efficiently configured GPIO pin {} as input: pull={:?}",
            pin.number(),
            pull
        );
        Ok(())
    }

    /// Apply a complete GPIO configuration efficiently using bulk operations.
    /// This batches multiple GPIO pins with the same settings to minimize HID transactions.
    ///
    /// **Performance**: Scales much better than individual pin operations.
    /// For N pins: ~6 HID transactions total vs ~8N for individual operations.
    pub fn gpio_apply_bulk_config(
        &self,
        pins: &[GpioPin],
        direction: GpioDirection,
        pull: GpioPull,
        initial_levels: Option<&[(GpioPin, GpioLevel)]>, // Only used for outputs
    ) -> Result<()> {
        if pins.is_empty() {
            return Ok(());
        }

        // Validate all pins and group them
        for pin in pins {
            self.check_gpio_pin_support(*pin)?;
        }

        // Group pins by GPIO group (0-15 vs 16-31)
        let mut group0_mask = 0u16;
        let mut group1_mask = 0u16;

        for pin in pins {
            if pin.group_index() == 0 {
                group0_mask |= pin.mask();
            } else {
                group1_mask |= pin.mask();
            }
        }

        // Apply pull configuration to all pins in each group
        if group0_mask != 0 {
            self.gpio_set_pull_masked(GpioGroup::Group0, group0_mask, pull)?;
        }
        if group1_mask != 0 {
            self.gpio_set_pull_masked(GpioGroup::Group1, group1_mask, pull)?;
        }

        // Apply direction to all pins in each group
        if group0_mask != 0 {
            self.gpio_set_direction_masked(GpioGroup::Group0, group0_mask, direction)?;
        }
        if group1_mask != 0 {
            self.gpio_set_direction_masked(GpioGroup::Group1, group1_mask, direction)?;
        }

        // Set initial levels for outputs (if specified)
        if matches!(direction, GpioDirection::Output) {
            if let Some(levels) = initial_levels {
                for (pin, level) in levels {
                    self.gpio_write(*pin, *level)?;
                }
            }
        }

        debug!(
            "Bulk configured {} GPIO pins: direction={:?}, pull={:?}",
            pins.len(),
            direction,
            pull
        );
        Ok(())
    }

    /// Convenience function to setup multiple output pins with the same configuration.
    /// This is much more efficient than calling gpio_setup_output for each pin individually.
    pub fn gpio_setup_outputs(
        &self,
        pin_configs: &[(GpioPin, GpioLevel)], // (pin, initial_level) pairs
        pull: GpioPull,
    ) -> Result<()> {
        let pins: Vec<GpioPin> = pin_configs.iter().map(|(pin, _)| *pin).collect();
        self.gpio_apply_bulk_config(&pins, GpioDirection::Output, pull, Some(pin_configs))?;
        Ok(())
    }

    /// Convenience function to setup multiple input pins with the same pull configuration.
    /// This is much more efficient than calling gpio_setup_input for each pin individually.
    pub fn gpio_setup_inputs(&self, pins: &[GpioPin], pull: GpioPull) -> Result<()> {
        self.gpio_apply_bulk_config(pins, GpioDirection::Input, pull, None)?;
        Ok(())
    }

    // --- GPIO Group Operations (Bulk) ---
    //
    // These masked operations are the most efficient way to configure multiple GPIO pins.
    // They operate on entire 16-bit register groups and require only 2 HID transactions each
    // (1 read + 1 write) regardless of how many pins are affected.
    /// Sets the direction of multiple GPIO pins in a group using a mask.
    /// Bit positions in the mask correspond to pins 0-15 within the group.
    ///
    /// **Performance**: Uses 2 HID transactions regardless of how many pins are affected.
    /// This is much more efficient than calling `gpio_set_direction()` multiple times.
    pub fn gpio_set_direction_masked(
        &self,
        group: GpioGroup,
        mask: u16,
        direction: GpioDirection,
    ) -> Result<()> {
        self.check_gpio_group_support(group)?;
        let reg_dir = self.get_gpio_reg_for_group(group, consts::edge::REG_DIR_0);

        let current = self.read_gpio_register_masked(group, reg_dir)?;
        let new_value = match direction {
            GpioDirection::Input => current & !mask, // 0 = Input
            GpioDirection::Output => current | mask, // 1 = Output
        };
        debug!(
            "Setting {:?} pins (mask=0x{:04X}) direction to {:?}",
            group, mask, direction
        );
        self.write_gpio_register_masked(group, reg_dir, new_value)?;
        Ok(())
    }

    /// Writes levels to multiple GPIO pins in a group.
    /// The `mask` determines which pins are affected (1 = write, 0 = ignore).
    /// The `values` determine the levels to write (1 = High, 0 = Low).
    ///
    /// **Performance**: Uses 1-2 HID transactions (depending on whether both SET and CLEAR
    /// operations are needed). Much more efficient than multiple `gpio_write()` calls.
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
            self.write_gpio_register_masked(group, reg_set, set_mask)?;
        }
        if clear_mask != 0 {
            self.write_gpio_register_masked(group, reg_clear, clear_mask)?;
        }
        Ok(())
    }

    /// Reads the current levels of all GPIO pins in a group.
    /// Returns a 16-bit value where each bit represents a pin's state (1 = High, 0 = Low).
    pub fn gpio_read_group(&self, group: GpioGroup) -> Result<u16> {
        self.check_gpio_group_support(group)?;
        let reg_state = self.get_gpio_reg_for_group(group, consts::edge::REG_STATE_0);
        let value = self.read_gpio_register_masked(group, reg_state)?;
        trace!("Read {:?} state: 0x{:04X}", group, value);
        Ok(value)
    }

    /// Sets the pull resistor configuration for multiple GPIO pins in a group.
    ///
    /// **Performance**: Uses 4 HID transactions (2 reads + 2 writes for pull-up/pull-down registers).
    /// Still much more efficient than multiple `gpio_set_pull()` calls.
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

        let current = self.read_gpio_register_masked(group, reg_od)?;
        let new_value = if enable {
            current | mask
        } else {
            current & !mask
        };
        debug!(
            "Setting {:?} pins (mask=0x{:04X}) open-drain to {}",
            group, mask, enable
        );
        self.write_gpio_register_masked(group, reg_od, new_value)?;
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

        let current = self.read_gpio_register_masked(group, reg_ts)?;
        let new_value = if enable {
            current | mask
        } else {
            current & !mask
        };
        debug!(
            "Setting {:?} pins (mask=0x{:04X}) tri-state to {}",
            group, mask, enable
        );
        self.write_gpio_register_masked(group, reg_ts, new_value)?;
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

    /// Check if the specified GPIO group is supported by this device.
    pub(crate) fn check_gpio_group_support(&self, group: GpioGroup) -> Result<()> {
        if self.capabilities.gpio_count == 8 && group == GpioGroup::Group1 {
            Err(unsupported_gpio_group1())
        } else {
            Ok(())
        }
    }

    /// GPIO-specific wrapper for reading HID registers with enhanced error context.
    fn read_gpio_register(&self, pin: GpioPin, register: u16) -> Result<u16> {
        self.read_hid_register(register).map_err(|e| match e {
            Error::Hid(hid_err) => gpio_register_read_error(
                pin.number(),
                register,
                format!("HID communication error: {}", hid_err),
            ),
            Error::InvalidReport(_) => gpio_register_read_error(
                pin.number(),
                register,
                "Invalid HID report received - check device connection".to_string(),
            ),
            _ => e, // Pass through other error types unchanged
        })
    }

    /// GPIO-specific wrapper for writing HID registers with enhanced error context.
    fn write_gpio_register(&self, pin: GpioPin, register: u16, value: u16) -> Result<()> {
        self.write_hid_register(register, value)
            .map_err(|e| match e {
                Error::Hid(hid_err) => gpio_register_write_error(
                    pin.number(),
                    register,
                    format!("HID communication error: {}", hid_err),
                ),
                Error::InvalidReport(_) => gpio_register_write_error(
                    pin.number(),
                    register,
                    "Invalid HID report received - check device connection and power".to_string(),
                ),
                _ => e, // Pass through other error types unchanged
            })
    }

    /// Group-aware GPIO register read with enhanced error context for masked operations.
    fn read_gpio_register_masked(&self, group: GpioGroup, register: u16) -> Result<u16> {
        self.read_hid_register(register).map_err(|e| match e {
            Error::Hid(hid_err) => gpio_register_read_error(
                group as u8,
                register,
                format!(
                    "HID communication error for GPIO group {:?}: {}",
                    group, hid_err
                ),
            ),
            Error::InvalidReport(_) => gpio_register_read_error(
                group as u8, // Use group index as pseudo-pin for error context
                register,
                format!(
                    "Invalid HID report for GPIO group {:?} - check device connection",
                    group
                ),
            ),
            _ => e, // Pass through other error types unchanged
        })
    }

    /// Group-aware GPIO register write with enhanced error context for masked operations.
    fn write_gpio_register_masked(
        &self,
        group: GpioGroup,
        register: u16,
        value: u16,
    ) -> Result<()> {
        self.write_hid_register(register, value)
            .map_err(|e| match e {
                Error::Hid(hid_err) => gpio_register_write_error(
                    group as u8,
                    register,
                    format!("HID communication error for GPIO group {:?}: {}", group, hid_err),
                ),
                Error::InvalidReport(_) => gpio_register_write_error(
                    group as u8, // Use group index as pseudo-pin for error context
                    register,
                    format!("Invalid HID report for GPIO group {:?} - check device connection and power", group),
                ),
                _ => e, // Pass through other error types unchanged
            })
    }
}
