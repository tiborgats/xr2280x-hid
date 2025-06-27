//! Demonstrates the improved, consistent Pin API for GPIO interrupt handling.
//!
//! This example showcases the new `parse_gpio_interrupt_pins()` function that provides
//! a more ergonomic and type-safe API by returning individual `(GpioPin, GpioEdge)`
//! combinations instead of raw group masks.
//!
//! ## Key Improvements
//!
//! 1. **Type Safety**: Returns typed `GpioPin` objects instead of raw `u8` values
//! 2. **Consistency**: Entire GPIO API now uses `GpioPin` throughout
//! 3. **Ergonomics**: No manual conversion from `u8` to `GpioPin` required
//! 4. **Error Handling**: Invalid pin numbers are caught at the API boundary
//!
//! ## Hardware Requirements
//!
//! - XR22800/1/2/4 device with EDGE interface
//! - GPIO pins configured as inputs with interrupt generation capability
//! - External signal source to trigger interrupts (buttons, switches, etc.)

use std::collections::HashMap;
use std::time::{Duration, Instant};

use hidapi::HidApi;
use log::{error, info, warn};

use xr2280x_hid::{Error, GpioEdge, GpioLevel, GpioPin, GpioPull, Result, Xr2280x};

fn main() -> Result<()> {
    env_logger::init();

    let hid_api = HidApi::new().map_err(Error::Hid)?;

    info!("🔍 Searching for XR2280x devices...");
    let device = Xr2280x::device_open_first(&hid_api)?;
    info!("✅ Successfully opened XR2280x device");

    // Check if device has EDGE interface for GPIO interrupts
    let capabilities = device.get_capabilities();
    if capabilities.gpio_count == 0 {
        error!("❌ Device does not support GPIO operations");
        return Err(Error::UnsupportedFeature(
            "GPIO functionality not available".to_string(),
        ));
    }

    info!(
        "📊 Device capabilities: {} GPIO pins available",
        capabilities.gpio_count
    );

    // Set up GPIO pins for interrupt monitoring
    setup_gpio_interrupts(&device)?;

    // Demonstrate the new consistent Pin API
    demonstrate_consistent_pin_api(&device)?;

    Ok(())
}

fn setup_gpio_interrupts(device: &Xr2280x) -> Result<()> {
    info!("🔧 Configuring GPIO pins for interrupt generation...");

    // Configure first few pins as inputs with interrupts
    let test_pins = [0, 1, 2, 3];

    for &pin_num in &test_pins {
        if let Ok(pin) = GpioPin::new(pin_num) {
            // Assign pin to EDGE interface
            device.gpio_assign_to_edge(pin)?;

            // Configure as input with pull-up
            device.gpio_setup_input(pin, GpioPull::Up)?;

            // Enable interrupts on both edges
            device.gpio_configure_interrupt(pin, true, true, true)?;

            info!("✅ Configured GPIO pin {pin_num} for interrupts");
        } else {
            warn!("⚠️  Failed to create GpioPin for pin {pin_num}");
        }
    }

    info!("🎯 GPIO interrupt setup complete");
    Ok(())
}

