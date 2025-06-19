//! GPIO Interrupt Handling: Safe vs Unsafe Approaches
//!
//! This example demonstrates how to handle GPIO interrupts from XR2280x devices,
//! comparing the safe raw data access approach with the unsafe speculative parsing.
//!
//! ## Key Concepts Demonstrated
//!
//! 1. **Safe Approach**: Using `get_raw_interrupt_data()` for raw access
//! 2. **Unsafe Approach**: Using `parse_gpio_interrupt_report()` with proper precautions
//! 3. **Data Validation**: Implementing sanity checks for parsed data
//! 4. **Error Handling**: Robust handling of interrupt parsing failures
//! 5. **Debugging**: Logging raw data for analysis and verification
//!
//! ## Hardware Requirements
//!
//! - XR22800/1/2/4 device with EDGE interface
//! - GPIO pins configured for interrupt generation
//! - Physical setup to trigger GPIO interrupts (buttons, sensors, etc.)

use std::collections::HashMap;
use std::time::{Duration, Instant};

use hidapi::HidApi;
use log::{debug, error, info, warn};

use xr2280x_hid::{
    Error, GpioInterruptReport, GpioLevel, GpioPin, ParsedGpioInterruptReport, Xr2280x,
};

/// Tracks GPIO pin state history for validation purposes
#[derive(Debug, Clone)]
struct GpioPinHistory {
    last_known_state: Option<GpioLevel>,
    state_changes: u32,
    last_interrupt_time: Option<Instant>,
}

impl GpioPinHistory {
    fn new() -> Self {
        Self {
            last_known_state: None,
            state_changes: 0,
            last_interrupt_time: None,
        }
    }

