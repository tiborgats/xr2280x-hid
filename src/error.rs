use crate::i2c::I2cAddress;
use thiserror::Error;
// Removed: use crate::Xr2280x;

/// Errors that can occur when using XR2280x devices.
///
/// This enum covers all possible error conditions that may arise during
/// device communication, I2C operations, GPIO control, and other device
/// interactions.
#[derive(Error, Debug)]
pub enum Error {
    /// Error from the underlying HID API layer.
    #[error("HID API error: {0}")]
    Hid(#[from] hidapi::HidError),
    /// No XR2280x device was found with the specified vendor/product ID.
    #[error("Device not found with specified VID/PID")]
    DeviceNotFound,
    /// No XR2280x device was found with the specified serial number.
    #[error("Device not found with serial number '{serial}': {message}")]
    DeviceNotFoundBySerial {
        /// The serial number that was searched for.
        serial: String,
        /// Additional error details.
        message: String,
    },
    /// No XR2280x device was found at the specified path.
    #[error("Device not found at path '{path}': {message}")]
    DeviceNotFoundByPath {
        /// The device path that was searched for.
        path: String,
        /// Additional error details.
        message: String,
    },
    /// No XR2280x device was found at the specified index.
    #[error("Device not found at index {index}: {message}")]
    DeviceNotFoundByIndex {
        /// The index that was requested.
        index: usize,
        /// Additional error details.
        message: String,
    },
    /// Multiple XR2280x devices were found when only one was expected.
    #[error("Multiple devices found ({count}): {message}")]
    MultipleDevicesFound {
        /// The number of devices that were found.
        count: usize,
        /// Additional context about the ambiguity.
        message: String,
    },
    /// General I/O error during device communication.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Invalid or malformed HID report received from device.
    #[error("Invalid HID report received or unexpected size ({0} bytes)")]
    InvalidReport(usize),
    /// Timeout waiting for device response.
    #[error("Timeout waiting for device response")] // Generic timeout
    Timeout,
    /// Function argument is outside the valid range.
    #[error("Argument out of range: {0}")]
    ArgumentOutOfRange(String),
    /// GPIO pin number is outside the valid range for this device.
    #[error("GPIO pin {pin} argument out of range (0-31): {message}")]
    PinArgumentOutOfRange {
        /// The invalid pin number that was specified.
        pin: u8,
        /// Detailed error message explaining the constraint.
        message: String,
    },
    /// GPIO register read operation failed.
    #[error("GPIO register read failed for pin {pin} (register 0x{register:04X}): {message}")]
    GpioRegisterReadError {
        /// The GPIO pin number that was being accessed.
        pin: u8,
        /// The register address that failed to read.
        register: u16,
        /// Additional error details.
        message: String,
    },
    /// GPIO register write operation failed.
    #[error("GPIO register write failed for pin {pin} (register 0x{register:04X}): {message}")]
    GpioRegisterWriteError {
        /// The GPIO pin number that was being accessed.
        pin: u8,
        /// The register address that failed to write.
        register: u16,
        /// Additional error details.
        message: String,
    },
    /// Invalid GPIO configuration combination.
    #[error("Invalid GPIO configuration for pin {pin}: {message}")]
    GpioConfigurationError {
        /// The GPIO pin number with the invalid configuration.
        pin: u8,
        /// Description of the configuration conflict.
        message: String,
    },
    /// GPIO hardware-specific error.
    #[error("GPIO hardware error on pin {pin}: {message}. Check pin connections and device power.")]
    GpioHardwareError {
        /// The GPIO pin number where the hardware error occurred.
        pin: u8,
        /// Description of the hardware issue.
        message: String,
    },
    /// GPIO write verification failed - pin did not reach expected level.
    #[error(
        "GPIO write verification failed for pin {pin} on attempt {attempt}: expected {expected:?}, but pin reads {actual:?}. This indicates a hardware timing issue or pin conflict."
    )]
    GpioWriteVerificationFailed {
        /// The GPIO pin number that failed verification.
        pin: u8,
        /// The level that was expected after the write.
        expected: crate::gpio::GpioLevel,
        /// The level that was actually read from the pin.
        actual: crate::gpio::GpioLevel,
        /// The attempt number when verification failed.
        attempt: u32,
    },
    /// GPIO operation timed out before completion.
    #[error(
        "GPIO {operation} operation on pin {pin} timed out after {timeout_ms}ms. This may indicate hardware issues or excessive retry delays."
    )]
    GpioOperationTimeout {
        /// The GPIO pin number where the timeout occurred.
        pin: u8,
        /// Description of the operation that timed out.
        operation: String,
        /// The timeout duration in milliseconds.
        timeout_ms: u32,
    },
    /// All GPIO write retry attempts have been exhausted.
    #[error(
        "GPIO write retries exhausted for pin {pin} after {attempts} attempts. Consider increasing retry delay or checking hardware connections."
    )]
    GpioWriteRetriesExhausted {
        /// The GPIO pin number where retries were exhausted.
        pin: u8,
        /// The total number of attempts that were made.
        attempts: u32,
    },
    /// PWM channel configuration error.
    #[error("PWM channel {channel} configuration error: {message}")]
    PwmConfigurationError {
        /// The PWM channel number (0 or 1).
        channel: u8,
        /// Description of the configuration issue.
        message: String,
    },
    /// PWM parameter validation error.
    #[error("PWM parameter validation failed for channel {channel}: {message}")]
    PwmParameterError {
        /// The PWM channel number.
        channel: u8,
        /// Description of the parameter issue.
        message: String,
    },
    /// PWM hardware-specific error.
    #[error(
        "PWM hardware error on channel {channel}: {message}. Check device capabilities and pin assignments."
    )]
    PwmHardwareError {
        /// The PWM channel number where the error occurred.
        channel: u8,
        /// Description of the hardware issue.
        message: String,
    },
    /// I2C slave device responded with NACK (not acknowledged).
    #[error(
        "No device found at I2C address {address}: Device did not acknowledge (NACK). This is normal when scanning for devices."
    )]
    I2cNack {
        /// The I2C address that sent the NACK.
        address: I2cAddress,
    },
    /// I2C bus arbitration was lost during transaction.
    #[error(
        "I2C bus conflict at address {address}: Arbitration lost (multiple masters competing for bus control). Check for other I2C controllers, loose connections, or electrical interference. Try disconnecting other devices and retrying."
    )]
    I2cArbitrationLost {
        /// The I2C address being accessed when arbitration was lost.
        address: I2cAddress,
    },
    /// I2C bus timeout occurred during transaction.
    #[error(
        "I2C timeout at address {address}: Device did not respond within timeout period. This may indicate: stuck bus (unpowered device holding lines low), very slow device, or hardware issues. Check device power and connections."
    )]
    I2cTimeout {
        /// The I2C address being accessed when timeout occurred.
        address: I2cAddress,
    }, // Keep specific I2C timeout
    /// I2C transaction failed due to invalid request parameters.
    #[error(
        "I2C request error at address {address}: Invalid parameters sent to XR2280x firmware. Check data length (max 32 bytes), address validity, and operation flags."
    )]
    I2cRequestError {
        /// The I2C address being accessed when the error occurred.
        address: I2cAddress,
    },
    /// I2C transaction failed with unknown error condition.
    #[error(
        "I2C unknown error at address {address}: Unexpected condition reported by XR2280x firmware (Status: 0x{flags:02X}). This may indicate firmware issues or unsupported operation. Try power cycling the XR2280x device."
    )]
    I2cUnknownError {
        /// The I2C address being accessed when the error occurred.
        address: I2cAddress,
        /// Raw status flags from the device indicating the error condition.
        flags: u8,
    },

    /// Provided buffer is smaller than required for the operation.
    #[error("Provided buffer is too small (expected at least {expected}, got {actual})")]
    BufferTooSmall {
        /// Minimum required buffer size.
        expected: usize,
        /// Actual buffer size provided.
        actual: usize,
    },
    /// Requested operation exceeds device or protocol limits.
    #[error("Requested operation size is too large (max {max}, got {actual})")]
    OperationTooLarge {
        /// Maximum allowed size for this operation.
        max: usize,
        /// Actual size requested.
        actual: usize,
    },
    /// Feature is not supported by this device model.
    #[error("Feature not supported by this chip model: {0}")]
    UnsupportedFeature(String),
    /// Invalid 10-bit I2C address specified.
    #[error("Invalid I2C 10-bit address: {0:04X}")]
    InvalidI2c10BitAddress(u16),
    /// Failed to parse GPIO interrupt report from device.
    #[error("GPIO Interrupt report parsing failed: {0}")]
    InterruptParseError(String),
}

