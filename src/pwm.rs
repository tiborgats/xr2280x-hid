//! PWM (Pulse Width Modulation) functionality for XR2280x devices.

use crate::consts;
use crate::device::Xr2280x;
use crate::error::{Error, Result, pwm_hardware_error, pwm_parameter_error, unsupported_pwm_pin};
use crate::gpio::GpioPin;
use log::{debug, trace};

/// Represents the two PWM channels available.
/// PWM channel identifier for XR2280x devices.
///
/// XR2280x devices support up to 2 independent PWM channels that can be
/// assigned to any available GPIO pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwmChannel {
    /// PWM channel 0 - can be assigned to any GPIO pin.
    Pwm0,
    /// PWM channel 1 - can be assigned to any GPIO pin.
    Pwm1,
}

/// PWM command/mode for controlling PWM output behavior.
///
/// These commands control how the PWM channel behaves after being enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PwmCommand {
    /// PWM channel is idle (no output).
    Idle,
    /// Force PWM output to constantly low level.
    AssertLow,
    /// Generate a single PWM pulse and then stop.
    OneShot,
    /// Continuously generate PWM pulses with configured high/low periods.
    FreeRun,
    /// Undefined command value (for forward compatibility).
    Undefined(u16),
}

impl Xr2280x {
    // --- PWM Configuration ---
    /// Converts nanoseconds to PWM units (increments of ~266.667ns).
    /// Returns `Err` if the time is out of range (1 - 4095 units).
    pub fn ns_to_pwm_units(&self, nanoseconds: u64) -> Result<u16> {
        if nanoseconds == 0 {
            return Err(pwm_parameter_error(
                255, // Generic channel since this is a utility function
                "PWM time must be greater than 0 ns".to_string(),
            ));
        }
        let units = (nanoseconds as f64 / consts::edge::PWM_UNIT_TIME_NS).round() as u64;
        if units < consts::edge::PWM_MIN_UNITS as u64 {
            Err(pwm_parameter_error(
                255, // Generic channel since this is a utility function
                format!(
                    "PWM time {} ns is too small (min {} ns)",
                    nanoseconds,
                    (consts::edge::PWM_MIN_UNITS as f64 * consts::edge::PWM_UNIT_TIME_NS).round()
                        as u64
                ),
            ))
        } else if units > consts::edge::PWM_MAX_UNITS as u64 {
            Err(pwm_parameter_error(
                255, // Generic channel since this is a utility function
                format!(
                    "PWM time {} ns is too large (max {} ns)",
                    nanoseconds,
                    (consts::edge::PWM_MAX_UNITS as f64 * consts::edge::PWM_UNIT_TIME_NS).round()
                        as u64
                ),
            ))
        } else {
            Ok(units as u16)
        }
    }

    /// Converts PWM units to nanoseconds (units * 266.667ns).
    pub fn pwm_units_to_ns(&self, units: u16) -> u64 {
        (units as f64 * consts::edge::PWM_UNIT_TIME_NS).round() as u64
    }

    /// Sets the high and low periods for a PWM channel in units (increments of ~266.667ns).
    pub fn pwm_set_periods(
        &self,
        channel: PwmChannel,
        high_units: u16,
        low_units: u16,
    ) -> Result<()> {
        let (reg_high, reg_low) = match channel {
            PwmChannel::Pwm0 => (consts::edge::REG_PWM0_HIGH, consts::edge::REG_PWM0_LOW),
            PwmChannel::Pwm1 => (consts::edge::REG_PWM1_HIGH, consts::edge::REG_PWM1_LOW),
        };

        match (high_units, low_units) {
            (1..=4095, 1..=4095) => {} // Valid range
            _ => {
                return Err(pwm_parameter_error(
                    channel as u8,
                    format!(
                        "PWM period units must be 1-4095 (got high={high_units}, low={low_units})"
                    ),
                ));
            }
        }
        debug!("Setting {channel:?} periods: high={high_units} units, low={low_units} units");
        self.write_pwm_register(channel, reg_high, high_units)?;
        self.write_pwm_register(channel, reg_low, low_units)?;
        Ok(())
    }

