# I2C Timeout System Improvements

## Overview

This document describes the major improvements made to the I2C timeout handling system in xr2280x-hid, addressing critical issues with stuck bus detection and inflexible timeout configuration.

## Problems Addressed

### 1. Inflexible Single Timeout

**Previous Issue:**
- All I2C operations used a single `DEFAULT_I2C_TIMEOUT_MS = 500` constant
- No way to optimize timeouts for different device types or use cases
- Slow for responsive devices, potentially too fast for slow devices

**Solution:**
- Operation-specific timeout constants in `timeouts` module
- Custom timeout methods for precise control
- Specialized methods for common device types (e.g., EEPROMs)

### 2. Stuck Bus Hanging

**Previous Issue:**
- When unpowered devices drive I2C lines low, scanning would hang indefinitely
- No detection or recovery mechanism for stuck bus conditions
- Applications could freeze waiting for unresponsive hardware

**Solution:**
- Automatic stuck bus detection during scanning
- Pre-scan verification using reserved addresses
- Consecutive timeout monitoring with early termination
- Clear error reporting for hardware issues

## New Timeout System

### Default Timeouts

| Constant | Value | Use Case |
|----------|-------|----------|
| `timeouts::PROBE` | 3ms | Ultra-fast firmware responsiveness testing |
| `timeouts::SCAN` | 8ms | Fast device discovery with stuck bus protection |
| `timeouts::READ` | 100ms | Sensor readings, register access |
| `timeouts::WRITE` | 200ms | Configuration writes |
| `timeouts::WRITE_READ` | 250ms | Combined operations |
| `timeouts::EEPROM_WRITE` | 5000ms | Slow memory operations |
| `timeouts::FIRMWARE_RESPONSIVENESS` | 100ms | Maximum wait for firmware response |

### Method Categories

#### 1. Default Timeout Methods
```rust
// Use operation-appropriate defaults
device.i2c_read_7bit(0x48, &mut buffer)?;           // 100ms
device.i2c_write_7bit(0x48, &data)?;                // 200ms
device.i2c_write_read_7bit(0x48, &cmd, &mut buf)?;  // 250ms
```

#### 2. Custom Timeout Methods
```rust
// Precise timeout control
device.i2c_read_7bit_with_timeout(0x48, &mut buffer, 50)?;    // 50ms
device.i2c_write_7bit_with_timeout(0x50, &data, 2000)?;      // 2 seconds
```

#### 3. Specialized Methods
```rust
// Pre-configured for device types
device.i2c_eeprom_write_7bit(0x50, &data)?;                  // 5 second timeout
device.i2c_scan_with_progress_and_timeout(0x08, 0x77, 3, callback)?; // 3ms per address
```

## Stuck Bus Detection

### How It Works

1. **Pre-scan Test**: Tests reserved address 0x00 with ultra-fast 3ms timeout
2. **Immediate Failure**: Fails after just 1 consecutive timeout (not 5+)
3. **Firmware Responsiveness**: Tests if XR2280x firmware responds within 3ms
4. **Fast Termination**: Returns error within seconds instead of 29+ second hangs

### Example Error Handling

```rust
match device.i2c_scan_default() {
    Ok(devices) => {
        println!("Found {} devices", devices.len());
        for addr in devices {
            println!("  Device at 0x{:02X}", addr);
        }
    }
    Err(Error::I2cTimeout { address }) => {
        eprintln!("ERROR: Stuck I2C bus detected at {}", address);
        eprintln!("Possible causes:");
        eprintln!("  - Unpowered device driving bus lines low");
        eprintln!("  - Short circuit on SDA/SCL lines");
        eprintln!("  - Hardware malfunction");
        eprintln!("Check connections and device power before retrying.");
    }
    Err(e) => eprintln!("Scan failed: {}", e),
}
```

## Performance Improvements

### Scanning Speed

| Scenario | Old System | New System | Improvement |
|----------|------------|------------|-------------|
| Fast devices (responsive) | ~500ms per addr | ~3-8ms per addr | **60x faster** |
| Mixed devices | ~500ms per addr | ~8-100ms per addr | **5-60x faster** |
| Stuck bus | Hangs for 29+ seconds | Fails in <3 seconds | **Prevents hanging** |

### Real-World Impact

```rust
// Scanning 112 addresses (0x08-0x77):
// Old: 112 × 500ms = 56 seconds (if all timeout) or 29+ sec hang (stuck bus)
// New: 112 × 8ms = 0.9 seconds (normal case)
//      or early termination in <3 seconds (stuck bus)
```

