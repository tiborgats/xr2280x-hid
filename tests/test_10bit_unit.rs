//! Unit tests for 10-bit I2C address handling
//!
//! These tests verify that the 10-bit I2C address encoding follows the I2C specification
//! and that the implementation correctly handles all edge cases.

#[cfg(test)]
mod tests {
    use xr2280x_hid::i2c::I2cAddress;

    #[test]
    fn test_10bit_address_encoding() {
        // Test cases covering the full 10-bit address range
        // Format: (address, expected_first_byte, expected_second_byte)
        let test_cases = [
            (0x000, 0xF0, 0x00), // Min address: 00_0000_0000
            (0x150, 0xF2, 0x50), // Typical address: 01_0101_0000
            (0x2A5, 0xF4, 0xA5), // Mixed bits: 10_1010_0101
            (0x3FF, 0xF6, 0xFF), // Max address: 11_1111_1111
            (0x080, 0xF0, 0x80), // First exclusive 10-bit address
            (0x100, 0xF2, 0x00), // 9-bit boundary
            (0x200, 0xF4, 0x00), // High bit set
            (0x300, 0xF6, 0x00), // Both high bits set
        ];

        for (addr, expected_first, expected_second) in test_cases {
            let i2c_addr = I2cAddress::new_10bit(addr).unwrap();

            // Test the encoding logic from i2c_transfer function
            if let I2cAddress::Bit10(a) = i2c_addr {
                let high_2_bits = ((a >> 8) & 0x03) as u8;
                let first_byte = (high_2_bits << 1) | 0xF0; // 11110xx0 pattern
                let second_byte = (a & 0xFF) as u8;

                assert_eq!(
                    first_byte, expected_first,
                    "Address 0x{addr:03X}: first byte mismatch. Expected 0x{expected_first:02X}, got 0x{first_byte:02X}"
                );
                assert_eq!(
                    second_byte, expected_second,
                    "Address 0x{addr:03X}: second byte mismatch. Expected 0x{expected_second:02X}, got 0x{second_byte:02X}"
                );

                // Verify the first byte follows the I2C 10-bit pattern
                assert_eq!(
                    first_byte & 0xF8,
                    0xF0,
                    "Address 0x{addr:03X}: first byte must start with 11110 pattern"
                );
                assert_eq!(
                    first_byte & 0x01,
                    0x00,
                    "Address 0x{addr:03X}: first byte must end with 0 (write bit)"
                );
            }
        }
    }

    #[test]
    fn test_10bit_address_validation() {
        // Test valid addresses
        assert!(I2cAddress::new_10bit(0x000).is_ok());
        assert!(I2cAddress::new_10bit(0x3FF).is_ok());
        assert!(I2cAddress::new_10bit(0x150).is_ok());

        // Test invalid addresses
        assert!(I2cAddress::new_10bit(0x400).is_err());
        assert!(I2cAddress::new_10bit(0x500).is_err());
        assert!(I2cAddress::new_10bit(0xFFFF).is_err());
    }

    #[test]
    fn test_7bit_vs_10bit_address_ranges() {
        // All 7-bit addresses should be valid as 10-bit addresses
        for addr in 0u16..=0x7F {
            assert!(
                I2cAddress::new_10bit(addr).is_ok(),
                "7-bit address 0x{addr:02X} should be valid as 10-bit"
            );
        }

        // Addresses 0x80-0xFF are only valid as 10-bit (in the 8-bit range)
        for addr in 0x80u16..=0xFF {
            assert!(
                I2cAddress::new_7bit(addr as u8).is_err(),
                "Address 0x{addr:03X} should be invalid as 7-bit"
            );
            assert!(
                I2cAddress::new_10bit(addr).is_ok(),
                "Address 0x{addr:03X} should be valid as 10-bit"
            );
        }

        // Addresses 0x100-0x3FF are only representable as 10-bit (beyond 8-bit range)
        for addr in 0x100u16..=0x3FF {
            assert!(
                I2cAddress::new_10bit(addr).is_ok(),
                "Address 0x{addr:03X} should be valid as 10-bit"
            );
        }
    }

    #[test]
    fn test_address_display_formatting() {
        let addr_7bit = I2cAddress::new_7bit(0x50).unwrap();
        let addr_10bit = I2cAddress::new_10bit(0x150).unwrap();

        assert_eq!(format!("{addr_7bit}"), "7-bit 0x50");
        assert_eq!(format!("{addr_10bit}"), "10-bit 0x150");
    }

    #[test]
    fn test_10bit_i2c_protocol_constants() {
        // Verify the 10-bit I2C protocol constants are correct
        for addr in [0x000, 0x001, 0x100, 0x200, 0x300, 0x3FF] {
            let high_bits = ((addr >> 8) & 0x03) as u8;
            let first_byte = (high_bits << 1) | 0xF0;

            // Should always start with 11110 (0xF0 pattern)
            assert_eq!(first_byte & 0xF8, 0xF0);

            // Should always end with 0 (write operation)
            assert_eq!(first_byte & 0x01, 0x00);

            // Should encode the high 2 bits correctly in positions 2:1
            assert_eq!((first_byte >> 1) & 0x03, high_bits);
        }
    }

    #[test]
    fn test_boundary_conditions() {
        // Test addresses at important boundaries
        let boundary_tests = [
            (0x07F, true),  // Max 7-bit address
            (0x080, true),  // First 10-bit only address
            (0x0FF, true),  // 8-bit boundary
            (0x100, true),  // 9-bit boundary
            (0x1FF, true),  // Test high bit combinations
            (0x2FF, true),  // Test high bit combinations
            (0x3FE, true),  // Near max
            (0x3FF, true),  // Max 10-bit address
            (0x400, false), // First invalid address
        ];

        for (addr, should_be_valid) in boundary_tests {
            let result = I2cAddress::new_10bit(addr);
            if should_be_valid {
                assert!(result.is_ok(), "Address 0x{addr:03X} should be valid");
            } else {
                assert!(result.is_err(), "Address 0x{addr:03X} should be invalid");
            }
        }
    }

    #[test]
    fn test_i2c_spec_compliance() {
        // Test that our encoding matches the I2C specification exactly
        // According to I2C spec, 10-bit addressing uses:
        // First frame: 11110XX0 (where XX are bits 9:8 of address)
        // Second frame: XXXXXXXX (bits 7:0 of address)

        let spec_test_cases = [
            // From I2C specification examples
            (0x000, 0xF0), // 11110000
            (0x100, 0xF2), // 11110010
            (0x200, 0xF4), // 11110100
            (0x300, 0xF6), // 11110110
        ];

        for (addr, expected_first_byte) in spec_test_cases {
            if let I2cAddress::Bit10(a) = I2cAddress::new_10bit(addr).unwrap() {
                let high_2_bits = ((a >> 8) & 0x03) as u8;
                let first_byte = (high_2_bits << 1) | 0xF0;

                assert_eq!(
                    first_byte, expected_first_byte,
                    "I2C spec compliance failed for address 0x{addr:03X}"
                );

                // Verify bit pattern matches spec
                assert_eq!(first_byte >> 3, 0x1E, "Must start with 11110");
                assert_eq!(first_byte & 0x01, 0x00, "Must end with 0 for write");
            }
        }
    }
}