    /// Sets the high and low periods for a PWM channel in nanoseconds.
    pub fn pwm_set_periods_ns(&self, channel: PwmChannel, high_ns: u64, low_ns: u64) -> Result<()> {
        let high_units = self.ns_to_pwm_units(high_ns)?;
        let low_units = self.ns_to_pwm_units(low_ns)?;
        self.pwm_set_periods(channel, high_units, low_units)
    }

    /// Gets the high and low periods for a PWM channel in units (increments of ~266.667ns).
    pub fn pwm_get_periods(&self, channel: PwmChannel) -> Result<(u16, u16)> {
        let (reg_high, reg_low) = match channel {
            PwmChannel::Pwm0 => (consts::edge::REG_PWM0_HIGH, consts::edge::REG_PWM0_LOW),
            PwmChannel::Pwm1 => (consts::edge::REG_PWM1_HIGH, consts::edge::REG_PWM1_LOW),
        };
        let high_units = self.read_pwm_register(channel, reg_high)?;
        let low_units = self.read_pwm_register(channel, reg_low)?;
        trace!("Read {channel:?} periods: high={high_units} units, low={low_units} units");
        Ok((high_units, low_units))
    }

    /// Gets the high and low periods for a PWM channel in nanoseconds.
    pub fn pwm_get_periods_ns(&self, channel: PwmChannel) -> Result<(u64, u64)> {
        let (high_units, low_units) = self.pwm_get_periods(channel)?;
        Ok((
            self.pwm_units_to_ns(high_units),
            self.pwm_units_to_ns(low_units),
        ))
    }

    /// Sets the GPIO pin assigned to a PWM channel (0-31).
    pub fn pwm_set_pin(&self, channel: PwmChannel, pin: GpioPin) -> Result<()> {
        // XR22800/1 only support PWM on pins 0-7 (8 GPIOs)
        if self.capabilities.gpio_count == 8 && pin.number() > 7 {
            return Err(unsupported_pwm_pin(pin.number()));
        }

        let reg = match channel {
            PwmChannel::Pwm0 => consts::edge::REG_PWM0_CTRL,
            PwmChannel::Pwm1 => consts::edge::REG_PWM1_CTRL,
        };
        let current = self.read_pwm_register(channel, reg)?;
        let new_value = (current & !consts::edge::pwm_ctrl::PIN_MASK)
            | ((pin.number() as u16) << consts::edge::pwm_ctrl::PIN_SHIFT);
        debug!("Setting {:?} to pin {}", channel, pin.number());
        self.write_pwm_register(channel, reg, new_value)?;
        Ok(())
    }

    /// Gets the GPIO pin assigned to a PWM channel.
    pub fn pwm_get_pin(&self, channel: PwmChannel) -> Result<GpioPin> {
        let reg = match channel {
            PwmChannel::Pwm0 => consts::edge::REG_PWM0_CTRL,
            PwmChannel::Pwm1 => consts::edge::REG_PWM1_CTRL,
        };
        let value = self.read_pwm_register(channel, reg)?;
        let pin_num =
            ((value & consts::edge::pwm_ctrl::PIN_MASK) >> consts::edge::pwm_ctrl::PIN_SHIFT) as u8;
        GpioPin::new(pin_num)
    }

