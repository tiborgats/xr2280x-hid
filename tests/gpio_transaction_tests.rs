//! Unit tests for GPIO Transaction API
//!
//! These tests verify the transaction logic without requiring hardware.

use xr2280x_hid::gpio::{GpioLevel, GpioPin};

// Mock device for testing transaction logic
struct MockDevice;

impl MockDevice {
    fn gpio_transaction(&self) -> MockTransaction {
        MockTransaction::new()
    }
}

// Simplified mock transaction for testing logic
#[derive(Debug)]
struct MockTransaction {
    group0_changes: (u16, u16), // (set_mask, clear_mask)
    group1_changes: (u16, u16),
    has_changes: bool,
}

impl MockTransaction {
    fn new() -> Self {
        Self {
            group0_changes: (0, 0),
            group1_changes: (0, 0),
            has_changes: false,
        }
    }

    fn set_pin(&mut self, pin: GpioPin, level: GpioLevel) -> Result<(), &'static str> {
        let mask = pin.mask();
        let (set_mask, clear_mask) = match pin.group_index() {
            0 => &mut self.group0_changes,
            _ => &mut self.group1_changes,
        };

        match level {
            GpioLevel::High => {
                *set_mask |= mask;
                *clear_mask &= !mask;
            }
            GpioLevel::Low => {
                *clear_mask |= mask;
                *set_mask &= !mask;
            }
        }

