//! GPIO Write Reliability Demo
//!
//! This example demonstrates the new GPIO write reliability features that address
//! the critical intermittent failure issue where gpio_write() returns Ok(()) but
//! the hardware GPIO pin doesn't actually change state.
//!
//! ## Problem Being Solved
//!
//! The XR2280x HID devices have a timing-sensitive GPIO controller that can
//! intermittently fail to process write commands, leading to:
//! - Silent failures (function returns Ok but pin state unchanged)
//! - Unreliable hardware control
//! - Difficult-to-debug issues requiring oscilloscope verification
//!
//! ## Solution Features
//!
//! 1. **Write Verification**: Read back pin state to confirm writes
//! 2. **Retry Logic**: Automatically retry failed operations
//! 3. **Configurable Timeouts**: Prevent hanging on stuck hardware
//! 4. **Performance Modes**: Choose between speed and reliability
//!
//! ## Usage Patterns
//!
//! - **Critical Control**: Use verified writes for power control, safety systems
//! - **High-Speed Applications**: Use fast writes for bit-banging, PWM generation
//! - **Mixed Workloads**: Configure per-operation as needed

use hidapi::HidApi;
use std::time::{Duration, Instant};
use xr2280x_hid::{
    Result, Xr2280x,
    gpio::{GpioLevel, GpioPin, GpioPull, GpioWriteConfig},
};

fn main() -> Result<()> {
    env_logger::init();

    let hid_api = HidApi::new()?;
    let device = Xr2280x::device_open_first(&hid_api)?;

    println!("=== GPIO Write Reliability Demo ===\n");

    // Use pins that are commonly available across device variants
    let test_pins = [
        GpioPin::new(0)?, // E0
        GpioPin::new(1)?, // E1
        GpioPin::new(2)?, // E2
    ];

    // Setup pins as outputs
    for &pin in &test_pins {
        if pin.number() >= device.get_capabilities().gpio_count {
            println!(
                "⚠️  Pin {} not supported on this device, skipping",
                pin.number()
            );
            continue;
        }

        device.gpio_assign_to_edge(pin)?;
        device.gpio_setup_output(pin, GpioLevel::Low, GpioPull::None)?;
    }

    // Demo 1: Default (Fast) Mode
    println!("📦 Demo 1: Default Fast Mode (No Verification)");
    demo_fast_mode(&device, &test_pins)?;

    // Demo 2: Verified Mode
    println!("\n🔍 Demo 2: Verified Write Mode");
    demo_verified_mode(&device, &test_pins)?;

    // Demo 3: Custom Configuration
    println!("\n⚙️  Demo 3: Custom Reliability Configuration");
    demo_custom_config(&device, &test_pins)?;

    // Demo 4: Performance Comparison
    println!("\n⚡ Demo 4: Performance Comparison");
    demo_performance_comparison(&device, &test_pins)?;

    // Demo 5: Error Handling
    println!("\n🚨 Demo 5: Error Handling and Recovery");
    demo_error_handling(&device, &test_pins)?;

    // Demo 6: Real-World Scenarios
    println!("\n🏭 Demo 6: Real-World Application Patterns");
    demo_real_world_patterns(&device, &test_pins)?;

    println!("\n✅ All demonstrations completed successfully!");
    println!("\n💡 Key Takeaways:");
    println!("   • Use verified writes for critical control operations");
    println!("   • Use fast writes for high-speed bit-banging");
    println!("   • Configure retry logic based on your reliability requirements");
    println!("   • Monitor and handle verification failures appropriately");

    Ok(())
}

/// Demonstrates default fast mode behavior
fn demo_fast_mode(device: &Xr2280x, pins: &[GpioPin]) -> Result<()> {
    println!("Using default fast mode (no verification)...");

    // This is the traditional behavior - fast but potentially unreliable
    for &pin in pins {
        if pin.number() >= device.get_capabilities().gpio_count {
            continue;
        }

        device.gpio_write(pin, GpioLevel::High)?;
        device.gpio_write(pin, GpioLevel::Low)?;

        println!("  ✅ Pin {} written (fast mode)", pin.number());
    }

    println!("⚠️  Note: Fast mode may have intermittent failures (20-30% on some pins)");
    Ok(())
}

