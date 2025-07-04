//! GPIO interrupt functionality for XR2280x devices.

use crate::consts;
use crate::device::Xr2280x;
use crate::error::{Error, Result};
use crate::gpio::{GpioEdge, GpioPin};
use log::{debug, trace, warn};

/// Default timeout for interrupt reads in milliseconds.
const DEFAULT_INTERRUPT_TIMEOUT_MS: i32 = 1000;

// HID Report Structure Constants - GPIO Interrupt Parsing
// These constants define the structure of GPIO interrupt HID reports to eliminate magic numbers

/// GPIO Interrupt Report Structure Constants
#[allow(dead_code)]
mod report_offsets {
    /// HID Report ID is always at offset 0 (added by hidapi)
    pub const REPORT_ID: usize = 0;

    /// Minimum expected report size (Report ID + Group0 State + Group1 State)
    pub const MIN_REPORT_SIZE: usize = 5;

    /// Minimum report size including trigger mask data
    pub const MIN_REPORT_WITH_TRIGGERS: usize = 9;

    /// GPIO Group 0 (pins 0-15) state - Low byte offset
    pub const GROUP0_STATE_LOW: usize = 1;
    /// GPIO Group 0 (pins 0-15) state - High byte offset
    pub const GROUP0_STATE_HIGH: usize = 2;

    /// GPIO Group 1 (pins 16-31) state - Low byte offset
    pub const GROUP1_STATE_LOW: usize = 3;
    /// GPIO Group 1 (pins 16-31) state - High byte offset
    pub const GROUP1_STATE_HIGH: usize = 4;

    /// GPIO Group 0 (pins 0-15) trigger mask - Low byte offset
    pub const GROUP0_TRIGGER_LOW: usize = 5;
    /// GPIO Group 0 (pins 0-15) trigger mask - High byte offset
    pub const GROUP0_TRIGGER_HIGH: usize = 6;

    /// GPIO Group 1 (pins 16-31) trigger mask - Low byte offset
    pub const GROUP1_TRIGGER_LOW: usize = 7;
    /// GPIO Group 1 (pins 16-31) trigger mask - High byte offset
    pub const GROUP1_TRIGGER_HIGH: usize = 8;

    /// Offset where actual hardware data starts (skipping HID Report ID)
    pub const HARDWARE_DATA_START: usize = 1;
}

/// Raw GPIO interrupt report data received from XR2280x EDGE interface.
///
/// This structure contains the unprocessed binary data from a GPIO interrupt report.
/// The format and interpretation of this data is **currently unknown and undocumented**.
///
/// ## Safe Usage
///
/// This structure provides direct access to the raw interrupt data without any
/// interpretation or assumptions. Applications can:
///
/// - Analyze the raw bytes to understand the actual format
/// - Implement custom parsing logic after hardware verification
/// - Log the data for reverse engineering purposes
///
/// ## Data Format
///
/// - `raw_data[0]`: HID Report ID (automatically added by hidapi)
/// - `raw_data[1..]`: Actual interrupt report data from hardware (format unknown)
///
/// Use the safe `get_raw_interrupt_data()` method to access this data without
/// making any parsing assumptions.
#[derive(Debug, Clone)]
pub struct GpioInterruptReport {
    /// Raw binary data received from the interrupt report.
    /// Index 0 contains the HID Report ID, actual data starts at index 1.
    pub raw_data: Vec<u8>,
}

