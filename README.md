# xr2280x-hid

[![crates.io](https://img.shields.io/crates/v/xr2280x-hid.svg)](https://crates.io/crates/xr2280x-hid)
[![docs.rs](https://docs.rs/xr2280x-hid/badge.svg)](https://docs.rs/xr2280x-hid)
[![License: WTFPL](https://img.shields.io/badge/License-WTFPL-brightgreen.svg)](LICENSE)
[![CI](https://github.com/tiborgats/xr2280x-hid/actions/workflows/ci.yml/badge.svg)](https://github.com/tiborgats/xr2280x-hid/actions/workflows/ci.yml)

Rust library for controlling MaxLinear/Exar XR2280x series USB-to-I²C/GPIO/PWM bridge chips via HID interface.

## Supported Devices

- **XR22800/XR22801**: 8 GPIO pins (0-7), I²C, PWM
- **XR22802/XR22804**: 32 GPIO pins (0-31), I²C, PWM

## Features

- **Unified Device API**: Single handle for both I²C and GPIO/PWM functionality
- **Multi-Device Support**: Enumerate and select from multiple connected devices
- **I²C Communication**: 7-bit/10-bit addressing, configurable speed, bus scanning
- **GPIO Control**: Individual pin and bulk operations, interrupts
- **PWM Output**: Configurable frequency and duty cycle
- **Cross-Platform**: Linux, macOS, Windows via hidapi

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
xr2280x-hid = "0.9.5"
hidapi = "2.6"
```

Basic usage:

```rust
use xr2280x_hid::{Xr2280x, GpioPin, GpioDirection, GpioLevel};
use hidapi::HidApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hid_api = HidApi::new()?;
    let device = Xr2280x::device_open_first(&hid_api)?;

    // I²C
    device.i2c_set_speed_khz(100)?;
    let devices = device.i2c_scan_default()?;

    // GPIO
    let pin = GpioPin::new(0)?;
    device.gpio_assign_to_edge(pin)?;
    device.gpio_set_direction(pin, GpioDirection::Output)?;
    device.gpio_write(pin, GpioLevel::High)?;

    Ok(())
}
```

## Examples

Run examples to see various features in action:

```bash
cargo run --example enumerate_hardware   # List connected devices
cargo run --example i2c_scan             # Scan I²C bus
cargo run --example blink                # GPIO blink
cargo run --example pwm_out              # PWM output
```

## Platform Setup

### Linux
Install hidapi development library:
```bash
sudo apt-get install libhidapi-dev   # Debian/Ubuntu
sudo dnf install hidapi-devel        # Fedora/RHEL
```

Add udev rules for device permissions:
```bash
# /etc/udev/rules.d/99-xr2280x.rules
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04e2", ATTRS{idProduct}=="1100", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04e2", ATTRS{idProduct}=="1200", MODE="0666"
```

### macOS
```bash
brew install hidapi
```

### Windows
Download hidapi.dll or use vcpkg. See [hidapi crate docs](https://docs.rs/hidapi/) for details.

## Hardware Notes

- **I²C**: Requires external pull-up resistors (typically 4.7kΩ to 3.3V)
- **GPIO**: 3.3V logic levels
- **Pin Mapping**: E0-E31 hardware pins map to GPIO 0-31 in software

## Requirements

- **Rust**: 1.82.0+ (uses Rust 2024 edition)
- **Hardware**: XR22800/1/2/4 device connected via USB

## Documentation

- [API Documentation](https://docs.rs/xr2280x-hid)
- [Examples](examples/) - Complete working examples
- [Changelog](CHANGELOG.md) - Version history
- [Hardware Tests](tests/) - Integration tests requiring hardware

## Multi-Device Selection

When multiple devices are connected:

```rust
// List all devices
let devices = Xr2280x::device_enumerate(&hid_api)?;

// Open by serial number
let device = Xr2280x::open_by_serial(&hid_api, "ABC123456")?;

// Open by index
let device = Xr2280x::open_by_index(&hid_api, 0)?;
```

## License

WTFPL - See [LICENSE](LICENSE) file for details.

## Contributing

Issues and pull requests welcome on [GitHub](https://github.com/tiborgats/xr2280x-hid).