/// Demonstrates verified write mode with automatic retry
fn demo_verified_mode(device: &Xr2280x, pins: &[GpioPin]) -> Result<()> {
    println!("Using verified write mode with automatic verification...");

    for &pin in pins {
        if pin.number() >= device.get_capabilities().gpio_count {
            continue;
        }

        println!("  📍 Testing pin {} with verification...", pin.number());

        // Use the verified write method
        match device.gpio_write_verified(pin, GpioLevel::High) {
            Ok(()) => {
                println!("    ✅ HIGH write verified successfully");

                // Verify it actually worked by reading back
                let level = device.gpio_read(pin)?;
                println!("    📖 Readback confirms: {level:?}");
            }
            Err(e) => {
                println!("    ❌ HIGH write failed: {e}");
            }
        }

        match device.gpio_write_verified(pin, GpioLevel::Low) {
            Ok(()) => {
                println!("    ✅ LOW write verified successfully");
                let level = device.gpio_read(pin)?;
                println!("    📖 Readback confirms: {level:?}");
            }
            Err(e) => {
                println!("    ❌ LOW write failed: {e}");
            }
        }
    }

    Ok(())
}

/// Demonstrates custom configuration options
fn demo_custom_config(device: &Xr2280x, pins: &[GpioPin]) -> Result<()> {
    println!("Configuring custom reliability settings...");

    // Create a custom configuration for maximum reliability
    let max_reliability_config = GpioWriteConfig {
        verify_writes: true,
        retry_attempts: 5,
        retry_delay: Duration::from_millis(50),
        operation_timeout: Duration::from_millis(2000),
    };

    // Apply the configuration
    device.gpio_set_write_config(max_reliability_config)?;
    println!("  ⚙️  Applied maximum reliability configuration");

    // Test with the new configuration
    for &pin in pins {
        if pin.number() >= device.get_capabilities().gpio_count {
            continue;
        }

        println!("  🔄 Testing pin {} with max reliability...", pin.number());

        let start = Instant::now();
        match device.gpio_write(pin, GpioLevel::High) {
            Ok(()) => {
                let duration = start.elapsed();
                println!("    ✅ Write succeeded in {duration:?}");
            }
            Err(e) => {
                println!("    ❌ Write failed even with max reliability: {e}");
            }
        }

        // Reset to low
        device.gpio_write(pin, GpioLevel::Low)?;
    }

    // Reset to default configuration
    device.gpio_set_write_config(GpioWriteConfig::default())?;
    println!("  🔄 Reset to default configuration");

    Ok(())
}

/// Compares performance between different modes
fn demo_performance_comparison(device: &Xr2280x, pins: &[GpioPin]) -> Result<()> {
    const ITERATIONS: usize = 10;

    println!("Comparing performance across {ITERATIONS} iterations...");

    if pins.is_empty() || pins[0].number() >= device.get_capabilities().gpio_count {
        println!("  ⚠️  No available pins for performance testing");
        return Ok(());
    }

    let test_pin = pins[0];

    // Test 1: Fast mode performance
    println!("  🏃 Testing fast mode performance...");
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        device.gpio_write_fast(test_pin, GpioLevel::High)?;
        device.gpio_write_fast(test_pin, GpioLevel::Low)?;
    }
    let fast_duration = start.elapsed();

    // Test 2: Verified mode performance
    println!("  🔍 Testing verified mode performance...");
    device.gpio_set_write_verification(true)?;

    let start = Instant::now();
    let mut success_count = 0;
    for _ in 0..ITERATIONS {
        if device.gpio_write(test_pin, GpioLevel::High).is_ok() {
            success_count += 1;
        }
        if device.gpio_write(test_pin, GpioLevel::Low).is_ok() {
            success_count += 1;
        }
    }
    let verified_duration = start.elapsed();

    // Reset to default
    device.gpio_set_write_verification(false)?;

    // Results
    println!("  📊 Performance Results:");
    println!(
        "     Fast mode:     {:?} total ({:?} per operation)",
        fast_duration,
        fast_duration / (ITERATIONS * 2) as u32
    );
    println!(
        "     Verified mode: {:?} total ({:?} per operation)",
        verified_duration,
        verified_duration / (ITERATIONS * 2) as u32
    );
    println!(
        "     Slowdown:      {:.1}x",
        verified_duration.as_secs_f64() / fast_duration.as_secs_f64()
    );
    println!(
        "     Success rate:  {}/{} operations",
        success_count,
        ITERATIONS * 2
    );

    Ok(())
}

