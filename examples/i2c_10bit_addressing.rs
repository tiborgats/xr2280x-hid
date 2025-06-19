//! 10-bit I2C Addressing Example
//!
//! This example demonstrates how to use 10-bit I2C addressing with the XR2280x device.
//! 10-bit addressing allows access to 1024 different I2C addresses (0x000-0x3FF)
//! compared to 128 addresses (0x00-0x7F) with standard 7-bit addressing.
//!
//! ## When to Use 10-bit Addressing
//!
//! 10-bit I2C addressing is useful when:
//! - You have many I2C devices and need more address space
//! - Working with devices that only support 10-bit addressing
//! - Building systems with high device density
//!
//! ## Important Notes
//!
//! - 10-bit I2C devices are relatively rare compared to 7-bit devices
//! - Not all I2C devices support 10-bit addressing
//! - 10-bit transactions are slightly slower due to the two-phase protocol
//! - Both master and slave must support 10-bit addressing

use hidapi::HidApi;
use xr2280x_hid::{self, Result, i2c::I2cAddress};

fn main() -> Result<()> {
    env_logger::init();
    let hid_api = HidApi::new()?;

    println!("=== XR2280x 10-bit I2C Addressing Example ===\n");

    println!("Opening first XR2280x device...");
    let device = xr2280x_hid::Xr2280x::device_open_first(&hid_api)?;
    println!("Device opened: {:?}\n", device.get_device_info());

    // Set I2C speed for testing
    device.i2c_set_speed_khz(100)?;
    println!("Set I2C speed to 100kHz for compatibility\n");

    // Demonstrate address creation and validation
    demonstrate_address_creation()?;

    // Demonstrate I2C operations with 10-bit addresses
    demonstrate_10bit_operations(&device)?;

    // Compare 7-bit vs 10-bit addressing
    demonstrate_address_comparison(&device)?;

    // Demonstrate error handling
    demonstrate_error_handling(&device)?;

    println!("✨ 10-bit I2C addressing example completed successfully!");
    Ok(())
}

fn demonstrate_address_creation() -> Result<()> {
    println!("1. 10-bit I2C Address Creation and Validation");
    println!("{}", "=".repeat(55));

    // Valid 10-bit addresses
    println!("✅ Valid 10-bit addresses:");
    let valid_addresses = [
        0x000, // Minimum address
        0x050, // Typical device address (overlaps with 7-bit range)
        0x150, // First exclusive 10-bit address range
        0x2A5, // Mixed bit pattern
        0x3FF, // Maximum address
    ];

    for addr in valid_addresses {
        match I2cAddress::new_10bit(addr) {
            Ok(i2c_addr) => println!("  • 0x{:03X} → {}", addr, i2c_addr),
            Err(e) => println!("  • 0x{:03X} → ERROR: {}", addr, e),
        }
    }

    // Invalid 10-bit addresses
    println!("\n❌ Invalid 10-bit addresses:");
    let invalid_addresses = [0x400, 0x500, 0x800, 0xFFF];

    for addr in invalid_addresses {
        match I2cAddress::new_10bit(addr) {
            Ok(i2c_addr) => println!("  • 0x{:03X} → {} (should be invalid!)", addr, i2c_addr),
            Err(e) => println!("  • 0x{:03X} → Correctly rejected: {}", addr, e),
        }
    }

    // Address range analysis
    println!("\n📊 Address Range Analysis:");
    println!("  • 7-bit range: 0x00-0x7F (128 addresses)");
    println!("  • 10-bit range: 0x000-0x3FF (1024 addresses)");
    println!("  • Overlap: 0x00-0x7F can be addressed as either 7-bit or 10-bit");
    println!("  • Exclusive 10-bit: 0x80-0x3FF (896 additional addresses)");

    println!();
    Ok(())
}

