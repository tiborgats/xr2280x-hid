# xr2280x-hid

[![crates.io](https://img.shields.io/crates/v/xr2280x-hid.svg)](https://crates.io/crates/xr2280x-hid)
[![docs.rs](https://docs.rs/xr2280x-hid/badge.svg)](https://docs.rs/xr2280x-hid)
[![License: WTFPL](https://img.shields.io/badge/License-WTFPL-brightgreen.svg)](LICENSE)
[![CI](https://github.com/tiborgats/xr2280x-hid/actions/workflows/ci.yml/badge.svg)](https://github.com/tiborgats/xr2280x-hid/actions/workflows/ci.yml)

Control MaxLinear/Exar XR22800, XR22801, XR22802, and XR22804 I²C, GPIO, PWM, and Interrupt configuration over USB HID using Rust.

This crate provides a **hardware-centric API** that groups USB logical interfaces by physical device, offering unified access to both I²C and GPIO/PWM functionality through a single device handle. Unlike other approaches that treat each USB interface separately, this crate presents a complete view of the hardware device matching physical reality.

Key advantages:
- **Unified Device Access**: Single handle for both I²C and GPIO/PWM (no separate logical device management)
- **Hardware Device Grouping**: Logical USB interfaces grouped by serial number into hardware devices  
- **Deterministic Ordering**: Consistent device enumeration ordered by serial number
- **Simplified Multi-Device Support**: Easy selection when multiple XR2280x devices are connected

It uses the cross-platform [`hidapi`](https://crates.io/crates/hidapi) crate and provides a high-level API over the raw HID reports.

## Features

*   **Device Grouping**: Groups logical USB interfaces by physical device
    *   Device discovery (`device_find_all`, `device_find_first`, `device_find`)
    *   Unified device opening (`device_open`, `device_open_first`)
    *   Multi-device selection (`open_by_serial`, `open_by_index`)

*   Device information and capabilities (`get_device_info`, `get_capabilities`).
*   I²C communication:
    *   Speed setting (`i2c_set_speed_khz`).
    *   7-bit and 10-bit addressing support.
    *   Basic transfers (`i2c_write_7bit`, `i2c_read_7bit`, `i2c_write_read_7bit`, etc.).
    *   Raw transfers with custom flags and timeouts (`i2c_transfer_raw`).
*   GPIO (EDGE) control (Pins 0-31 mapped from E0-E31, model dependent):
    *   Strongly-typed `GpioPin` struct.
    *   Single pin and bulk (masked) operations.
    *   Assigning pins between UART/GPIO and EDGE functions.
    *   Setting/getting pin direction.
    *   Reading/Writing pin levels.
    *   Setting/getting pull-up/pull-down resistors.
    *   Setting/getting open-drain/tri-state output modes.
    *   Checking pin assignment.
*   GPIO Interrupt configuration (`gpio_configure_interrupt`).
*   Reading raw GPIO interrupt reports (`read_gpio_interrupt_report`).
*   Speculative parsing of GPIO interrupt reports (`parse_gpio_interrupt_report` - **Format Unverified**).
*   PWM Output configuration:
    *   Setting/getting periods using device units or nanoseconds.
    *   Setting/getting assigned output pin.
    *   Setting/getting control mode and enable state.

## Chip Support & Limitations

This crate aims to support the HID interfaces common across the XR2280x family.

*   **I²C:** Fully supported on all models (XR22800/1/2/4). Includes 7-bit and 10-bit addressing.
*   **EDGE (GPIO/PWM/Interrupts):**
    *   **XR22802/XR22804:** Support 32 GPIOs (E0-E31), mapped to pins 0-31. PWM can be assigned to any of these pins (if configured as output).
    *   **XR22800/XR22801:** Support **only 8 GPIOs (E0-E7)**, mapped to pins 0-7, via the HID interface. Attempts to access pins 8-31 will return an `Error::UnsupportedFeature`. PWM output can only be assigned to pins 0-7 on these models.
*   **Interrupt Parsing:** Reading raw interrupt reports is supported, but parsing (`parse_gpio_interrupt_report`) is speculative due to lack of documentation and requires hardware verification.

The crate attempts to detect the GPIO capability (8 vs 32 pins) when the device is opened by checking for the presence of higher-group registers. Use `get_capabilities()` on the device handle to check.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
xr2280x-hid = "0.9.3" # Replace with the latest version
hidapi = "2.6"       # Or latest compatible version
log = "0.4"          # Optional, for logging

[dev-dependencies]   # For examples/tests
env_logger = "0.11"
approx = "0.5"
```

You also need the `hidapi` library installed on your system. See the [`hidapi` crate documentation](https://docs.rs/hidapi/) for details.

## Basic Usage

```rust
use xr2280x_hid::{self, gpio::{GpioDirection, GpioLevel, GpioPin}, Result};
use hidapi::HidApi;
use std::{thread, time::Duration};

fn main() -> Result<()> {
    // Optional: Initialize logging (e.g., RUST_LOG=xr2280x_hid=debug cargo run ...)
    // env_logger::init();

    let hid_api = HidApi::new()?;

    // --- Option 1: Open the first default device found ---
    // Convenient but potentially ambiguous if multiple devices are present.
    println!("Opening first default XR2280x device...");
    let device = match xr2280x_hid::Xr2280x::device_open_first(&hid_api) {
        Ok(dev) => dev,
        Err(e) => {
            eprintln!("Error opening device: {}", e);
            eprintln!("Ensure device is connected and permissions are set (e.g., udev rules on Linux).");
            return Err(e);
        }
    };
    println!("Device opened (first found). Info: {:?}", device.get_device_info()?);
    println!("Detected capabilities: {:?}", device.get_capabilities());


    // --- Use the opened device ---
    println!("\nSetting I2C speed...");
    device.i2c_set_speed_khz(100)?;

    // --- GPIO Example (Pin E0 / GPIO 0) ---
    let gpio_pin = GpioPin::new(0)?; // Use typed pin
    println!("\n--- GPIO Example (Pin {}) ---", gpio_pin.number());
    device.gpio_assign_to_edge(gpio_pin, true)?; // Assign E0 to EDGE
    device.gpio_set_direction(gpio_pin, GpioDirection::Output)?;
    device.gpio_set_pull(gpio_pin, xr2280x_hid::gpio::GpioPull::None)?;

    println!("Blinking pin {}...", gpio_pin.number());
    device.gpio_write(gpio_pin, GpioLevel::High)?;
    thread::sleep(Duration::from_millis(200));
    device.gpio_write(gpio_pin, GpioLevel::Low)?;

    // Set back to input
    device.gpio_set_direction(gpio_pin, GpioDirection::Input)?;

    Ok(())
}
```

## Multi-Device Selection

When multiple XR2280x devices are connected to the same system, the crate provides several methods to reliably select and open specific devices:

### Device Enumeration

```rust
use xr2280x_hid::Xr2280x;
use hidapi::HidApi;

let hid_api = HidApi::new()?;

// Enumerate all XR2280x devices
let devices = Xr2280x::device_enumerate(&hid_api)?;
println!("Found {} XR2280x devices:", devices.len());

for (i, info) in devices.iter().enumerate() {
    println!("  [{}] Serial: {}, Product: {}",
        i,
        info.serial_number.as_deref().unwrap_or("N/A"),
        info.product_string.as_deref().unwrap_or("Unknown")
    );
}
```

### Opening Devices by Specific Criteria

#### 1. Open by Serial Number

```rust
// Open specific device by serial number
let device = Xr2280x::open_by_serial(&hid_api, "ABC123456")?;
```

#### 2. Open by Index

```rust
// Open the second device found (0-based indexing)
let device = Xr2280x::open_by_index(&hid_api, 1)?;
```

#### 3. Open by Device Path

```rust
use std::ffi::CString;

// Open device by platform-specific path
let path = CString::new("/dev/hidraw0")?;
let device = Xr2280x::open_by_path(&hid_api, &path)?;
```

#### 4. Open from Existing HidDevice

```rust
// If you need to open by path, use open_by_path directly
let xr_device = Xr2280x::open_by_path(&hid_api, &device_path)?;
```

### Error Handling for Multi-Device Selection

The multi-device selection methods provide specific error types for better error handling:

```rust
use xr2280x_hid::{Xr2280x, Error};

match Xr2280x::open_by_serial(&hid_api, "NONEXISTENT") {
    Ok(device) => println!("Device opened successfully"),
    Err(Error::DeviceNotFoundBySerial { serial, message }) => {
        println!("No device found with serial '{}': {}", serial, message);
    },
    Err(Error::DeviceNotFoundByIndex { index, message }) => {
        println!("No device found at index {}: {}", index, message);
    },
    Err(Error::DeviceNotFoundByPath { path, message }) => {
        println!("No device found at path '{}': {}", path, message);
    },
    Err(e) => println!("Other error: {}", e),
}
```

### Interactive Device Selection

For applications that need user selection, you can combine enumeration with user input:

```rust
let devices = Xr2280x::device_enumerate(&hid_api)?;

if devices.len() > 1 {
    // Display devices to user
    for (i, info) in devices.iter().enumerate() {
        println!("[{}] {} (Serial: {})", 
            i, 
            info.product_string.as_deref().unwrap_or("XR2280x"),
            info.serial_number.as_deref().unwrap_or("N/A")
        );
    }
    
    // Get user selection
    let index = get_user_selection()?; // Your input function
    let device = Xr2280x::open_by_index(&hid_api, index)?;
} else {
    let device = Xr2280x::device_open_first(&hid_api)?;
}
```

### Legacy Methods

For compatibility with existing code:

- `device_find_all(&hid_api)` - Returns `Vec<XrDeviceInfo>` for all devices
- `device_find_first(&hid_api)` - Returns the first device's info
- `device_open(&hid_api, &device_info)` - Opens using device info

See the `examples/enumerate_hardware.rs` and `examples/multi_device_selection.rs` examples for complete demonstrations.

## Hardware Setup Notes

*   **I²C Pull-up Resistors:** Required externally (e.g., 4.7kΩ to 3.3V).
*   **Linux udev Rules:** Grant user permission to the HID devices. Create `/etc/udev/rules.d/99-xr2280x.rules`:
    ```udev
    # Rule for Exar/MaxLinear XR2280x HID Interfaces (Default PIDs: I2C=1100, EDGE=1200)
    SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04e2", ATTRS{idProduct}=="1100", MODE="0666", GROUP="plugdev"
    SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04e2", ATTRS{idProduct}=="1200", MODE="0666", GROUP="plugdev"
    
    # Add similar rules if using custom VID/PIDs
    # SUBSYSTEM=="hidraw", ATTRS{idVendor}=="YOUR_VID", ATTRS{idProduct}=="YOUR_PID", MODE="0666", GROUP="plugdev"
    ```
    *(Adjust `GROUP` if needed)*. Reload: `sudo udevadm control --reload-rules && sudo udevadm trigger`
*   **GPIO Voltage Levels:** Typically 3.3V logic.

## Pin Mapping

*   Pins E0-E7 map to `GpioPin(0)`-`GpioPin(7)` (Group 0). Supported on all models.
*   Pins E8-E15 map to `GpioPin(8)`-`GpioPin(15)` (Group 0). Supported on XR22802/4 only.
*   Pins E16-E31 map to `GpioPin(16)`-`GpioPin(31)` (Group 1). Supported on XR22802/4 only.

## Building

### Prerequisites

- **hidapi Development Library:** This is crucial for the hidapi crate to compile and link correctly.
  - **Linux (Debian/Ubuntu):** sudo apt-get update && sudo apt-get install libhidapi-dev
  - **Linux (Fedora/CentOS/RHEL):** sudo dnf install hidapi-devel
  - **macOS:** brew install hidapi
  - **Windows:** Download a pre-compiled hidapi.dll (usually from the hidapi release page or bundled with other software) and place it either in the same directory as your final executable or somewhere in your system's PATH. Alternatively, use vcpkg or build from source using CMake/MSVC. See the hidapi crate docs for more details.
- **Permissions (Linux):** Ensure your user has permission to access the HID device. Add the udev rules mentioned in the README.md and reload them (sudo udevadm control --reload-rules && sudo udevadm trigger). You might need to re-plug the device or add your user to the specified group (e.g., plugdev, dialout) and log out/in.

### Build commands

Library:

```sh
cargo build --release
```

Unit tests (don't require hardware):

```sh
cargo test
```

Examples:

```sh
cargo build --examples
```

Running the examples:

```sh
# Run the blink example
cargo run --example blink

# Run the I2C scanner
cargo run --example i2c_scan

# Run the device listing example
cargo run --example list_and_select

# Run the PWM output example
cargo run --example pwm_out
```*   **With Logging:** To see the `debug!` and `trace!` messages from the library, set the `RUST_LOG` environment variable before running:
```bash
# Linux/macOS (show debug messages from our crate)
RUST_LOG=xr2280x_hid=debug cargo run --example blink

# Linux/macOS (show trace messages - very verbose!)
RUST_LOG=xr2280x_hid=trace cargo run --example i2c_scan

# Windows (Command Prompt)
set RUST_LOG=xr2280x_hid=debug
cargo run --example blink

# Windows (PowerShell)
$env:RUST_LOG="xr2280x_hid=debug"
cargo run --example blink
```

Running Integration/Hardware Tests (Requires Hardware):

The tests in the tests/ directory are marked with #[ignore] because they require specific hardware setups (like an I2C device at a known address or an oscilloscope/logic analyzer to verify PWM/GPIO). To run *only* the ignored tests:

```sh
cargo test -- --ignored
```

To run *all* tests, including ignored ones:

```sh
cargo test -- --include-ignored
```

**Important:** These tests might panic if the expected hardware isn't present or doesn't behave as expected (e.g., the test_i2c_presence_check expects specific addresses to respond or not respond). You may need to modify the test code (like the I2C addresses) to match your specific setup. Running hardware tests sequentially (--test-threads=1) can sometimes help avoid bus contention issues:

```sh
cargo test -- --ignored --test-threads=1
```

### Code modification

After modifying the source code, check it with RustFmt

```sh
cargo fmt --check
```

modify automatically:

```sh
cargo fmt
```

Check with Clippy too:

```sh
cargo clippy --all-targets -- -D warnings
```

## License

This project is licensed under the WTFPL - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues on the [repository](https://github.com/tiborgats/xr2280x-hid) .