    fn update_state(&mut self, new_state: GpioLevel) {
        if self.last_known_state.is_some() && self.last_known_state != Some(new_state) {
            self.state_changes += 1;
        }
        self.last_known_state = Some(new_state);
        self.last_interrupt_time = Some(Instant::now());
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to see debug output
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    info!("🚀 GPIO Interrupt Handling Example: Safe vs Unsafe Approaches");

    // Initialize HID API and open device
    let hid_api = HidApi::new()?;
    let device = Xr2280x::device_open_first(&hid_api)?;

    info!("✅ Successfully opened XR2280x device");

    // Check if device has EDGE interface for GPIO interrupts
    let capabilities = device.get_capabilities();
    if capabilities.gpio_count == 0 {
        error!("❌ Device does not support GPIO functionality");
        return Err("No GPIO support".into());
    }

    info!(
        "📌 Device supports {} GPIO pins - setting up interrupt demonstration",
        capabilities.gpio_count
    );

    // Setup GPIO pins for interrupt demonstration
    setup_gpio_interrupts(&device)?;

    info!("🔧 GPIO interrupts configured - starting monitoring loop");
    info!("");
    info!("📋 This example will demonstrate both SAFE and UNSAFE interrupt handling:");
    info!("   • SAFE: Raw data access without parsing assumptions");
    info!("   • UNSAFE: Speculative parsing with validation checks");
    info!("");
    info!("💡 Trigger some GPIO interrupts (connect/disconnect pins, press buttons, etc.)");
    info!("   Press Ctrl+C to exit");
    info!("");

    // Track pin state history for validation
    let mut pin_history: HashMap<u8, GpioPinHistory> = HashMap::new();
    for pin_num in 0..std::cmp::min(capabilities.gpio_count, 16) {
        pin_history.insert(pin_num, GpioPinHistory::new());
    }

    // Main interrupt monitoring loop
    let mut interrupt_count = 0;
    loop {
        match monitor_interrupts_safely(&device, &mut pin_history, interrupt_count) {
            Ok(true) => {
                interrupt_count += 1;
                // Brief pause between interrupt checks
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok(false) => {
                // No interrupt received, brief pause
                std::thread::sleep(Duration::from_millis(500));
            }
            Err(e) => {
                error!("❌ Interrupt monitoring error: {}", e);
                std::thread::sleep(Duration::from_secs(1));
            }
        }

        // Periodically show summary
        if interrupt_count > 0 && interrupt_count % 10 == 0 {
            show_interrupt_summary(&pin_history);
        }
    }
}

/// Setup GPIO pins to generate interrupts
fn setup_gpio_interrupts(device: &Xr2280x) -> xr2280x_hid::Result<()> {
    info!("🔧 Configuring GPIO pins for interrupt generation...");

    // Configure first few pins as inputs with interrupts on both edges
    for pin_num in 0..4 {
        if let Ok(pin) = GpioPin::new(pin_num) {
            // Assign pin to EDGE interface
            device.gpio_assign_to_edge(pin)?;

            // Configure as input with pull-up
            device.gpio_setup_input(pin, xr2280x_hid::GpioPull::Up)?;

            // Enable interrupts on both positive and negative edges
            device.gpio_configure_interrupt(pin, true, true, true)?;

            debug!("✅ Configured GPIO pin {} for interrupts", pin_num);
        }
    }

    info!("✅ GPIO interrupt configuration complete");
    Ok(())
}

/// Demonstrate both safe and unsafe interrupt handling approaches
fn monitor_interrupts_safely(
    device: &Xr2280x,
    pin_history: &mut HashMap<u8, GpioPinHistory>,
    interrupt_count: u32,
) -> xr2280x_hid::Result<bool> {
    // Try to read an interrupt report with short timeout
    match device.read_gpio_interrupt_report(Some(100)) {
        Ok(raw_report) => {
            info!("");
            info!("🎯 INTERRUPT #{} DETECTED!", interrupt_count + 1);
            info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            // APPROACH 1: SAFE - Raw data access
            handle_interrupt_safely(device, &raw_report)?;

            info!("");

            // APPROACH 2: UNSAFE - Speculative parsing with validation
            handle_interrupt_unsafely(device, &raw_report, pin_history)?;

            info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            Ok(true)
        }
        Err(Error::Hid(hidapi::HidError::HidApiError { message }))
            if message.contains("timeout") =>
        {
            // Timeout is expected when no interrupts occur
            Ok(false)
        }
        Err(e) => Err(e),
    }
}

/// SAFE APPROACH: Handle interrupt using raw data access
fn handle_interrupt_safely(
    device: &Xr2280x,
    raw_report: &GpioInterruptReport,
) -> xr2280x_hid::Result<()> {
    info!("🛡️  SAFE APPROACH: Raw Data Access");

    // Get raw interrupt data without any parsing assumptions
    let raw_data = device.get_raw_interrupt_data(raw_report);

    info!("📊 Raw interrupt data ({} bytes):", raw_data.len());
    info!("   Hex: {:02X?}", raw_data);
    info!(
        "   Binary: {}",
        raw_data
            .iter()
            .map(|b| format!("{:08b}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );

    // Example: Implement basic pattern recognition without assumptions
    if raw_data.len() >= 4 {
        let word1 = u16::from_le_bytes([raw_data[0], raw_data[1]]);
        let word2 = u16::from_le_bytes([raw_data[2], raw_data[3]]);

        info!("🔍 Pattern Analysis:");
        info!("   First 16-bit word:  0x{:04X} ({})", word1, word1);
        info!("   Second 16-bit word: 0x{:04X} ({})", word2, word2);

        // Check for obvious patterns that might indicate data validity
        let has_data_pattern = word1 != 0 || word2 != 0;
        let all_bits_set = word1 == 0xFFFF && word2 == 0xFFFF;
        let reasonable_pin_count = word1.count_ones() <= 16 && word2.count_ones() <= 16;

        info!("   Data present: {}", has_data_pattern);
        info!("   All bits set (suspicious): {}", all_bits_set);
        info!("   Pin count reasonable: {}", reasonable_pin_count);
    }

    // This approach allows custom parsing logic based on actual hardware behavior
    info!("💡 Custom parsing logic would go here based on hardware verification");

    Ok(())
}

/// UNSAFE APPROACH: Handle interrupt using speculative parsing with extensive validation
fn handle_interrupt_unsafely(
    device: &Xr2280x,
    raw_report: &GpioInterruptReport,
    pin_history: &mut HashMap<u8, GpioPinHistory>,
) -> xr2280x_hid::Result<()> {
    info!("⚠️  UNSAFE APPROACH: Speculative Parsing with Validation");

    // Use unsafe parsing function with proper acknowledgment of risks
    let parsed_data = unsafe {
        match device.parse_gpio_interrupt_report(raw_report) {
            Ok(data) => data,
            Err(e) => {
                error!("🚨 Unsafe parsing failed: {}", e);
                return Err(e);
            }
        }
    };

    info!("🔍 SPECULATIVE parsed data (MAY BE INCORRECT):");
    info!(
        "   Group 0 State:    0x{:04X} (pins 0-15)",
        parsed_data.current_state_group0
    );
    info!(
        "   Group 0 Triggers: 0x{:04X}",
        parsed_data.trigger_mask_group0
    );
    info!(
        "   Group 1 State:    0x{:04X} (pins 16-31)",
        parsed_data.current_state_group1
    );
    info!(
        "   Group 1 Triggers: 0x{:04X}",
        parsed_data.trigger_mask_group1
    );

    // CRITICAL: Validate parsed data against known hardware state
    validate_parsed_interrupt_data(device, &parsed_data, pin_history)?;

    Ok(())
}

/// Validate speculative parsing results against actual hardware state
fn validate_parsed_interrupt_data(
    device: &Xr2280x,
    parsed: &ParsedGpioInterruptReport,
    pin_history: &mut HashMap<u8, GpioPinHistory>,
) -> xr2280x_hid::Result<()> {
    info!("🔬 VALIDATION: Cross-checking parsed data against hardware");

    let mut validation_errors = 0;
    let mut pin_checks = 0;

    // Check parsed pin states against direct GPIO reads
    for pin_num in 0..16 {
        if let Ok(pin) = GpioPin::new(pin_num) {
            match device.gpio_read(pin) {
                Ok(actual_level) => {
                    pin_checks += 1;

                    // Extract parsed state bit for this pin
                    let parsed_bit = (parsed.current_state_group0 >> pin_num) & 1;
                    let parsed_level = if parsed_bit == 1 {
                        GpioLevel::High
                    } else {
                        GpioLevel::Low
                    };

                    // Compare with actual hardware state
                    if parsed_level != actual_level {
                        validation_errors += 1;
                        warn!(
                            "❌ VALIDATION FAILED: Pin {} parsed as {:?} but hardware reads {:?}",
                            pin_num, parsed_level, actual_level
                        );
                    } else {
                        debug!("✅ Pin {} validation OK: {:?}", pin_num, actual_level);
                    }

                    // Update pin history
                    if let Some(history) = pin_history.get_mut(&pin_num) {
                        history.update_state(actual_level);
                    }
                }
                Err(e) => {
                    debug!("Cannot validate pin {} - read error: {}", pin_num, e);
                }
            }
        }
    }

    // Report validation results
    if validation_errors == 0 && pin_checks > 0 {
        info!(
            "✅ VALIDATION PASSED: All {} checked pins match parsed data",
            pin_checks
        );
        info!("   This suggests the parsing assumptions may be correct");
    } else if validation_errors > 0 {
        error!(
            "🚨 VALIDATION FAILED: {}/{} pins have incorrect parsed states",
            validation_errors, pin_checks
        );
        error!("   The parsing assumptions are likely INCORRECT");
        error!("   Application should NOT trust this interrupt data");
    } else {
        warn!("⚠️  VALIDATION INCONCLUSIVE: No pins could be validated");
    }

    // Additional sanity checks
    check_interrupt_data_sanity(parsed)?;

    Ok(())
}

/// Perform sanity checks on parsed interrupt data
fn check_interrupt_data_sanity(parsed: &ParsedGpioInterruptReport) -> xr2280x_hid::Result<()> {
    let mut warnings = Vec::new();

    // Check for obviously invalid patterns
    if parsed.current_state_group0 == 0xFFFF && parsed.current_state_group1 == 0xFFFF {
        warnings.push("All pins show HIGH - may indicate parsing error");
    }

    if parsed.current_state_group0 == 0x0000 && parsed.current_state_group1 == 0x0000 {
        warnings.push("All pins show LOW - may indicate parsing error");
    }

    if parsed.trigger_mask_group0 == 0xFFFF || parsed.trigger_mask_group1 == 0xFFFF {
        warnings.push("All pins show triggered - may indicate parsing error");
    }

    // Check for reasonable trigger patterns
    let total_triggers =
        parsed.trigger_mask_group0.count_ones() + parsed.trigger_mask_group1.count_ones();
    if total_triggers > 8 {
        warnings.push("Many pins triggered simultaneously - may be parsing error");
    }

    if !warnings.is_empty() {
        warn!("🔍 SANITY CHECK WARNINGS:");
        for warning in warnings {
            warn!("   • {}", warning);
        }
    } else {
        info!("✅ Basic sanity checks passed");
    }

    Ok(())
}

/// Show periodic summary of interrupt activity
fn show_interrupt_summary(pin_history: &HashMap<u8, GpioPinHistory>) {
    info!("");
    info!("📊 INTERRUPT ACTIVITY SUMMARY");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut active_pins = 0;
    let total_changes: u32 = pin_history.values().map(|h| h.state_changes).sum();

    for (pin_num, history) in pin_history.iter() {
        if history.state_changes > 0 {
            active_pins += 1;
            let last_state = history
                .last_known_state
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "Unknown".to_string());

            info!(
                "   Pin {}: {} changes, last state: {}",
                pin_num, history.state_changes, last_state
            );
        }
    }

    info!(
        "   Total: {} pins active, {} total state changes",
        active_pins, total_changes
    );
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("");
}