/// Result type alias for XR2280x operations.
///
/// This is a convenience alias for `std::result::Result<T, Error>` used
/// throughout the crate to reduce boilerplate.
pub type Result<T> = std::result::Result<T, Error>;

// Removed the impl Xr2280x block containing map_feature_err

// Helpers for creating specific UnsupportedFeature errors
pub(crate) fn unsupported_gpio_group1() -> Error {
    Error::UnsupportedFeature("GPIO Group 1 (pins 8-31) requires XR22802/XR22804".to_string())
}
pub(crate) fn unsupported_pwm_pin(pin: u8) -> Error {
    Error::UnsupportedFeature(format!(
        "Assigning PWM to pin {pin} requires XR22802/XR22804 (XR22800/1 only support pins 0-7)"
    ))
}

// Helpers for creating specific GPIO errors
pub(crate) fn gpio_register_read_error(pin: u8, register: u16, message: String) -> Error {
    Error::GpioRegisterReadError {
        pin,
        register,
        message,
    }
}

pub(crate) fn gpio_register_write_error(pin: u8, register: u16, message: String) -> Error {
    Error::GpioRegisterWriteError {
        pin,
        register,
        message,
    }
}

// Helpers for creating specific PWM errors

pub(crate) fn pwm_parameter_error(channel: u8, message: String) -> Error {
    Error::PwmParameterError { channel, message }
}

pub(crate) fn pwm_hardware_error(channel: u8, message: String) -> Error {
    Error::PwmHardwareError { channel, message }
}