/// Demonstrates error handling for write verification failures
fn demo_error_handling(device: &Xr2280x, pins: &[GpioPin]) -> Result<()> {
    println!("Demonstrating error handling patterns...");

    if pins.is_empty() || pins[0].number() >= device.get_capabilities().gpio_count {
        println!("  ⚠️  No available pins for error handling demo");
        return Ok(());
    }

    let test_pin = pins[0];

    // Configure for aggressive verification that might fail
    let aggressive_config = GpioWriteConfig {
        verify_writes: true,
        retry_attempts: 2,
        retry_delay: Duration::from_millis(10),
        operation_timeout: Duration::from_millis(100),
    };

    device.gpio_set_write_config(aggressive_config)?;

    println!("  🎯 Attempting write with aggressive timeout...");

    match device.gpio_write(test_pin, GpioLevel::High) {
        Ok(()) => {
            println!("    ✅ Write succeeded despite aggressive settings");
        }
        Err(e) => {
            println!("    ❌ Write failed as expected: {e}");

            // Demonstrate recovery strategies
            println!("    🔄 Attempting recovery strategies...");

            // Strategy 1: Retry with more lenient settings
            let recovery_config = GpioWriteConfig::reliable();
            device.gpio_set_write_config(recovery_config)?;

            match device.gpio_write(test_pin, GpioLevel::High) {
                Ok(()) => println!("    ✅ Recovery successful with reliable config"),
                Err(e) => {
                    println!("    ❌ Recovery failed: {e}");

                    // Strategy 2: Fall back to fast mode
                    println!("    🏃 Falling back to fast mode...");
                    device.gpio_write_fast(test_pin, GpioLevel::High)?;

                    // Manual verification
                    let actual = device.gpio_read(test_pin)?;
                    println!("    📖 Manual verification: {actual:?}");
                }
            }
        }
    }

    // Reset to default
    device.gpio_set_write_config(GpioWriteConfig::default())?;

    Ok(())
}

/// Demonstrates real-world application patterns
fn demo_real_world_patterns(device: &Xr2280x, pins: &[GpioPin]) -> Result<()> {
    println!("Real-world application patterns...");

    if pins.len() < 2 {
        println!("  ⚠️  Need at least 2 pins for real-world demos");
        return Ok(());
    }

    // Filter available pins
    let available_pins: Vec<GpioPin> = pins
        .iter()
        .filter(|&&pin| pin.number() < device.get_capabilities().gpio_count)
        .copied()
        .collect();

    if available_pins.len() < 2 {
        println!("  ⚠️  Insufficient available pins for real-world demos");
        return Ok(());
    }

    // Pattern 1: Critical Power Control
    println!("  🔌 Pattern 1: Critical Power Control");
    simulate_power_control(device, available_pins[0])?;

    // Pattern 2: High-Speed Bit-Banging
    println!("  ⚡ Pattern 2: High-Speed Bit-Banging");
    simulate_bit_banging(device, &available_pins[0..2])?;

    // Pattern 3: Mixed Reliability Requirements
    println!("  🎛️  Pattern 3: Mixed Reliability Requirements");
    simulate_mixed_control(device, &available_pins)?;

    Ok(())
}

