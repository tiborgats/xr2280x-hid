# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.1] - 2024-12-19

### Fixed
- **CRITICAL BUG**: Fixed I2C 7-bit address handling. Previously, 7-bit addresses were sent directly to the device without proper formatting for the I2C wire protocol. Addresses must be shifted left by 1 bit to create the proper 8-bit wire format where the 7-bit address occupies bits 7:1 and bit 0 is reserved for the R/W flag.
  - **Impact**: This bug caused devices to be addressed at half their intended address (e.g., a device at address 0x50 was actually being addressed as 0x28)
  - **Example**: EEPROM at 0x50 now correctly addressed as 0xA0 on wire instead of 0x50
  - **Breaking Change**: Existing code that was compensating for this bug by using doubled addresses will need to be updated

### Changed
- **Code Organization**: Refactored monolithic `lib.rs` into focused modules for better maintainability:
  - `device.rs` - Device discovery, opening, and management
  - `i2c.rs` - I2C communication functionality  
  - `gpio.rs` - GPIO control and configuration
  - `pwm.rs` - PWM generation and control
  - `interrupt.rs` - GPIO interrupt handling
- Minor API improvements:
  - `gpio_assign_to_edge()` no longer takes a boolean parameter (always assigns to EDGE)
  - `pwm_control()` parameter order changed to `(channel, enable, command)` for better clarity
  - `find_all()` now returns `Result<Vec<_>>` instead of an iterator for better error handling

### Added
- Comprehensive tests for I2C address format verification
- Detailed module-level documentation
- `REFACTORING.md` documenting the new module structure
- Better error messages and logging throughout

### Technical Details
- I2C 7-bit addresses are now properly converted to 8-bit wire format using left shift
- All examples and tests updated to reflect API changes
- Improved cross-module visibility using `pub(crate)` where appropriate
- Enhanced type safety and error handling

## [0.9.0] - Previous Release
- Initial modular release with I2C, GPIO, PWM, and interrupt support
- Support for XR22800, XR22801, XR22802, and XR22804 devices
- Cross-platform HID communication via hidapi
- Comprehensive examples and documentation