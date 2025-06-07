//! GPIO interrupt functionality for XR2280x devices.

use crate::consts;
use crate::device::Xr2280x;
use crate::error::{Error, Result};
use crate::gpio::GpioPin;
use log::{debug, trace, warn};

/// Default timeout for interrupt reads in milliseconds.
const DEFAULT_INTERRUPT_TIMEOUT_MS: i32 = 1000;

/// Represents the data received in a GPIO interrupt report.
/// **Note:** The exact format and interpretation of this data is currently unknown.
#[derive(Debug, Clone)]
pub struct GpioInterruptReport {
    /// Raw binary data received from the interrupt report.
    pub raw_data: Vec<u8>,
}

/// Represents the data potentially parsed from a raw GPIO interrupt report.
/// **Note:** This structure is speculative and based on common HID interrupt patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGpioInterruptReport {
    /// Speculative bitmask indicating which GPIO pins in group 0 triggered the interrupt.
    pub trigger_mask_group0: u16, // Speculative
    /// Speculative bitmask indicating which GPIO pins in group 1 triggered the interrupt.
    pub trigger_mask_group1: u16, // Speculative
    /// Current logic state of GPIO pins 0-15 (assumed from first 2 bytes).
    pub current_state_group0: u16, // Assumed from first 2 bytes
    /// Current logic state of GPIO pins 16-31 (assumed from next 2 bytes).
    pub current_state_group1: u16, // Assumed from next 2 bytes
}

impl Xr2280x {
    // --- GPIO Interrupt Handling ---
    /// Configures interrupt settings for a GPIO pin (enable, edge selection).
    /// This configures the pin to generate an interrupt on the selected edge(s).
    pub fn gpio_configure_interrupt(
        &self,
        pin: GpioPin,
        enable: bool,
        positive_edge: bool,
        negative_edge: bool,
    ) -> Result<()> {
        // Check support
        self.check_gpio_pin_support(pin)?;

        let (reg_mask, reg_pos, reg_neg) = if pin.group_index() == 0 {
            (
                consts::edge::REG_INTR_MASK_0,
                consts::edge::REG_INTR_POS_EDGE_0,
                consts::edge::REG_INTR_NEG_EDGE_0,
            )
        } else {
            (
                consts::edge::REG_INTR_MASK_1,
                consts::edge::REG_INTR_POS_EDGE_1,
                consts::edge::REG_INTR_NEG_EDGE_1,
            )
        };

        debug!(
            "Configuring interrupt for pin {}: enable={}, pos_edge={}, neg_edge={}",
            pin.number(),
            enable,
            positive_edge,
            negative_edge
        );

        // Enable/disable interrupt for this pin
        let mask_val = self.read_hid_register(reg_mask)?;
        let new_mask = if enable {
            mask_val | pin.mask()
        } else {
            mask_val & !pin.mask()
        };
        self.write_hid_register(reg_mask, new_mask)?;

        // Set edge detection if enabling
        if enable {
            // Positive edge
            let pos_val = self.read_hid_register(reg_pos)?;
            let new_pos = if positive_edge {
                pos_val | pin.mask()
            } else {
                pos_val & !pin.mask()
            };
            self.write_hid_register(reg_pos, new_pos)?;

            // Negative edge
            let neg_val = self.read_hid_register(reg_neg)?;
            let new_neg = if negative_edge {
                neg_val | pin.mask()
            } else {
                neg_val & !pin.mask()
            };
            self.write_hid_register(reg_neg, new_neg)?;
        }

        Ok(())
    }

    /// Reads a GPIO interrupt report with an optional timeout.
    /// Returns the raw interrupt data when an interrupt occurs.
    /// **Note:** The format of this data is currently unknown/undocumented.
    pub fn read_gpio_interrupt_report(
        &self,
        timeout_ms: Option<i32>,
    ) -> Result<GpioInterruptReport> {
        let timeout = timeout_ms.unwrap_or(DEFAULT_INTERRUPT_TIMEOUT_MS);
        let mut buffer = vec![0u8; 64]; // Adjust size as needed

        debug!("Reading GPIO interrupt report with timeout {}ms", timeout);
        match self.device.read_timeout(&mut buffer, timeout) {
            Ok(size) => {
                trace!("Received interrupt report: {:02X?}", &buffer[..size]);
                Ok(GpioInterruptReport {
                    raw_data: buffer[..size].to_vec(),
                })
            }
            Err(e) => {
                warn!("Failed to read interrupt report: {}", e);
                Err(Error::Hid(e))
            }
        }
    }

    /// Attempts to parse a raw GPIO interrupt report.
    /// **Warning:** This is speculative based on common HID patterns.
    pub fn parse_gpio_interrupt_report(
        &self,
        report: &GpioInterruptReport,
    ) -> Result<ParsedGpioInterruptReport> {
        // Speculative parsing based on common patterns
        if report.raw_data.len() < 4 {
            return Err(Error::InterruptParseError(
                "Interrupt report too small".to_string(),
            ));
        }

        // Assume first 4 bytes are the current GPIO states (2 bytes per group)
        let current_state_group0 = u16::from_le_bytes([report.raw_data[0], report.raw_data[1]]);
        let current_state_group1 = u16::from_le_bytes([report.raw_data[2], report.raw_data[3]]);

        // If there's more data, it might be trigger masks
        let (trigger_mask_group0, trigger_mask_group1) = if report.raw_data.len() >= 8 {
            (
                u16::from_le_bytes([report.raw_data[4], report.raw_data[5]]),
                u16::from_le_bytes([report.raw_data[6], report.raw_data[7]]),
            )
        } else {
            (0, 0) // No trigger info available
        };

        Ok(ParsedGpioInterruptReport {
            trigger_mask_group0,
            trigger_mask_group1,
            current_state_group0,
            current_state_group1,
        })
    }
}