fn demonstrate_10bit_operations(device: &xr2280x_hid::Xr2280x) -> Result<()> {
    println!("2. 10-bit I2C Operations");
    println!("{}", "=".repeat(55));

    // Test addresses that are likely to be unused (will get NACK, which is expected)
    let test_addresses = [
        0x150, // First exclusive 10-bit range
        0x2A5, // Mixed bit pattern
        0x380, // High address
    ];

    for &addr in &test_addresses {
        println!("Testing 10-bit address 0x{:03X}:", addr);

        // Write operation example
        println!("  📤 Write operation:");
        let write_data = [0x00, 0x01, 0x42, 0x55]; // Example data
        match device.i2c_write_10bit(addr, &write_data) {
            Ok(_) => {
                println!("    ✅ Write successful (device responded)");
                println!(
                    "    📝 Wrote {} bytes: {:02X?}",
                    write_data.len(),
                    write_data
                );
            }
            Err(xr2280x_hid::Error::I2cNack { address }) => {
                println!(
                    "    🔍 No device at address {} (NACK - normal for unused addresses)",
                    address
                );
            }
            Err(e) => {
                println!("    ❌ Error: {}", e);
            }
        }

        // Read operation example
        println!("  📥 Read operation:");
        let mut read_buffer = [0u8; 4];
        match device.i2c_read_10bit(addr, &mut read_buffer) {
            Ok(_) => {
                println!("    ✅ Read successful: {:02X?}", read_buffer);
            }
            Err(xr2280x_hid::Error::I2cNack { address }) => {
                println!(
                    "    🔍 No device at address {} (NACK - normal for unused addresses)",
                    address
                );
            }
            Err(e) => {
                println!("    ❌ Error: {}", e);
            }
        }

        // Write-then-read operation example
        println!("  🔄 Write-then-read operation:");
        let register_addr = [0x00]; // Read from register 0x00
        let mut read_data = [0u8; 2];
        match device.i2c_write_read_10bit(addr, &register_addr, &mut read_data) {
            Ok(_) => {
                println!("    ✅ Write-then-read successful: {:02X?}", read_data);
            }
            Err(xr2280x_hid::Error::I2cNack { address }) => {
                println!(
                    "    🔍 No device at address {} (NACK - normal for unused addresses)",
                    address
                );
            }
            Err(e) => {
                println!("    ❌ Error: {}", e);
            }
        }

        println!();
    }

    Ok(())
}

fn demonstrate_address_comparison(device: &xr2280x_hid::Xr2280x) -> Result<()> {
    println!("3. 7-bit vs 10-bit Address Comparison");
    println!("{}", "=".repeat(55));

    // Test the same logical address using both 7-bit and 10-bit addressing
    let test_addr = 0x50; // Common EEPROM address

    println!("Comparing address 0x50 using both addressing modes:");
    println!("(Both should behave identically for devices that support both)");

    // 7-bit addressing
    println!("\n  📍 7-bit addressing (0x50):");
    let write_data = [0x00, 0x55, 0xAA];
    match device.i2c_write_7bit(test_addr as u8, &write_data) {
        Ok(_) => println!("    ✅ 7-bit write successful"),
        Err(xr2280x_hid::Error::I2cNack { address }) => {
            println!("    🔍 7-bit: No device at address {} (NACK)", address);
        }
        Err(e) => println!("    ❌ 7-bit error: {}", e),
    }

    // 10-bit addressing of the same address
    println!("  📍 10-bit addressing (0x050):");
    match device.i2c_write_10bit(test_addr, &write_data) {
        Ok(_) => println!("    ✅ 10-bit write successful"),
        Err(xr2280x_hid::Error::I2cNack { address }) => {
            println!("    🔍 10-bit: No device at address {} (NACK)", address);
        }
        Err(e) => println!("    ❌ 10-bit error: {}", e),
    }

    // Demonstrate exclusive 10-bit address
    println!("\n  📍 Exclusive 10-bit address (0x150):");
    println!("    (This address cannot be accessed with 7-bit addressing)");
    match device.i2c_write_10bit(0x150, &write_data) {
        Ok(_) => println!("    ✅ Exclusive 10-bit write successful"),
        Err(xr2280x_hid::Error::I2cNack { address }) => {
            println!("    🔍 No device at address {} (NACK)", address);
        }
        Err(e) => println!("    ❌ Error: {}", e),
    }

    println!();
    Ok(())
}

