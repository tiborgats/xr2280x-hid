# I2C Error Guide for XR2280x

This guide explains common I2C errors you might encounter when using the XR2280x HID I2C interface and how to resolve them.

## Quick Error Reference

| Error | Meaning | Severity | Action |
|-------|---------|----------|--------|
| `I2cNack` | No device at address | Normal | Continue scanning |
| `I2cTimeout` | Bus stuck or slow device | **Critical** | Check power/connections |
| `I2cArbitrationLost` | Bus contention | **Critical** | Check for interference |
| `I2cRequestError` | Invalid parameters | Software | Fix code parameters |
| `I2cUnknownError` | Firmware issue | Hardware | Power cycle device |

## Error Details & Solutions

### 1. I2cNack (No Device Found)

**What it means:**
```
No device found at I2C address 0x48: Device did not acknowledge (NACK). 
This is normal when scanning for devices.
```

**Cause:** No I2C device is present at the specified address.

**Is this bad?** ❌ **NO** - This is completely normal during I2C bus scanning.

**What to do:** Continue scanning other addresses. This just means no device is connected at that specific I2C address.

---

### 2. I2cTimeout (Bus Stuck or Slow Device)

**What it means:**
```
I2C timeout at address 0x50: Device did not respond within timeout period. 
This may indicate: stuck bus (unpowered device holding lines low), very slow 
device, or hardware issues. Check device power and connections.
```

**Cause:** One of these issues:
1. **Unpowered device holding I2C lines low** (most common)
2. Very slow device that needs longer timeout
3. Defective device
4. Wiring issues

**Is this bad?** ⚠️ **YES** - This indicates a hardware problem.

**Solutions:**
1. **Check device power** - Ensure all I2C devices have proper power (3.3V or 5V)
2. **Disconnect devices one by one** to isolate the problematic device
3. **Check pull-up resistors** - Should be 4.7kΩ from SDA/SCL to 3.3V
4. **Try longer timeouts** for slow devices like EEPROMs:
   ```rust
   device.i2c_eeprom_write_7bit(0x50, &data)?; // Uses 5-second timeout
   // or
   device.i2c_read_7bit_with_timeout(0x50, &mut buf, 1000)?; // 1 second
   ```

---

### 3. I2cArbitrationLost (Bus Contention)

**What it means:**
```
I2C bus conflict at address 0x48: Arbitration lost (multiple masters competing 
for bus control). Check for other I2C controllers, loose connections, or 
electrical interference.
```

**Cause:** 
1. **Multiple I2C masters** trying to control the bus simultaneously
2. **Loose connections** causing signal glitches
3. **Electrical interference** from nearby devices
4. **Poor signal integrity** (long wires, no pull-ups)

**Is this bad?** ⚠️ **YES** - This indicates electrical or configuration issues.

**Solutions:**
1. **Disconnect other I2C controllers** - Only one master should be active
2. **Check connections** - Ensure SDA/SCL are firmly connected
3. **Reduce I2C speed** to improve signal integrity:
   ```rust
   device.i2c_set_speed_khz(50)?; // Slower = more reliable
   ```
4. **Use shorter wires** - Keep I2C wires under 30cm if possible
5. **Add/check pull-up resistors** - 4.7kΩ to 10kΩ to 3.3V
6. **Shield wires** if near sources of interference

---

### 4. I2cRequestError (Invalid Parameters)

**What it means:**
```
I2C request error at address 0x48: Invalid parameters sent to XR2280x firmware. 
Check data length (max 32 bytes), address validity, and operation flags.
```

**Cause:** Software error in your code:
1. **Data too long** - Maximum 32 bytes per transaction
2. **Invalid I2C address** - Must be 0x08-0x77 for 7-bit
3. **Invalid flags** passed to low-level functions

**Is this bad?** ❌ **NO** - This is a programming error, not hardware.

**Solutions:**
1. **Check data size** - Split large transfers into 32-byte chunks
2. **Validate addresses** - Use `I2cAddress::new_7bit()` for validation
3. **Use high-level methods** instead of `i2c_transfer_raw()` when possible

---

### 5. I2cUnknownError (Firmware Issue)

**What it means:**
```
I2C unknown error at address 0x48: Unexpected condition reported by XR2280x 
firmware (Status: 0x42). This may indicate firmware issues or unsupported 
operation.
```

**Cause:** XR2280x firmware reported an unexpected status code.

**Is this bad?** ⚠️ **YES** - This suggests hardware or firmware issues.