## Usage Guidelines

### Choose the Right Timeout

#### Fast/Responsive Devices (Sensors, etc.)
```rust
// Use fast timeouts for snappy response
device.i2c_read_7bit_with_timeout(0x48, &mut temp, timeouts::PROBE)?; // 3ms
```

#### Standard Devices (Most cases)
```rust
// Use defaults - balanced performance
device.i2c_read_7bit(0x48, &mut data)?;  // 100ms
device.i2c_write_7bit(0x48, &config)?;   // 200ms
```

#### Slow Devices (EEPROMs, Flash memory)
```rust
// Use extended timeouts or specialized methods
device.i2c_eeprom_write_7bit(0x50, &page_data)?;           // 5 seconds
device.i2c_write_7bit_with_timeout(0x50, &data, 10000)?;   // Custom 10s
```

#### Real-Time Applications
```rust
// Guarantee maximum response time
device.i2c_read_7bit_with_timeout(0x68, &mut imu_data, 50)?; // 50ms max
```

### Scanning Optimization

#### Quick Discovery
```rust
// Ultra-fast scan for known responsive devices
let devices = device.i2c_scan_with_progress_and_timeout(
    0x08, 0x77, 
    3,  // 3ms timeout
    |addr, found, idx, total| {
        if found { println!("Quick device at 0x{:02X}", addr); }
    }
)?;
```

#### Comprehensive Scan
```rust
// Standard scan with stuck bus protection
let devices = device.i2c_scan_default()?;  // Uses 8ms timeout with 3ms responsiveness test
```

#### Tolerant Scan
```rust
// Slower scan for difficult environments
let devices = device.i2c_scan_with_progress_and_timeout(
    0x08, 0x77,
    100, // 100ms timeout
    |addr, found, idx, total| {
        println!("Scanned {}/{}: 0x{:02X} {}", 
                idx + 1, total, addr, 
                if found { "✓" } else { "-" });
    }
)?;
```

## Migration Guide

### From Old API

```rust
// OLD: Fixed 500ms timeout
device.i2c_transfer_raw(addr, Some(&data), None, flags, Some(500))?;

// NEW: Operation-specific defaults
device.i2c_write_7bit(slave_addr, &data)?;  // 200ms default

// NEW: Custom timeout when needed
device.i2c_write_7bit_with_timeout(slave_addr, &data, 500)?;  // Explicit 500ms
```

### Timeout Selection Strategy

1. **Start with defaults** - They work well for most cases
2. **Profile your devices** - Measure actual response times
3. **Optimize hot paths** - Use shorter timeouts for frequent operations
4. **Plan for failures** - Use longer timeouts for critical operations
5. **Handle stuck bus** - Always catch and handle `I2cTimeout` errors

## Error Handling Best Practices

```rust
use xr2280x_hid::{Error, timeouts};

fn robust_i2c_operation(device: &Xr2280x) -> Result<Vec<u8>> {
    match device.i2c_scan_with_progress_and_timeout(0x08, 0x77, timeouts::SCAN, |_, _, _, _| {}) {
        Ok(devices) => {
            println!("Scan completed successfully");
            Ok(devices)
        }
        Err(Error::I2cTimeout { address }) => {
            eprintln!("Stuck bus detected at {}", address);
            eprintln!("Hardware intervention required");
            Err(Error::I2cTimeout { address })
        }
        Err(Error::I2cNack { address }) => {
            // Normal - device not present
            println!("No device at {}", address);
            Ok(vec![])
        }
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
            Err(e)
        }
    }
}
```

## Hardware Troubleshooting

When you encounter stuck bus errors:

1. **Check Power**: Ensure all I2C devices are properly powered
2. **Verify Connections**: Confirm SDA/SCL wiring and pull-up resistors
3. **Isolate Devices**: Disconnect devices one by one to identify the problematic one
4. **Test Voltages**: Measure SDA/SCL line voltages (should be ~3.3V when idle)
5. **Reset Bus**: Power cycle the entire I2C bus if possible

## Performance Monitoring

```rust
use std::time::Instant;

let start = Instant::now();
let devices = device.i2c_scan_default()?;
let duration = start.elapsed();

println!("Scanned {} addresses in {:?}", 
         devices.len(), duration);
println!("Average time per address: {:?}", 
         duration / devices.len() as u32);
```

This improvement makes I2C communication more reliable, **dramatically faster** (up to 60x), and **guarantees fast failure** (within 3 seconds) instead of 29+ second hangs when hardware issues occur.