fn demonstrate_error_handling(device: &xr2280x_hid::Xr2280x) -> Result<()> {
    println!("4. Error Handling and Best Practices");
    println!("{}", "=".repeat(55));

    // Demonstrate timeout handling
    println!("🕐 Custom timeout example:");
    match device.i2c_read_10bit_with_timeout(0x200, &mut [0u8; 4], 50) {
        Ok(_) => println!("    ✅ Read with custom timeout successful"),
        Err(xr2280x_hid::Error::I2cNack { address }) => {
            println!("    🔍 No device at address {} (NACK)", address);
        }
        Err(xr2280x_hid::Error::I2cTimeout { address }) => {
            println!("    ⏰ Timeout reading from address {}", address);
        }
        Err(e) => println!("    ❌ Error: {}", e),
    }

    // Demonstrate invalid address handling
    println!("\n❌ Invalid address handling:");
    match I2cAddress::new_10bit(0x500) {
        Ok(_) => println!("    ❌ Should have failed validation!"),
        Err(e) => println!("    ✅ Address validation works: {}", e),
    }

    // Best practices
    println!("\n📋 Best Practices for 10-bit I2C:");
    println!("  1. Always validate addresses before use:");
    println!("     let addr = I2cAddress::new_10bit(0x150)?;");
    println!("  2. Handle NACK errors gracefully (device not present)");
    println!("  3. Use appropriate timeouts for your devices");
    println!("  4. Consider using 7-bit addressing when possible (more compatible)");
    println!("  5. Verify your I2C devices actually support 10-bit addressing");
    println!("  6. Use write-then-read for register-based devices");

    println!("\n💡 Real-world 10-bit I2C usage examples:");
    println!("  • High-density sensor arrays");
    println!("  • Multi-node communication systems");
    println!("  • Industrial automation with many sensors");
    println!("  • Memory devices requiring extended addressing");

    println!();
    Ok(())
}

/// Example of a real-world 10-bit I2C device interface
#[allow(dead_code)]
struct Example10BitSensor {
    device: xr2280x_hid::Xr2280x,
    address: u16,
}

#[allow(dead_code)]
impl Example10BitSensor {
    /// Create a new sensor interface
    pub fn new(device: xr2280x_hid::Xr2280x, address: u16) -> Result<Self> {
        // Validate the address
        I2cAddress::new_10bit(address)?;

        Ok(Self { device, address })
    }

    /// Read sensor data from a specific register
    pub fn read_register(&self, register: u8) -> Result<u16> {
        let mut data = [0u8; 2];
        self.device
            .i2c_write_read_10bit(self.address, &[register], &mut data)?;
        Ok(u16::from_be_bytes(data))
    }

    /// Write configuration to a register
    pub fn write_register(&self, register: u8, value: u16) -> Result<()> {
        let data = [register, (value >> 8) as u8, (value & 0xFF) as u8];
        self.device.i2c_write_10bit(self.address, &data)?;
        Ok(())
    }

    /// Check if the sensor is present and responding
    pub fn is_present(&self) -> bool {
        // Try to read a status register (register 0x00 is common)
        self.read_register(0x00).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_validation() {
        // Valid addresses
        assert!(I2cAddress::new_10bit(0x000).is_ok());
        assert!(I2cAddress::new_10bit(0x3FF).is_ok());
        assert!(I2cAddress::new_10bit(0x150).is_ok());

        // Invalid addresses
        assert!(I2cAddress::new_10bit(0x400).is_err());
        assert!(I2cAddress::new_10bit(0x1000).is_err());
    }

    #[test]
    fn test_address_ranges() {
        // All 7-bit addresses should be valid as 10-bit
        for addr in 0x00..=0x7F {
            assert!(I2cAddress::new_10bit(addr).is_ok());
        }

        // Extended 10-bit range
        for addr in 0x80..=0x3FF {
            assert!(I2cAddress::new_10bit(addr).is_ok());
        }
    }
}
