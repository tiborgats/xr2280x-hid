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
    /// I2C slave device responded with NACK (not acknowledged).
    #[error("I2C transaction aborted for address {address:?}: NACK received from slave")]
    I2cNack {
        /// The I2C address that sent the NACK.
        address: I2cAddress,
    },
    /// I2C bus arbitration was lost during transaction.
    #[error("I2C transaction aborted for address {address:?}: Arbitration Lost")]
    I2cArbitrationLost {
        /// The I2C address being accessed when arbitration was lost.
        address: I2cAddress,
    },
    /// I2C bus timeout occurred during transaction.
    #[error("I2C transaction aborted for address {address:?}: Bus Timeout")]
    I2cTimeout {
        /// The I2C address being accessed when timeout occurred.
        address: I2cAddress,
    }, // Keep specific I2C timeout
    /// I2C transaction failed due to invalid request parameters.
    #[error("I2C transaction failed for address {address:?}: Invalid request from host (check arguments)")]
    I2cRequestError {
        /// The I2C address being accessed when the error occurred.
        address: I2cAddress,
    },
    /// I2C transaction failed with unknown error condition.
    #[error(
        "I2C transaction failed for address {address:?}: Unknown error (Status Flags: {flags:02X})"
    )]
    I2cUnknownError {
        /// The I2C address being accessed when the error occurred.
        address: I2cAddress,
        /// Raw status flags from the device indicating the error condition.
        flags: u8,
    },
    /// HID feature report operation failed.
    #[error("Feature report error (e.g., incorrect length, device error) while accessing register 0x{reg_addr:04X}")]
    FeatureReportError {
        /// The register address that was being accessed.
        reg_addr: u16,
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
        "Assigning PWM to pin {} requires XR22802/XR22804 (XR22800/1 only support pins 0-7)",
        pin
    ))
}
