//! GPIO Transaction API Example
//!
//! This example demonstrates the efficient Transaction API for batch GPIO operations.
//! The Transaction API dramatically reduces HID communication overhead by batching
//! multiple pin changes into a single set of hardware operations.

use hidapi::HidApi;
use std::time::Instant;
use xr2280x_hid::{Xr2280x, gpio::*};

fn main() -> xr2280x_hid::Result<()> {
    // Initialize device
    let hid_api = HidApi::new().expect("Failed to create HID API");
    let device = Xr2280x::device_open_first(&hid_api)?;

    println!("=== GPIO Transaction API Demo ===\n");

    // Setup pins for demonstration
    let pins = [
        GpioPin::new(0)?,
        GpioPin::new(1)?,
        GpioPin::new(2)?,
        GpioPin::new(3)?,
        GpioPin::new(4)?,
    ];

    // Configure all pins as outputs
    device.gpio_setup_outputs(
        &pins
            .iter()
            .map(|&p| (p, GpioLevel::Low))
            .collect::<Vec<_>>(),
        GpioPull::None,
    )?;

    // Demo 1: Basic Transaction Usage
    println!("📦 Demo 1: Basic Transaction Usage");
    basic_transaction_demo(&device, &pins)?;

    // Demo 2: Method Chaining
    println!("\n🔗 Demo 2: Method Chaining");
    method_chaining_demo(&device, &pins)?;

    // Demo 3: Performance Comparison
    println!("\n⚡ Demo 3: Performance Comparison");
    performance_comparison_demo(&device, &pins)?;

    // Demo 4: Bit-banging Protocol Simulation
    println!("\n🔌 Demo 4: Bit-banging Protocol Simulation");
    bitbang_protocol_demo(&device, &pins)?;

    // Demo 5: LED Matrix Control Simulation
    println!("\n💡 Demo 5: LED Matrix Control Simulation");
    led_matrix_demo(&device, &pins)?;

    println!("\n✅ All demonstrations completed successfully!");
    Ok(())
}

/// Demonstrates basic transaction usage
fn basic_transaction_demo(device: &Xr2280x, pins: &[GpioPin]) -> xr2280x_hid::Result<()> {
    println!("Creating transaction and setting multiple pins...");

    let mut transaction = device.gpio_transaction();

    // Add multiple pin changes to the transaction
    transaction.set_pin(pins[0], GpioLevel::High)?;
    transaction.set_pin(pins[1], GpioLevel::Low)?;
    transaction.set_pin(pins[2], GpioLevel::High)?;
    transaction.set_pin(pins[3], GpioLevel::Low)?;

    println!(
        "Transaction has {} pending changes",
        transaction.pending_pin_count()
    );

    // Commit all changes at once
    let hid_transactions = transaction.commit()?;
    println!(
        "✅ Applied all changes with {} HID transactions",
        hid_transactions
    );

    // Reuse the same transaction object
    transaction.set_all_low(&pins[0..4])?;
    let hid_transactions = transaction.commit()?;
    println!(
        "✅ Reset all pins with {} HID transactions",
        hid_transactions
    );

    Ok(())
}

/// Demonstrates method chaining for fluent API usage
fn method_chaining_demo(device: &Xr2280x, pins: &[GpioPin]) -> xr2280x_hid::Result<()> {
    println!("Using method chaining for fluent API...");

    let hid_transactions = device
        .gpio_transaction()
        .with_high(pins[0])?
        .with_low(pins[1])?
        .with_high(pins[2])?
        .with_low(pins[3])?
        .with_high(pins[4])?
        .commit()?;

    println!(
        "✅ Chained operations completed with {} HID transactions",
        hid_transactions
    );

    Ok(())
}