/// Simulates critical power control where reliability is paramount
fn simulate_power_control(device: &Xr2280x, power_pin: GpioPin) -> Result<()> {
    println!("    Simulating power control sequence...");

    // Use maximum reliability for power control
    let power_config = GpioWriteConfig {
        verify_writes: true,
        retry_attempts: 5,
        retry_delay: Duration::from_millis(100),
        operation_timeout: Duration::from_millis(1000),
    };

    device.gpio_set_write_config(power_config)?;

    // Power-on sequence
    println!("    🔋 Powering on critical system...");
    match device.gpio_write(power_pin, GpioLevel::High) {
        Ok(()) => {
            println!("    ✅ Power enabled and verified");

            // Simulate some work
            std::thread::sleep(Duration::from_millis(100));

            // Power-off sequence
            println!("    🔌 Powering off critical system...");
            device.gpio_write(power_pin, GpioLevel::Low)?;
            println!("    ✅ Power disabled and verified");
        }
        Err(e) => {
            println!("    ❌ CRITICAL: Power control failed: {e}");
            println!("    🚨 System may be in unsafe state!");
        }
    }

    Ok(())
}

/// Simulates high-speed bit-banging where performance matters
fn simulate_bit_banging(device: &Xr2280x, pins: &[GpioPin]) -> Result<()> {
    if pins.len() < 2 {
        return Ok(());
    }

    let clk_pin = pins[0];
    let data_pin = pins[1];

    println!("    Simulating high-speed bit-banging protocol...");

    // Use fast mode for bit-banging
    device.gpio_set_write_config(GpioWriteConfig::fast())?;

    let data_byte = 0xA5u8; // 10100101
    println!("    📡 Sending byte: 0x{data_byte:02X}");

    let start = Instant::now();

    // Send each bit
    for bit_pos in (0..8).rev() {
        let bit_value = (data_byte >> bit_pos) & 1;
        let level = if bit_value == 1 {
            GpioLevel::High
        } else {
            GpioLevel::Low
        };

        // Setup data
        device.gpio_write_fast(data_pin, level)?;

        // Clock pulse
        device.gpio_write_fast(clk_pin, GpioLevel::High)?;
        device.gpio_write_fast(clk_pin, GpioLevel::Low)?;
    }

    let duration = start.elapsed();
    println!("    ⚡ Bit-banging completed in {duration:?}");

    Ok(())
}

/// Simulates mixed control requirements
fn simulate_mixed_control(device: &Xr2280x, pins: &[GpioPin]) -> Result<()> {
    if pins.is_empty() {
        return Ok(());
    }

    println!("    Simulating mixed reliability requirements...");

    // Different pins have different reliability requirements
    for (i, &pin) in pins.iter().enumerate() {
        let reliability_level = match i % 3 {
            0 => "Critical",
            1 => "Standard",
            _ => "Fast",
        };

        println!(
            "    📌 Pin {} - {} reliability",
            pin.number(),
            reliability_level
        );

        let config = match i % 3 {
            0 => GpioWriteConfig::reliable(), // Critical
            1 => GpioWriteConfig {
                // Standard
                verify_writes: true,
                retry_attempts: 1,
                retry_delay: Duration::from_millis(20),
                operation_timeout: Duration::from_millis(500),
            },
            _ => GpioWriteConfig::fast(), // Fast
        };

        device.gpio_set_write_config(config)?;

        // Perform operation
        let start = Instant::now();
        match device.gpio_write(pin, GpioLevel::High) {
            Ok(()) => {
                let duration = start.elapsed();
                println!("      ✅ Operation completed in {duration:?}");
            }
            Err(e) => {
                println!("      ❌ Operation failed: {e}");
            }
        }

        // Reset pin
        device.gpio_write_fast(pin, GpioLevel::Low)?;
    }

    Ok(())
}