fn demonstrate_consistent_pin_api(device: &Xr2280x) -> Result<()> {
    info!("🚀 Demonstrating improved Pin API consistency...");

    let mut pin_event_counts: HashMap<u8, usize> = HashMap::new();
    let start_time = Instant::now();
    let monitoring_duration = Duration::from_secs(10);

    info!(
        "👂 Monitoring GPIO interrupts for {} seconds...",
        monitoring_duration.as_secs()
    );
    info!("💡 Trigger interrupts by connecting/disconnecting pins to generate events");

    while start_time.elapsed() < monitoring_duration {
        // Read interrupt report with timeout
        match device.read_gpio_interrupt_report(Some(1000)) {
            Ok(report) => {
                // OLD WAY (commented out): Would return raw group masks
                // let parsed = unsafe { device.parse_gpio_interrupt_report(&report)? };
                // // User would need to manually parse masks and convert u8 to GpioPin

                // NEW WAY: Get individual pin/edge combinations with type safety
                match device.parse_gpio_interrupt_pins(&report) {
                    Ok(pin_events) => {
                        if !pin_events.is_empty() {
                            info!("🎉 Received {} GPIO interrupt events:", pin_events.len());

                            for (pin, edge) in pin_events {
                                // COUNT: Track events per pin
                                *pin_event_counts.entry(pin.number()).or_insert(0) += 1;

                                info!("  📌 Pin {} triggered on {:?} edge", pin.number(), edge);

                                // CONSISTENCY: Can directly use typed pin with other GPIO functions
                                // (no conversion from u8 to GpioPin required!)
                                match device.gpio_read(pin) {
                                    Ok(level) => {
                                        info!("     Current level: {level:?}");

                                        // Demonstrate edge validation
                                        let edge_matches = matches!(
                                            (edge, level),
                                            (GpioEdge::Rising, GpioLevel::High)
                                                | (GpioEdge::Falling, GpioLevel::Low)
                                                | (GpioEdge::Both, _)
                                        );

                                        if edge_matches {
                                            info!(
                                                "     ✅ Edge detection consistent with current level"
                                            );
                                        } else {
                                            warn!(
                                                "     ⚠️  Edge/level mismatch - possible race condition"
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        warn!("     ❌ Failed to read pin {}: {}", pin.number(), e);
                                    }
                                }

                                // TYPE SAFETY: The pin is guaranteed to be valid (0-31)
                                // because GpioPin::new() was called during parsing
                                assert!(pin.number() <= 31);

                                // ERGONOMICS: Can use pin directly with other operations
                                demonstrate_pin_operations(device, pin)?;
                            }
                        }
                    }
                    Err(e) => {
                        error!("❌ Failed to parse interrupt pins: {e}");
                    }
                }
            }
            Err(Error::Timeout) => {
                // Normal timeout, continue monitoring
                continue;
            }
            Err(e) => {
                error!("❌ Failed to read interrupt report: {e}");
                break;
            }
        }
    }

    // Display summary
    display_monitoring_summary(&pin_event_counts);

    Ok(())
}

fn demonstrate_pin_operations(device: &Xr2280x, pin: GpioPin) -> Result<()> {
    // Example: Toggle pin output briefly (if supported)
    if let Ok(current_direction) = device.gpio_get_direction(pin) {
        info!(
            "     🔧 Pin {} current direction: {:?}",
            pin.number(),
            current_direction
        );

        // Note: In a real application, you'd be more careful about changing
        // pin directions, especially if they're configured for interrupts
    }

    // Example: Check pull resistor configuration
    if let Ok(pull_config) = device.gpio_get_pull(pin) {
        info!(
            "     🔌 Pin {} pull configuration: {:?}",
            pin.number(),
            pull_config
        );
    }

    Ok(())
}

fn display_monitoring_summary(pin_event_counts: &HashMap<u8, usize>) {
    info!("📈 GPIO Interrupt Monitoring Summary:");

    if pin_event_counts.is_empty() {
        info!("   No GPIO interrupts detected during monitoring period");
        info!("   💡 Try connecting/disconnecting pins to ground to generate events");
    } else {
        info!("   Events detected on {} pins:", pin_event_counts.len());
        for (&pin_num, &count) in pin_event_counts {
            info!("     Pin {pin_num}: {count} events");
        }

        let total_events: usize = pin_event_counts.values().sum();
        info!("   Total events: {total_events}");
    }

    info!("✨ API Improvements Demonstrated:");
    info!("   ✅ Type-safe GpioPin objects throughout");
    info!("   ✅ No manual u8 → GpioPin conversion required");
    info!("   ✅ Consistent API across all GPIO functions");
    info!("   ✅ Error handling at API boundary");
    info!("   ✅ Direct pin object reuse with other operations");
}