/// Compares performance between individual operations and transactions
fn performance_comparison_demo(device: &Xr2280x, pins: &[GpioPin]) -> xr2280x_hid::Result<()> {
    const ITERATIONS: usize = 10;

    println!(
        "Comparing individual operations vs transactions ({} iterations)...",
        ITERATIONS
    );

    // Test individual operations
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for (i, &pin) in pins.iter().enumerate() {
            let level = if i % 2 == 0 {
                GpioLevel::High
            } else {
                GpioLevel::Low
            };
            device.gpio_write(pin, level)?;
        }
    }
    let individual_time = start.elapsed();
    let individual_transactions = ITERATIONS * pins.len(); // Each gpio_write is 1 HID transaction

    // Test transaction operations
    let start = Instant::now();
    let mut total_hid_transactions = 0;
    for _ in 0..ITERATIONS {
        let mut transaction = device.gpio_transaction();
        for (i, &pin) in pins.iter().enumerate() {
            let level = if i % 2 == 0 {
                GpioLevel::High
            } else {
                GpioLevel::Low
            };
            transaction.set_pin(pin, level)?;
        }
        total_hid_transactions += transaction.commit()?;
    }
    let transaction_time = start.elapsed();

    println!("📊 Performance Results:");
    println!(
        "  Individual operations: {:?} ({} HID transactions)",
        individual_time, individual_transactions
    );
    println!(
        "  Transaction operations: {:?} ({} HID transactions)",
        transaction_time, total_hid_transactions
    );
    println!(
        "  🚀 Speedup: {:.1}x faster, {:.1}x fewer HID transactions",
        individual_time.as_secs_f64() / transaction_time.as_secs_f64(),
        individual_transactions as f64 / total_hid_transactions as f64
    );

    Ok(())
}

/// Simulates a bit-banging protocol (like SPI or custom serial)
fn bitbang_protocol_demo(device: &Xr2280x, pins: &[GpioPin]) -> xr2280x_hid::Result<()> {
    if pins.len() < 3 {
        println!("Need at least 3 pins for bit-banging demo");
        return Ok(());
    }

    let clk_pin = pins[0];
    let data_pin = pins[1];
    let cs_pin = pins[2];

    println!(
        "Simulating bit-banging protocol (CLK={}, DATA={}, CS={})...",
        clk_pin.number(),
        data_pin.number(),
        cs_pin.number()
    );

    // Simulate sending a byte (0xA5 = 10100101)
    let data_byte = 0xA5u8;
    println!("Sending byte: 0x{:02X} ({:08b})", data_byte, data_byte);

    let mut transaction = device.gpio_transaction();

    // Start transmission - CS low
    transaction.set_low(cs_pin)?;
    transaction.set_low(clk_pin)?;
    let hid_count = transaction.commit()?;
    println!("🔽 CS asserted with {} HID transactions", hid_count);

    // Send each bit with clock cycles
    for bit_pos in (0..8).rev() {
        let bit_value = (data_byte >> bit_pos) & 1;
        let level = if bit_value == 1 {
            GpioLevel::High
        } else {
            GpioLevel::Low
        };

        let mut transaction = device.gpio_transaction();

        // Setup data and clock low
        transaction.set_pin(data_pin, level)?;
        transaction.set_low(clk_pin)?;

        // Then clock high (data is clocked on rising edge)
        transaction.set_high(clk_pin)?;

        let hid_count = transaction.commit()?;
        println!(
            "  Bit {}: {} (with {} HID transactions)",
            bit_pos, bit_value, hid_count
        );
    }

    // End transmission - CS high, clock low
    let mut transaction = device.gpio_transaction();
    transaction.set_high(cs_pin)?;
    transaction.set_low(clk_pin)?;
    transaction.set_low(data_pin)?;
    let hid_count = transaction.commit()?;
    println!("🔼 CS deasserted with {} HID transactions", hid_count);

    println!("✅ Bit-banging protocol completed efficiently!");

    Ok(())
}

/// Simulates LED matrix control with multiple state changes
fn led_matrix_demo(device: &Xr2280x, pins: &[GpioPin]) -> xr2280x_hid::Result<()> {
    println!("Simulating LED matrix control...");

    // Create different patterns
    let patterns = [
        vec![true, false, true, false, true],    // Alternating
        vec![true, true, false, false, true],    // Groups
        vec![false, true, true, true, false],    // Center
        vec![true, false, false, false, true],   // Edges
        vec![false, false, false, false, false], // All off
    ];

    for (pattern_num, pattern) in patterns.iter().enumerate() {
        println!("📋 Pattern {}: {:?}", pattern_num + 1, pattern);

        let mut transaction = device.gpio_transaction();

        for (pin_idx, &state) in pattern.iter().enumerate() {
            if pin_idx < pins.len() {
                let level = if state {
                    GpioLevel::High
                } else {
                    GpioLevel::Low
                };
                transaction.set_pin(pins[pin_idx], level)?;
            }
        }

        let hid_count = transaction.commit()?;
        println!("  ✅ Applied pattern with {} HID transactions", hid_count);

        // In a real application, you might add a delay here
        // std::thread::sleep(std::time::Duration::from_millis(500));
    }

    println!("✅ LED matrix patterns completed!");

    Ok(())
}