/// # ⚠️ CRITICAL WARNING: Speculative Data Structure ⚠️
///
/// **This structure contains potentially INCORRECT data parsed from XR2280x GPIO interrupt reports
/// using UNVERIFIED format assumptions. The actual hardware report format is undocumented.**
///
/// ## Risk Summary
///
/// - **Data Accuracy**: All fields may contain completely incorrect values
/// - **Hardware Mismatch**: Parsing assumptions may not match actual XR2280x behavior
/// - **Application Impact**: Using this data may cause incorrect system behavior
///
/// ## Field Reliability
///
/// All fields are marked as **SPECULATIVE** and should be treated as potentially incorrect:
///
/// - Values may not correspond to actual GPIO pin states
/// - Trigger masks may not indicate actual interrupt sources
/// - Data interpretation is based on unverified common HID patterns
///
/// ## Usage Recommendations
///
/// 1. **Extensive Testing**: Verify all parsed values against known hardware states
/// 2. **Validation Logic**: Implement sanity checks for implausible values
/// 3. **Fallback Mechanisms**: Design graceful handling of incorrect data
/// 4. **Consider Alternatives**: Use raw data access or GPIO polling instead
///
/// This structure is only accessible through the `unsafe` parsing function to acknowledge these risks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGpioInterruptReport {
    /// **SPECULATIVE**: Bitmask indicating which GPIO pins in group 0 may have triggered the interrupt.
    /// **WARNING**: This interpretation is unverified and may be completely incorrect.
    pub trigger_mask_group0: u16,
    /// **SPECULATIVE**: Bitmask indicating which GPIO pins in group 1 may have triggered the interrupt.
    /// **WARNING**: This interpretation is unverified and may be completely incorrect.
    pub trigger_mask_group1: u16,
    /// **SPECULATIVE**: Current logic state of GPIO pins 0-15 (assumed from bytes 1-2).
    /// **WARNING**: This interpretation is unverified and may be completely incorrect.
    pub current_state_group0: u16,
    /// **SPECULATIVE**: Current logic state of GPIO pins 16-31 (assumed from bytes 3-4).
    /// **WARNING**: This interpretation is unverified and may be completely incorrect.
    pub current_state_group1: u16,
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

        let (reg_mask, reg_pos, reg_neg) = match pin.group_index() {
            0 => (
                consts::edge::REG_INTR_MASK_0,
                consts::edge::REG_INTR_POS_EDGE_0,
                consts::edge::REG_INTR_NEG_EDGE_0,
            ),
            _ => (
                consts::edge::REG_INTR_MASK_1,
                consts::edge::REG_INTR_POS_EDGE_1,
                consts::edge::REG_INTR_NEG_EDGE_1,
            ),
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
        let new_mask = match enable {
            true => mask_val | pin.mask(),
            false => mask_val & !pin.mask(),
        };
        self.write_hid_register(reg_mask, new_mask)?;

        // Set edge detection if enabling
        if enable {
            // Positive edge
            let pos_val = self.read_hid_register(reg_pos)?;
            let new_pos = match positive_edge {
                true => pos_val | pin.mask(),
                false => pos_val & !pin.mask(),
            };
            self.write_hid_register(reg_pos, new_pos)?;

            // Negative edge
            let neg_val = self.read_hid_register(reg_neg)?;
            let new_neg = match negative_edge {
                true => neg_val | pin.mask(),
                false => neg_val & !pin.mask(),
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

        debug!("Reading GPIO interrupt report with timeout {timeout}ms");
        let edge_device = self.edge_device.as_ref().ok_or(Error::DeviceNotFound)?;
        match edge_device.read_timeout(&mut buffer, timeout) {
            Ok(size) => {
                trace!("Received interrupt report: {:02X?}", &buffer[..size]);
                Ok(GpioInterruptReport {
                    raw_data: buffer[..size].to_vec(),
                })
            }
            Err(e) => {
                warn!("Failed to read interrupt report: {e}");
                Err(Error::Hid(e))
            }
        }
    }

    /// # UNSAFE: Attempts to parse a raw GPIO interrupt report with UNVERIFIED format assumptions
    ///
    /// ## ⚠️ CRITICAL WARNING ⚠️
    ///
    /// **This function is marked `unsafe` because it makes unverified assumptions about the
    /// XR2280x EDGE HID interrupt report format that are NOT documented in any official datasheet.**
    ///
    /// ### Why This Function Is Unsafe
    ///
    /// 1. **Unverified Hardware Behavior**: The parsing logic is based entirely on speculation
    ///    about common HID patterns, not actual XR2280x documentation or reverse engineering.
    ///
    /// 2. **Potential Data Corruption**: If the actual hardware report format differs from
    ///    assumptions, this function may:
    ///    - Return completely incorrect GPIO pin states
    ///    - Misidentify interrupt trigger sources
    ///    - Cause application logic failures and unpredictable behavior
    ///
    /// 3. **Undefined Behavior Risk**: The function assumes specific byte positions and
    ///    endianness that may not match actual hardware behavior.
    ///
    /// ### Current Speculative Assumptions (UNVERIFIED)
    ///
    /// - `raw_data[0]`: HID Report ID (added by hidapi)
    /// - `raw_data[1..=2]`: GPIO Group 0 (pins 0-15) current state (little-endian u16)
    /// - `raw_data[3..=4]`: GPIO Group 1 (pins 16-31) current state (little-endian u16)
    /// - `raw_data[5..=6]`: GPIO Group 0 interrupt trigger mask (little-endian u16) *\[OPTIONAL\]*
    /// - `raw_data[7..=8]`: GPIO Group 1 interrupt trigger mask (little-endian u16) *\[OPTIONAL\]*
    ///
    /// ### Safety Requirements
    ///
    /// **Before calling this function, you MUST ensure:**
    ///
    /// 1. **Thorough Testing**: Test extensively with your specific hardware to verify
    ///    the parsed values match expected GPIO states and interrupt conditions.
    ///
    /// 2. **Validation Logic**: Implement application-level validation to detect
    ///    implausible values that could indicate parsing errors.
    ///
    /// 3. **Fallback Handling**: Design your application to handle incorrect interrupt
    ///    data gracefully without causing system failures.
    ///
    /// 4. **Hardware Verification**: If possible, cross-reference parsed values with
    ///    direct GPIO register reads to verify correctness.
    ///
    /// ### Recommended Alternatives
    ///
    /// - **Raw Data Access**: Use `GpioInterruptReport.raw_data` directly and implement
    ///   your own parsing after hardware verification.
    ///
    /// - **Polling**: Consider GPIO polling via `gpio_read()` if interrupt parsing
    ///   proves unreliable.
    ///
    /// - **Hardware Documentation**: Contact MaxLinear/Exar for official documentation
    ///   of the interrupt report format.
    ///
    /// ## Parameters
    ///
    /// * `report` - Raw GPIO interrupt report received from hardware
    ///
    /// ## Returns
    ///
    /// * `Ok(ParsedGpioInterruptReport)` - **POTENTIALLY INCORRECT** parsed interrupt data
    /// * `Err(Error::InterruptParseError)` - Report format validation failed
    ///
    /// # Safety
    ///
    /// This function is unsafe because it makes unverified assumptions about the XR2280x
    /// EDGE HID interrupt report format. The caller must ensure:
    ///
    /// - The report data matches the assumed format through hardware testing
    /// - Parsed values are validated against actual GPIO states
    /// - Application can handle potentially incorrect interrupt data gracefully
    /// - Hardware verification confirms the byte layout assumptions are correct
    ///
    /// Using this function with incompatible hardware or report formats may result in
    /// incorrect GPIO state interpretation and unpredictable application behavior.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// # use xr2280x_hid::*;
    /// # fn example(device: &Xr2280x) -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let raw_report = device.read_gpio_interrupt_report(Some(1000))?;
    ///
    /// // UNSAFE: This parsing is speculative and may return incorrect data!
    /// let parsed = unsafe {
    ///     device.parse_gpio_interrupt_report(&raw_report)?
    /// };
    ///
    /// // CRITICAL: Validate parsed data against known hardware state
    /// println!("⚠️  UNVERIFIED Group 0 state: 0x{:04X}", parsed.current_state_group0);
    /// println!("⚠️  UNVERIFIED Group 1 state: 0x{:04X}", parsed.current_state_group1);
    ///
    /// // Application MUST implement validation logic here
    /// // to detect and handle potentially incorrect data
    /// # Ok(())
    /// # }
    /// ```
    pub unsafe fn parse_gpio_interrupt_report(
        &self,
        report: &GpioInterruptReport,
    ) -> Result<ParsedGpioInterruptReport> {
        // Comprehensive input validation with detailed error messages
        if report.raw_data.is_empty() {
            return Err(Error::InterruptParseError(
                "Interrupt report is empty - no data to parse".to_string(),
            ));
        }

        if report.raw_data.len() < report_offsets::MIN_REPORT_SIZE {
            return Err(Error::InterruptParseError(format!(
                "Interrupt report too small: got {} bytes, need at least {} bytes (Report ID + 4 state bytes). \
                    This may indicate an incompatible hardware report format.",
                report.raw_data.len(),
                report_offsets::MIN_REPORT_SIZE
            )));
        }

        // Log the raw data for debugging/verification purposes
        debug!(
            "⚠️  UNSAFE: Parsing GPIO interrupt report with UNVERIFIED format assumptions. \
            Raw data ({} bytes): {:02X?}",
            report.raw_data.len(),
            report.raw_data
        );

        // UNSAFE ASSUMPTION: First 4 bytes after Report ID are GPIO states (2 bytes per group)
        // WARNING: This assumption is NOT verified against hardware documentation
        let current_state_group0 = u16::from_le_bytes([
            report.raw_data[report_offsets::GROUP0_STATE_LOW],
            report.raw_data[report_offsets::GROUP0_STATE_HIGH],
        ]);
        let current_state_group1 = u16::from_le_bytes([
            report.raw_data[report_offsets::GROUP1_STATE_LOW],
            report.raw_data[report_offsets::GROUP1_STATE_HIGH],
        ]);

        // UNSAFE ASSUMPTION: Additional bytes might contain trigger masks
        // WARNING: This is pure speculation based on common patterns
        let (trigger_mask_group0, trigger_mask_group1) = if report.raw_data.len()
            >= report_offsets::MIN_REPORT_WITH_TRIGGERS
        {
            // Additional bounds checking for trigger mask data
            if report.raw_data.len() < report_offsets::MIN_REPORT_WITH_TRIGGERS {
                return Err(Error::InterruptParseError(format!(
                    "Interrupt report claims trigger data but insufficient bytes: got {} bytes, need {}",
                    report.raw_data.len(),
                    report_offsets::MIN_REPORT_WITH_TRIGGERS
                )));
            }

            (
                u16::from_le_bytes([
                    report.raw_data[report_offsets::GROUP0_TRIGGER_LOW],
                    report.raw_data[report_offsets::GROUP0_TRIGGER_HIGH],
                ]),
                u16::from_le_bytes([
                    report.raw_data[report_offsets::GROUP1_TRIGGER_LOW],
                    report.raw_data[report_offsets::GROUP1_TRIGGER_HIGH],
                ]),
            )
        } else {
            warn!(
                "GPIO interrupt report only {} bytes - no trigger mask data available. \
                Setting trigger masks to 0.",
                report.raw_data.len()
            );
            (0, 0) // No trigger info available
        };

        // Log parsed values for verification
        debug!(
            "⚠️  UNVERIFIED parsed GPIO interrupt: Group0 state=0x{current_state_group0:04X} triggers=0x{trigger_mask_group0:04X}, \
            Group1 state=0x{current_state_group1:04X} triggers=0x{trigger_mask_group1:04X}"
        );

        warn!(
            "⚠️  CRITICAL: Returning GPIO interrupt data parsed with UNVERIFIED assumptions! \
            Application MUST validate these values against known hardware state."
        );

        Ok(ParsedGpioInterruptReport {
            trigger_mask_group0,
            trigger_mask_group1,
            current_state_group0,
            current_state_group1,
        })
    }

    /// **SAFE**: Get raw GPIO interrupt report data without parsing assumptions.
    ///
    /// This function provides direct access to the raw interrupt report bytes
    /// without making any assumptions about the data format. This is the
    /// recommended approach until the actual XR2280x interrupt report format
    /// is properly documented or reverse-engineered.
    ///
    /// ## Usage
    ///
    /// Use this function to:
    /// - Implement custom parsing logic after hardware verification
    /// - Log raw data for analysis and reverse engineering
    /// - Avoid the risks of speculative parsing
    ///
    /// ## Data Layout
    ///
    /// - `[0]`: HID Report ID (added automatically by hidapi)
    /// - `[1..]`: Actual interrupt report data from XR2280x hardware
    ///
    /// ## Parameters
    ///
    /// * `report` - GPIO interrupt report received from hardware
    ///
    /// ## Returns
    ///
    /// Slice containing the raw interrupt data (excluding Report ID at index 0)
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// # use xr2280x_hid::*;
    /// # fn example(device: &Xr2280x) -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let raw_report = device.read_gpio_interrupt_report(Some(1000))?;
    ///
    /// // SAFE: Get raw data without parsing assumptions
    /// let raw_data = device.get_raw_interrupt_data(&raw_report);
    ///
    /// println!("Raw interrupt data ({} bytes): {:02X?}", raw_data.len(), raw_data);
    ///
    /// // Implement your own parsing logic here based on hardware verification
    /// // ...
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_raw_interrupt_data<'a>(&self, report: &'a GpioInterruptReport) -> &'a [u8] {
        if report.raw_data.is_empty() {
            &[]
        } else {
            // Skip the HID Report ID at index 0, return actual hardware data
            &report.raw_data[report_offsets::HARDWARE_DATA_START..]
        }
    }

    /// **IMPROVED**: Parse GPIO interrupt report into individual pin/edge combinations.
    ///
    /// This function provides a more ergonomic API by converting the raw group masks
    /// into individual `(GpioPin, GpioEdge)` combinations. This eliminates the need
    /// for callers to manually parse group masks and handle pin number conversions.
    ///
    /// ## Type Safety
    ///
    /// - Returns typed `GpioPin` instances instead of raw `u8` values
    /// - Handles `GpioPin::new()` conversion internally with proper error handling
    /// - Ensures all returned pins are valid (0-31 range)
    ///
    /// ## Edge Detection Logic
    ///
    /// The function analyzes interrupt trigger masks and current pin states to determine:
    /// - **Rising Edge**: Pin triggered and current state is high
    /// - **Falling Edge**: Pin triggered and current state is low
    /// - **Both**: When ambiguous, defaults to `GpioEdge::Both`
    ///
    /// ## Parameters
    ///
    /// * `report` - GPIO interrupt report received from hardware
    ///
    /// ## Returns
    ///
    /// * `Ok(Vec<(GpioPin, GpioEdge)>)` - List of pins that triggered with their edge types
    /// * `Err(Error)` - Parsing failed or invalid pin numbers detected
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// # use xr2280x_hid::*;
    /// # fn example(device: &Xr2280x) -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let raw_report = device.read_gpio_interrupt_report(Some(1000))?;
    ///
    /// // NEW: Get individual pin/edge combinations with type safety
    /// let pin_events = device.parse_gpio_interrupt_pins(&raw_report)?;
    ///
    /// for (pin, edge) in pin_events {
    ///     println!("Pin {} triggered on {:?} edge", pin.number(), edge);
    ///
    ///     // Can directly use typed pin with other GPIO functions
    ///     let current_level = device.gpio_read(pin)?;
    ///     println!("Current level: {:?}", current_level);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Safety
    ///
    /// This function is safe because it:
    /// - Uses the existing `parse_gpio_interrupt_report` for data extraction
    /// - Validates all pin numbers through `GpioPin::new()`
    /// - Returns errors for any invalid hardware data
    ///
    /// **Note**: The underlying interrupt report parsing is still speculative
    /// and may return incorrect data until the XR2280x interrupt format is
    /// properly documented.
    pub fn parse_gpio_interrupt_pins(
        &self,
        report: &GpioInterruptReport,
    ) -> Result<Vec<(GpioPin, GpioEdge)>> {
        // Use existing parsing function to get raw data
        let parsed = unsafe { self.parse_gpio_interrupt_report(report)? };

        let mut pin_events = Vec::new();

        // Process Group 0 (pins 0-15)
        let group0_triggers = parsed.trigger_mask_group0;
        let group0_states = parsed.current_state_group0;

        for bit_pos in 0..16 {
            if group0_triggers & (1 << bit_pos) != 0 {
                let pin_num = bit_pos;
                let pin = GpioPin::new(pin_num)?;

                // Determine edge type based on current state
                let is_high = group0_states & (1 << bit_pos) != 0;
                let edge = if is_high {
                    GpioEdge::Rising
                } else {
                    GpioEdge::Falling
                };

                pin_events.push((pin, edge));
            }
        }

        // Process Group 1 (pins 16-31)
        let group1_triggers = parsed.trigger_mask_group1;
        let group1_states = parsed.current_state_group1;

        for bit_pos in 0..16 {
            if group1_triggers & (1 << bit_pos) != 0 {
                let pin_num = bit_pos + 16;
                let pin = GpioPin::new(pin_num)?;

                // Determine edge type based on current state
                let is_high = group1_states & (1 << bit_pos) != 0;
                let edge = if is_high {
                    GpioEdge::Rising
                } else {
                    GpioEdge::Falling
                };

                pin_events.push((pin, edge));
            }
        }

        debug!(
            "Parsed {} GPIO interrupt events from report",
            pin_events.len()
        );

        Ok(pin_events)
    }
}
