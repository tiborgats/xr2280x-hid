//! GPIO Efficient Configuration Example
//!
//! This example demonstrates the performance differences between individual GPIO
//! operations and bulk/efficient operations, along with best practices for
//! minimizing HID transaction overhead.

use hidapi::HidApi;
use std::time::Instant;
use xr2280x_hid::{
    self, Result,
    gpio::{GpioDirection, GpioGroup, GpioLevel, GpioPin, GpioPull},
};

fn main() -> Result<()> {
    env_logger::init();
    let hid_api = HidApi::new()?;

    println!("=== XR2280x GPIO Efficient Configuration Demo ===\n");

    println!("Opening first XR2280x device...");
    let device = xr2280x_hid::Xr2280x::device_open_first(&hid_api)?;
    println!("Device opened: {:?}\n", device.get_device_info());

    let gpio_count = device.get_capabilities().gpio_count;
    println!("Available GPIO count: {gpio_count}\n");

    // Use first 4 available pins for demo
    let demo_pins: Vec<GpioPin> = (0..4.min(gpio_count))
        .map(|i| GpioPin::new(i).unwrap())
        .collect();

    println!(
        "Demo pins: {:?}\n",
        demo_pins.iter().map(|p| p.number()).collect::<Vec<_>>()
    );

    // === INEFFICIENT WAY (Individual Operations) ===
    println!("⚠️  INEFFICIENT: Individual GPIO operations");
    println!("   This approach uses ~8 HID transactions per pin!");

    let start_time = Instant::now();

    for pin in &demo_pins {
        println!("   Configuring pin {} individually...", pin.number());

        // Each of these is a separate HID transaction:
        device.gpio_assign_to_edge(*pin)?; // 1 read + 1 write = 2 HID transactions
        device.gpio_set_direction(*pin, GpioDirection::Output)?; // 1 read + 1 write = 2 HID transactions
        device.gpio_set_pull(*pin, GpioPull::None)?; // 2 reads + 2 writes = 4 HID transactions
        // Total: 8 HID transactions per pin!
    }

    let inefficient_time = start_time.elapsed();
    println!(
        "   Time: {:?} ({} pins × ~8 HID transactions = ~{} total)\n",
        inefficient_time,
        demo_pins.len(),
        demo_pins.len() * 8
    );

    // === EFFICIENT WAY (Bulk Operations) ===
    println!("✅ EFFICIENT: Bulk GPIO operations");
    println!("   This approach uses ~6 HID transactions total!");

    let start_time = Instant::now();

    // Configure all pins at once with bulk operations
    device.gpio_setup_outputs(
        &demo_pins
            .iter()
            .map(|&pin| (pin, GpioLevel::Low))
            .collect::<Vec<_>>(),
        GpioPull::None,
    )?;

    let efficient_time = start_time.elapsed();
    println!("   Time: {efficient_time:?} (~6 HID transactions total)\n");

    // === PERFORMANCE COMPARISON ===
    if inefficient_time > efficient_time {
        let speedup = inefficient_time.as_nanos() as f64 / efficient_time.as_nanos() as f64;
        println!("🚀 Performance improvement: {speedup:.1}x faster!");
    }
    println!(
        "   HID transaction reduction: {}x fewer transactions\n",
        (demo_pins.len() * 8) / 6
    );

    // === DEMONSTRATION OF DIFFERENT EFFICIENT PATTERNS ===
    println!("=== Efficient Configuration Patterns ===\n");

    // Pattern 1: Single pin efficient setup
    if let Ok(pin) = GpioPin::new(0) {
        println!("1. Single pin efficient output setup:");
        let start = Instant::now();
        device.gpio_setup_output(pin, GpioLevel::High, GpioPull::Up)?;
        println!("   Time: {:?} (5 HID transactions)\n", start.elapsed());
    }

    // Pattern 2: Single pin efficient input setup
    if let Ok(pin) = GpioPin::new(1) {
        println!("2. Single pin efficient input setup:");
        let start = Instant::now();
        device.gpio_setup_input(pin, GpioPull::Down)?;
        println!("   Time: {:?} (4 HID transactions)\n", start.elapsed());
    }

    // Pattern 3: Mixed configuration bulk setup
    if demo_pins.len() >= 3 {
        println!("3. Mixed configuration (some inputs, some outputs):");
        let start = Instant::now();

        // Setup inputs
        device.gpio_setup_inputs(&demo_pins[0..2], GpioPull::Up)?;

        // Setup outputs
        device.gpio_setup_outputs(
            &[
                (demo_pins[2], GpioLevel::High),
                (demo_pins[3], GpioLevel::Low),
            ],
            GpioPull::None,
        )?;

        println!(
            "   Time: {:?} (~10 HID transactions for mixed config)\n",
            start.elapsed()
        );
    }

    // === DEMONSTRATE BULK WRITE OPERATIONS ===
    println!("4. Bulk write operations:");

    // Individual writes (inefficient)
    println!("   Individual writes (inefficient):");
    let start = Instant::now();
    for (i, pin) in demo_pins.iter().enumerate() {
        device.gpio_write(
            *pin,
            if i % 2 == 0 {
                GpioLevel::High
            } else {
                GpioLevel::Low
            },
        )?;
    }
    let individual_write_time = start.elapsed();
    println!(
        "   Time: {:?} ({} HID transactions)",
        individual_write_time,
        demo_pins.len()
    );

    // Bulk write (efficient) - for pins in same group
    println!("   Bulk write (efficient):");
    let start = Instant::now();

    // Group pins by GPIO group
    let group0_pins: Vec<_> = demo_pins.iter().filter(|p| p.group_index() == 0).collect();
    let group1_pins: Vec<_> = demo_pins.iter().filter(|p| p.group_index() == 1).collect();

    // Write to group 0 pins
    if !group0_pins.is_empty() {
        let mask = group0_pins.iter().fold(0u16, |acc, pin| acc | pin.mask());
        let values = group0_pins.iter().enumerate().fold(0u16, |acc, (i, pin)| {
            if i % 2 == 0 { acc | pin.mask() } else { acc }
        });
        device.gpio_write_masked(GpioGroup::Group0, mask, values)?;
    }

    // Write to group 1 pins
    if !group1_pins.is_empty() {
        let mask = group1_pins.iter().fold(0u16, |acc, pin| acc | pin.mask());
        let values = group1_pins.iter().enumerate().fold(0u16, |acc, (i, pin)| {
            if i % 2 == 0 { acc | pin.mask() } else { acc }
        });
        device.gpio_write_masked(GpioGroup::Group1, mask, values)?;
    }

    let bulk_write_time = start.elapsed();
    println!("   Time: {bulk_write_time:?} (2 HID transactions max)\n");

    // === PERFORMANCE RECOMMENDATIONS ===
    println!("=== Performance Recommendations ===\n");

    println!("✅ DO:");
    println!("   • Use gpio_setup_output() and gpio_setup_input() for single pins");
    println!("   • Use gpio_setup_outputs() and gpio_setup_inputs() for multiple pins");
    println!("   • Use gpio_write_masked() for updating multiple pins at once");
    println!("   • Group operations by GPIO group (0-15 vs 16-31) when possible");
    println!("   • Batch configuration changes together");

    println!("\n⚠️  AVOID:");
    println!("   • Individual gpio_set_direction() + gpio_set_pull() calls");
    println!("   • Multiple gpio_write() calls in loops");
    println!("   • Mixing individual and bulk operations unnecessarily");
    println!("   • Frequent re-configuration of the same pins");

    println!("\n📊 Transaction Count Comparison:");
    println!("   • Old way (4 pins): ~32 HID transactions");
    println!("   • New way (4 pins): ~6 HID transactions");
    println!("   • Improvement: ~5.3x fewer transactions");

    println!("\n🔧 For Advanced Users:");
    println!("   • Consider caching pin states in your application");
    println!("   • Use the masked operations directly for maximum efficiency");
    println!("   • Profile your specific use case to identify bottlenecks");

    println!("\n✨ Demo completed successfully!");
    Ok(())
}