**Solutions:**
1. **Power cycle** the XR2280x device (unplug and reconnect USB)
2. **Try different I2C speed** - some devices are sensitive to timing
3. **Check USB connection** - try different USB port or cable
4. **Report issue** if error persists with device details and status code

---

## Hardware Troubleshooting Checklist

### Power Issues
- [ ] All I2C devices have proper power (3.3V or 5V as required)
- [ ] Power supply can provide enough current
- [ ] No brown-out conditions under load

### Connections
- [ ] SDA and SCL lines properly connected
- [ ] No loose or intermittent connections
- [ ] Pull-up resistors present (4.7kΩ recommended)
- [ ] Common ground between XR2280x and I2C devices

### Signal Integrity
- [ ] I2C wires shorter than 50cm (ideally under 30cm)
- [ ] Twisted pair or shielded cable for longer runs
- [ ] No parallel runs with power cables or PWM signals
- [ ] Proper grounding and shielding

### Device Issues
- [ ] I2C devices are not damaged
- [ ] Correct I2C addresses (not conflicting)
- [ ] Devices support the I2C speed being used

## Quick Diagnostic Commands

### Test Firmware Responsiveness
```rust
// This should complete in under 100ms
match device.i2c_scan_with_progress_and_timeout(0x00, 0x00, 5, |_,_,_,_| {}) {
    Err(Error::I2cTimeout { .. }) => println!("Firmware stuck - power cycle XR2280x"),
    _ => println!("Firmware responsive"),
}
```

### Test Bus Health
```rust
// Quick scan - should complete in under 1 second
let start = std::time::Instant::now();
match device.i2c_scan_default() {
    Ok(devices) => {
        println!("✓ Bus healthy, found {} devices in {:?}", devices.len(), start.elapsed());
    }
    Err(Error::I2cTimeout { .. }) => {
        println!("✗ Bus stuck - check for unpowered devices");
    }
    Err(Error::I2cArbitrationLost { .. }) => {
        println!("✗ Bus contention - check connections/interference");
    }
    Err(e) => println!("✗ Other issue: {}", e),
}
```

### Test Specific Device
```rust
let addr = 0x48; // Your device address
let mut test_data = [0u8; 1];

match device.i2c_read_7bit_with_timeout(addr, &mut test_data, 100) {
    Ok(_) => println!("✓ Device at 0x{:02X} is responsive", addr),
    Err(Error::I2cNack { .. }) => println!("- No device at 0x{:02X}", addr),
    Err(Error::I2cTimeout { .. }) => println!("✗ Device at 0x{:02X} is stuck/slow", addr),
    Err(e) => println!("✗ Device at 0x{:02X} error: {}", addr, e),
}
```

## Common Scenarios

### Scenario 1: "Scan finds nothing but I know devices are connected"
**Likely causes:**
- Wrong I2C address (check device datasheet)
- Device needs initialization sequence first
- Wrong power voltage (3.3V vs 5V)
- Missing pull-up resistors

### Scenario 2: "Scan hangs for a long time then fails"
**Likely causes:**
- Unpowered device holding bus lines low
- Defective device
- Short circuit on SDA or SCL

### Scenario 3: "Intermittent arbitration lost errors"
**Likely causes:**
- Loose connections
- Too-long wires with poor signal integrity
- Electrical interference from nearby devices
- Multiple I2C masters without proper coordination

### Scenario 4: "Device responds sometimes but not always"
**Likely causes:**
- Marginal power supply
- Borderline signal integrity
- Device-specific timing requirements
- Temperature-sensitive issues

## Performance Tips

### For Maximum Speed
```rust
// Ultra-fast scanning for responsive devices
device.i2c_scan_with_progress_and_timeout(0x08, 0x77, 3, callback)?;
```

### For Maximum Reliability
```rust
// Slower but more reliable for marginal setups
device.i2c_set_speed_khz(50)?; // Reduce speed
device.i2c_scan_with_progress_and_timeout(0x08, 0x77, 100, callback)?; // Longer timeout
```

### For Slow Devices (EEPROMs, etc.)
```rust
// Use specialized methods with appropriate timeouts
device.i2c_eeprom_write_7bit(0x50, &data)?; // 5-second timeout
// or custom timeout
device.i2c_write_7bit_with_timeout(0x50, &data, 2000)?; // 2-second timeout
```

---

**Remember:** The new timeout system prevents 29+ second hangs by failing fast (within 3 seconds) when hardware issues are detected. Fast failure is good - it means the protection is working!