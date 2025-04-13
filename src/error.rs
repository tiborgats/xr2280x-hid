use crate::i2c::I2cAddress;
use thiserror::Error;
// Removed: use crate::Xr2280x;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HID API error: {0}")]
    Hid(#[from] hidapi::HidError),
    #[error("Device not found with specified VID/PID")]
    DeviceNotFound,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid HID report received or unexpected size ({0} bytes)")]
    InvalidReport(usize),
    #[error("Timeout waiting for device response")] // Generic timeout
    Timeout,
    #[error("Argument out of range: {0}")]
    ArgumentOutOfRange(String),
    #[error("GPIO pin {pin} argument out of range (0-31): {message}")]
    PinArgumentOutOfRange { pin: u8, message: String },
    #[error("I2C transaction aborted for address {address:?}: NACK received from slave")]
    I2cNack { address: I2cAddress },
    #[error("I2C transaction aborted for address {address:?}: Arbitration Lost")]
    I2cArbitrationLost { address: I2cAddress },
    #[error("I2C transaction aborted for address {address:?}: Bus Timeout")]
    I2cTimeout { address: I2cAddress }, // Keep specific I2C timeout
    #[error("I2C transaction failed for address {address:?}: Invalid request from host (check arguments)")]
    I2cRequestError { address: I2cAddress },
    #[error(
        "I2C transaction failed for address {address:?}: Unknown error (Status Flags: {flags:02X})"
    )]
    I2cUnknownError { address: I2cAddress, flags: u8 },
    #[error("Feature report error (e.g., incorrect length, device error) while accessing register 0x{reg_addr:04X}")]
    FeatureReportError { reg_addr: u16 },
    #[error("Provided buffer is too small (expected at least {expected}, got {actual})")]
    BufferTooSmall { expected: usize, actual: usize },
    #[error("Requested operation size is too large (max {max}, got {actual})")]
    OperationTooLarge { max: usize, actual: usize },
    #[error("Feature not supported by this chip model: {0}")]
    UnsupportedFeature(String),
    #[error("Invalid I2C 10-bit address: {0:04X}")]
    InvalidI2c10BitAddress(u16),
    #[error("GPIO Interrupt report parsing failed: {0}")]
    InterruptParseError(String),
}

// Custom result type alias
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