        self.has_changes = true;
        Ok(())
    }

    fn set_high(&mut self, pin: GpioPin) -> Result<(), &'static str> {
        self.set_pin(pin, GpioLevel::High)
    }

    fn set_low(&mut self, pin: GpioPin) -> Result<(), &'static str> {
        self.set_pin(pin, GpioLevel::Low)
    }

    fn set_all_high(&mut self, pins: &[GpioPin]) -> Result<(), &'static str> {
        for &pin in pins {
            self.set_high(pin)?;
        }
        Ok(())
    }

    fn set_all_low(&mut self, pins: &[GpioPin]) -> Result<(), &'static str> {
        for &pin in pins {
            self.set_low(pin)?;
        }
        Ok(())
    }

    fn with_pin(mut self, pin: GpioPin, level: GpioLevel) -> Result<Self, &'static str> {
        self.set_pin(pin, level)?;
        Ok(self)
    }

    fn with_high(mut self, pin: GpioPin) -> Result<Self, &'static str> {
        self.set_high(pin)?;
        Ok(self)
    }

    fn with_low(mut self, pin: GpioPin) -> Result<Self, &'static str> {
        self.set_low(pin)?;
        Ok(self)
    }

    fn clear(&mut self) {
        self.group0_changes = (0, 0);
        self.group1_changes = (0, 0);
        self.has_changes = false;
    }

    fn has_pending_changes(&self) -> bool {
        self.has_changes
    }

    fn pending_pin_count(&self) -> usize {
        let group0_count = (self.group0_changes.0 | self.group0_changes.1).count_ones();
        let group1_count = (self.group1_changes.0 | self.group1_changes.1).count_ones();
        (group0_count + group1_count) as usize
    }

    fn commit(&mut self) -> usize {
        if !self.has_changes {
            return 0;
        }

        let mut transaction_count = 0;

        // Count Group 0 transactions
        let (set_mask_0, clear_mask_0) = self.group0_changes;
        if set_mask_0 != 0 {
            transaction_count += 1;
        }
        if clear_mask_0 != 0 {
            transaction_count += 1;
        }

        // Count Group 1 transactions
        let (set_mask_1, clear_mask_1) = self.group1_changes;
        if set_mask_1 != 0 {
            transaction_count += 1;
        }
        if clear_mask_1 != 0 {
            transaction_count += 1;
        }

        self.clear();
        transaction_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pins() -> Vec<GpioPin> {
        vec![
            GpioPin::new(0).unwrap(),  // Group 0, bit 0
            GpioPin::new(1).unwrap(),  // Group 0, bit 1
            GpioPin::new(2).unwrap(),  // Group 0, bit 2
            GpioPin::new(16).unwrap(), // Group 1, bit 0
            GpioPin::new(17).unwrap(), // Group 1, bit 1
        ]
    }

    #[test]
    fn test_transaction_creation() {
        let device = MockDevice;
        let transaction = device.gpio_transaction();

        assert!(!transaction.has_pending_changes());
        assert_eq!(transaction.pending_pin_count(), 0);
        assert_eq!(transaction.group0_changes, (0, 0));
        assert_eq!(transaction.group1_changes, (0, 0));
    }

    #[test]
    fn test_single_pin_operations() {
        let pins = create_test_pins();
        let mut transaction = MockTransaction::new();

        // Test setting pin high
        transaction.set_pin(pins[0], GpioLevel::High).unwrap();
        assert!(transaction.has_pending_changes());
        assert_eq!(transaction.pending_pin_count(), 1);
        assert_eq!(transaction.group0_changes.0, 0b0001); // Set mask
        assert_eq!(transaction.group0_changes.1, 0b0000); // Clear mask

        // Test setting pin low
        transaction.set_pin(pins[1], GpioLevel::Low).unwrap();
        assert_eq!(transaction.pending_pin_count(), 2);
        assert_eq!(transaction.group0_changes.0, 0b0001); // Set mask unchanged
        assert_eq!(transaction.group0_changes.1, 0b0010); // Clear mask has bit 1
    }

    #[test]
    fn test_pin_override() {
        let pins = create_test_pins();
        let mut transaction = MockTransaction::new();

        // Set pin high, then low
        transaction.set_pin(pins[0], GpioLevel::High).unwrap();
        assert_eq!(transaction.group0_changes.0, 0b0001);
        assert_eq!(transaction.group0_changes.1, 0b0000);

        transaction.set_pin(pins[0], GpioLevel::Low).unwrap();
        assert_eq!(transaction.group0_changes.0, 0b0000); // Removed from set
        assert_eq!(transaction.group0_changes.1, 0b0001); // Added to clear

        // Set pin high again
        transaction.set_pin(pins[0], GpioLevel::High).unwrap();
        assert_eq!(transaction.group0_changes.0, 0b0001); // Back in set
        assert_eq!(transaction.group0_changes.1, 0b0000); // Removed from clear
    }

    #[test]
    fn test_multi_group_operations() {
        let pins = create_test_pins();
        let mut transaction = MockTransaction::new();

        // Set pins in both groups
        transaction.set_pin(pins[0], GpioLevel::High).unwrap(); // Group 0, pin 0
        transaction.set_pin(pins[3], GpioLevel::High).unwrap(); // Group 1, pin 16

        assert_eq!(transaction.pending_pin_count(), 2);
        assert_eq!(transaction.group0_changes.0, 0b0001);
        assert_eq!(transaction.group1_changes.0, 0b0001);
    }

    #[test]
    fn test_convenience_methods() {
        let pins = create_test_pins();
        let mut transaction = MockTransaction::new();

        // Test set_high and set_low
        transaction.set_high(pins[0]).unwrap();
        transaction.set_low(pins[1]).unwrap();

        assert_eq!(transaction.group0_changes.0, 0b0001);
        assert_eq!(transaction.group0_changes.1, 0b0010);

        // Test set_all_high and set_all_low
        transaction.clear();
        transaction.set_all_high(&pins[0..3]).unwrap();
        assert_eq!(transaction.group0_changes.0, 0b0111); // Bits 0, 1, 2

        transaction.clear();
        transaction.set_all_low(&pins[0..3]).unwrap();
        assert_eq!(transaction.group0_changes.1, 0b0111); // Bits 0, 1, 2
    }

    #[test]
    fn test_method_chaining() {
        let pins = create_test_pins();

        let transaction = MockTransaction::new()
            .with_high(pins[0])
            .unwrap()
            .with_low(pins[1])
            .unwrap()
            .with_pin(pins[2], GpioLevel::High)
            .unwrap();

        assert_eq!(transaction.group0_changes.0, 0b0101); // Bits 0, 2 set
        assert_eq!(transaction.group0_changes.1, 0b0010); // Bit 1 clear
        assert_eq!(transaction.pending_pin_count(), 3);
    }

    #[test]
    fn test_commit_behavior() {
        let pins = create_test_pins();
        let mut transaction = MockTransaction::new();

        // Empty transaction commit
        assert_eq!(transaction.commit(), 0);
        assert!(!transaction.has_pending_changes());

        // Single group, both set and clear
        transaction.set_high(pins[0]).unwrap();
        transaction.set_low(pins[1]).unwrap();
        let hid_count = transaction.commit();
        assert_eq!(hid_count, 2); // One SET, one CLEAR transaction
        assert!(!transaction.has_pending_changes());
        assert_eq!(transaction.pending_pin_count(), 0);

        // Multi-group transaction
        transaction.set_high(pins[0]).unwrap(); // Group 0
        transaction.set_high(pins[3]).unwrap(); // Group 1
        let hid_count = transaction.commit();
        assert_eq!(hid_count, 2); // One SET per group
    }

    #[test]
    fn test_clear_functionality() {
        let pins = create_test_pins();
        let mut transaction = MockTransaction::new();

        // Add some changes
        transaction.set_high(pins[0]).unwrap();
        transaction.set_low(pins[1]).unwrap();
        assert!(transaction.has_pending_changes());
        assert_eq!(transaction.pending_pin_count(), 2);

        // Clear transaction
        transaction.clear();
        assert!(!transaction.has_pending_changes());
        assert_eq!(transaction.pending_pin_count(), 0);
        assert_eq!(transaction.group0_changes, (0, 0));
        assert_eq!(transaction.group1_changes, (0, 0));
    }

    #[test]
    fn test_transaction_reuse() {
        let pins = create_test_pins();
        let mut transaction = MockTransaction::new();

        // First use
        transaction.set_high(pins[0]).unwrap();
        let hid_count = transaction.commit();
        assert_eq!(hid_count, 1);
        assert!(!transaction.has_pending_changes());

        // Reuse the same transaction
        transaction.set_low(pins[1]).unwrap();
        assert!(transaction.has_pending_changes());
        assert_eq!(transaction.pending_pin_count(), 1);

        let hid_count = transaction.commit();
        assert_eq!(hid_count, 1);
        assert!(!transaction.has_pending_changes());
    }

    #[test]
    fn test_complex_pin_patterns() {
        let pins = create_test_pins();
        let mut transaction = MockTransaction::new();

        // Create a complex pattern across multiple groups
        transaction.set_high(pins[0]).unwrap(); // Group 0, bit 0
        transaction.set_low(pins[1]).unwrap(); // Group 0, bit 1
        transaction.set_high(pins[2]).unwrap(); // Group 0, bit 2
        transaction.set_high(pins[3]).unwrap(); // Group 1, bit 0
        transaction.set_low(pins[4]).unwrap(); // Group 1, bit 1

        assert_eq!(transaction.pending_pin_count(), 5);

        // Group 0: pins 0,2 high (0b0101), pin 1 low (0b0010)
        assert_eq!(transaction.group0_changes.0, 0b0101);
        assert_eq!(transaction.group0_changes.1, 0b0010);

        // Group 1: pin 0 high (0b0001), pin 1 low (0b0010)
        assert_eq!(transaction.group1_changes.0, 0b0001);
        assert_eq!(transaction.group1_changes.1, 0b0010);

        // Should result in 4 HID transactions (set+clear for each group)
        let hid_count = transaction.commit();
        assert_eq!(hid_count, 4);
    }

    #[test]
    fn test_pin_mask_calculations() {
        // Test that pin masks are calculated correctly
        let pin0 = GpioPin::new(0).unwrap();
        let pin1 = GpioPin::new(1).unwrap();
        let pin15 = GpioPin::new(15).unwrap();
        let pin16 = GpioPin::new(16).unwrap();

        assert_eq!(pin0.mask(), 0b0000_0000_0000_0001);
        assert_eq!(pin1.mask(), 0b0000_0000_0000_0010);
        assert_eq!(pin15.mask(), 0b1000_0000_0000_0000);
        assert_eq!(pin16.mask(), 0b0000_0000_0000_0001); // Group 1, bit 0

        assert_eq!(pin0.group_index(), 0);
        assert_eq!(pin15.group_index(), 0);
        assert_eq!(pin16.group_index(), 1);
    }

    #[test]
    fn test_only_set_operations() {
        let pins = create_test_pins();
        let mut transaction = MockTransaction::new();

        // Only set pins high (no clear operations)
        transaction.set_high(pins[0]).unwrap();
        transaction.set_high(pins[1]).unwrap();
        transaction.set_high(pins[3]).unwrap(); // Different group

        let hid_count = transaction.commit();
        assert_eq!(hid_count, 2); // One SET operation per group
    }

    #[test]
    fn test_only_clear_operations() {
        let pins = create_test_pins();
        let mut transaction = MockTransaction::new();

        // Only set pins low (no set operations)
        transaction.set_low(pins[0]).unwrap();
        transaction.set_low(pins[1]).unwrap();
        transaction.set_low(pins[3]).unwrap(); // Different group

        let hid_count = transaction.commit();
        assert_eq!(hid_count, 2); // One CLEAR operation per group
    }
}
