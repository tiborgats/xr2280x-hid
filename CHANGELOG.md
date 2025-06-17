# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.3] - 2025-06-16

### Added
- **Multi-Device Selection Support**: Comprehensive device selection when multiple XR2280x devices are connected
  - `Xr2280x::enumerate_devices()` - Get list of all available XR2280x devices
  - `Xr2280x::open_by_serial()` - Open device by serial number
  - `Xr2280x::open_by_index()` - Open device by enumeration index (0-based)
  - `Xr2280x::open_by_path()` - Open device by platform-specific path
  - `Xr2280x::from_hid_device()` - Create instance from existing HidDevice
- **Enhanced Error Handling**: Specific error types for multi-device selection failures
  - `Error::DeviceNotFoundBySerial` - Serial number not found
  - `Error::DeviceNotFoundByIndex` - Index out of range
  - `Error::DeviceNotFoundByPath` - Invalid device path
  - `Error::MultipleDevicesFound` - Ambiguous selection when expecting one device
- **Re-exported Types**: Essential hidapi types now available through the crate
  - `hidapi::DeviceInfo` and `hidapi::HidApi` re-exported for convenience
- **New Example**: `multi_device_selection.rs` demonstrating all selection methods

### Changed
- Refactored device opening logic to use unified `from_hid_device()` method internally
- Updated documentation with comprehensive multi-device selection examples
- Enhanced README with dedicated multi-device selection section

### Fixed
- Improved consistency between different device opening methods

## [0.9.2] - 2024-12-19

### Added
- **Robust I2C Timeout System**: Operation-specific timeout constants for different use cases
  - `timeouts::PROBE` (3ms) - Ultra-fast firmware responsiveness testing
  - `timeouts::SCAN` (8ms) - Fast device discovery with stuck bus protection
  - `timeouts::READ` (100ms) - Standard sensor readings and register access
  - `timeouts::WRITE` (200ms) - Configuration writes to device registers
  - `timeouts::WRITE_READ` (250ms) - Combined write-then-read operations
  - `timeouts::EEPROM_WRITE` (5000ms) - Slow memory operations
- **Custom Timeout Methods**: All I2C operations now have `_with_timeout` variants for precise control
  - `i2c_read_7bit_with_timeout()`, `i2c_write_7bit_with_timeout()`, etc.
- **Specialized Methods**: Pre-configured methods for common device types
  - `i2c_eeprom_write_7bit()` - EEPROM operations with appropriate long timeouts
  - `i2c_scan_with_progress_and_timeout()` - Custom timeout scanning
- **Fast Stuck Bus Detection**: Prevents 29+ second hangs when unpowered devices hold I2C lines low
  - Pre-scan firmware responsiveness test (3ms timeout on reserved address)
  - Consecutive timeout pattern detection with immediate failure
  - Application-level timeout protection independent of HID layer
- **Comprehensive Error Documentation**: New user guides with troubleshooting steps
  - `docs/I2C_ERROR_GUIDE.md` - Complete troubleshooting guide with hardware checklists
  - `docs/I2C_TIMEOUT_IMPROVEMENTS.md` - Technical details of the timeout system
- **Enhanced Examples**: New test programs demonstrating the improvements
  - `examples/i2c_test_stuck_bus.rs` - Verifies fast stuck bus detection
  - `examples/i2c_error_demo.rs` - Shows what each error looks like
  - `examples/i2c_timeouts.rs` - Demonstrates flexible timeout usage

### Changed
- **User-Friendly Error Messages**: All I2C errors now provide clear, actionable guidance
  - `I2cNack`: Explains this is normal during scanning, not an error
  - `I2cTimeout`: Provides specific hardware troubleshooting steps
  - `I2cArbitrationLost`: Explains bus contention and how to resolve it
  - `I2cRequestError`: Points to specific code parameters to check
  - `I2cUnknownError`: Suggests firmware recovery actions
- **Enhanced i2c_scan Example**: Updated with comprehensive error handling and troubleshooting guidance
- **Improved Documentation**: All timeout constants now properly documented with intra-doc links
- **Default Timeouts**: Operations now use appropriate defaults instead of single 500ms timeout
  - Scanning operations: 8ms (vs previous 500ms) - **62x faster**
  - Read operations: 100ms (vs previous 500ms) - **5x faster**
  - Write operations: 200ms (vs previous 500ms) - **2.5x faster**

### Fixed
- **Critical: Eliminated 29+ Second Hangs**: When unpowered I2C devices hold bus lines low
  - **Root Cause**: XR2280x firmware becomes unresponsive, HID timeouts ineffective
  - **Solution**: Pre-scan responsiveness testing catches stuck firmware immediately
  - **Result**: Maximum 3 seconds to detect stuck bus vs previous 29+ second hangs
- **Broken Documentation Links**: Fixed intra-doc links for timeout constants
- **Code Formatting**: Applied consistent Rust formatting across all files

### Performance
- **60x Faster I2C Scanning**: For responsive devices (3-8ms vs 500ms per address)
- **Fast Failure Protection**: Stuck bus detection in <3 seconds vs 29+ second hangs
- **Real-World Impact**: Full bus scan (112 addresses)
  - **Before**: 56 seconds (all timeout) or 29+ second hang (stuck bus)
  - **After**: 0.9 seconds (normal) or <3 seconds (stuck bus detection)

### Technical Details
- **Firmware Responsiveness Testing**: Ultra-fast 3ms probe of reserved address 0x00
- **Pattern-Based Detection**: Fails after 1-2 consecutive timeouts instead of 5+
- **Multi-Layered Protection**: Both HID-level and application-level timeout enforcement
- **Backward Compatibility**: All existing code continues to work with better defaults
- **Module Exports**: Added `timeouts` module to public API for user access

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