    /// Controls a PWM channel (enable/disable, set command mode).
    pub fn pwm_control(
        &self,
        channel: PwmChannel,
        enable: bool,
        command: PwmCommand,
    ) -> Result<()> {
        let reg = match channel {
            PwmChannel::Pwm0 => consts::edge::REG_PWM0_CTRL,
            PwmChannel::Pwm1 => consts::edge::REG_PWM1_CTRL,
        };
        let current = self.read_pwm_register(channel, reg)?;
        let enable_bits = if enable {
            consts::edge::pwm_ctrl::ENABLE_MASK
        } else {
            0
        };
        let cmd_bits = match command {
            PwmCommand::Idle => consts::edge::pwm_ctrl::CMD_IDLE,
            PwmCommand::AssertLow => consts::edge::pwm_ctrl::CMD_ASSERT_LOW,
            PwmCommand::OneShot => consts::edge::pwm_ctrl::CMD_ONE_SHOT,
            PwmCommand::FreeRun => consts::edge::pwm_ctrl::CMD_FREE_RUN,
            PwmCommand::Undefined(raw) => match raw & !0b111 {
                0 => raw,
                _ => {
                    return Err(pwm_parameter_error(
                        channel as u8,
                        "PWM command raw value must fit in 3 bits".to_string(),
                    ));
                }
            },
        };
        let cmd_shifted = cmd_bits << consts::edge::pwm_ctrl::CMD_SHIFT;
        let new_value = (current
            & !(consts::edge::pwm_ctrl::ENABLE_MASK | consts::edge::pwm_ctrl::CMD_MASK))
            | enable_bits
            | cmd_shifted;
        debug!(
            "Setting {channel:?}: enable={enable}, command={command:?} (ctrl=0x{new_value:04X})"
        );
        self.write_pwm_register(channel, reg, new_value)?;
        Ok(())
    }

    /// Gets the current state of a PWM channel (enabled, command mode).
    pub fn pwm_get_control(&self, channel: PwmChannel) -> Result<(bool, PwmCommand)> {
        let reg = match channel {
            PwmChannel::Pwm0 => consts::edge::REG_PWM0_CTRL,
            PwmChannel::Pwm1 => consts::edge::REG_PWM1_CTRL,
        };
        let value = self.read_pwm_register(channel, reg)?;
        let enabled = (value & consts::edge::pwm_ctrl::ENABLE_MASK) != 0;
        let cmd_raw =
            (value & consts::edge::pwm_ctrl::CMD_MASK) >> consts::edge::pwm_ctrl::CMD_SHIFT;
        let command = match cmd_raw {
            consts::edge::pwm_ctrl::CMD_IDLE => PwmCommand::Idle,
            consts::edge::pwm_ctrl::CMD_ASSERT_LOW => PwmCommand::AssertLow,
            consts::edge::pwm_ctrl::CMD_ONE_SHOT => PwmCommand::OneShot,
            consts::edge::pwm_ctrl::CMD_FREE_RUN => PwmCommand::FreeRun,
            _ => PwmCommand::Undefined(cmd_raw),
        };
        trace!("Read {channel:?} control: enabled={enabled}, command={command:?}");
        Ok((enabled, command))
    }

    /// PWM-specific wrapper for reading HID registers with enhanced error context.
    fn read_pwm_register(&self, channel: PwmChannel, register: u16) -> Result<u16> {
        self.read_hid_register(register).map_err(|e| match e {
            Error::Hid(hid_err) => pwm_hardware_error(
                channel as u8,
                format!("HID communication error for register 0x{register:04X}: {hid_err}"),
            ),
            Error::InvalidReport(_) => pwm_hardware_error(
                channel as u8,
                format!(
                    "Invalid HID report for register 0x{register:04X} - check device connection"
                ),
            ),
            _ => e, // Pass through other error types unchanged
        })
    }

    /// PWM-specific wrapper for writing HID registers with enhanced error context.
    fn write_pwm_register(&self, channel: PwmChannel, register: u16, value: u16) -> Result<()> {
        self.write_hid_register(register, value)
            .map_err(|e| match e {
                Error::Hid(hid_err) => pwm_hardware_error(
                    channel as u8,
                    format!("HID communication error for register 0x{register:04X}: {hid_err}"),
                ),
                Error::InvalidReport(_) => pwm_hardware_error(
                    channel as u8,
                    format!(
                        "Invalid HID report for register 0x{register:04X} - check device connection"
                    ),
                ),
                _ => e, // Pass through other error types unchanged
            })
    }
}
