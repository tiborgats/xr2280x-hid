//! Internal constants, register addresses, and bit definitions.

// Default Vendor/Product IDs
/// Exar Corporation vendor ID for XR2280x devices.
pub const EXAR_VID: u16 = 0x04E2;

// Default Product IDs for the relevant HID interfaces
// Note: Datasheets suggest these PIDs are the same across models for these interfaces.
/// Product ID for XR2280x I2C interface (common for XR22800/1/2/4).
pub const XR2280X_I2C_PID: u16 = 0x1100; // Common for XR22800/1/2/4
/// Product ID for XR2280x EDGE (GPIO/PWM/Interrupt) interface (common for XR22800/1/2/4).
pub const XR2280X_EDGE_PID: u16 = 0x1200; // Common for XR22800/1/2/4

// --- Feature Reports (Control Transfer) ---
pub const REPORT_ID_WRITE_HID_REGISTER: u8 = 0x3C;
pub const REPORT_ID_SET_HID_READ_ADDRESS: u8 = 0x4B;
pub const REPORT_ID_READ_HID_REGISTER: u8 = 0x5A;

// --- I2C Related Constants ---
pub mod i2c {
    // HID Reports (Interrupt Transfer)
    // Report ID 0x00 is used for I2C_SLAVE_OUT. Implicit in hidapi.
    // Report ID for I2C_SLAVE_IN assumed 0 (implicit).
    pub const REPORT_MAX_DATA_SIZE: usize = 32;
    // Size of buffer passed to hidapi write() (hidapi handles Report ID internally)
    pub const OUT_REPORT_WRITE_BUF_SIZE: usize = 36; // Flags(1) + WrSize(1) + RdSize(1) + SlaveAddr(1) + Data(32)
    // Expected size of buffer received from hidapi read() (includes Report ID byte added by hidapi)
    pub const IN_REPORT_READ_BUF_SIZE: usize = 36; // ReportID(1) + Flags(1) + WrSize(1) + RdSize(1) + Reserved(1) + Data(32)

    // Register Addresses
    pub const REG_SCL_LOW: u16 = 0x0341;
    pub const REG_SCL_HIGH: u16 = 0x0342;

    // I2C_SLAVE_OUT Flags (Byte 0 of OUT report buffer)
    pub mod out_flags {
        /// Generate I2C START condition at beginning of transaction.
        pub const START_BIT: u8 = 1 << 0;
        /// Generate I2C STOP condition at end of transaction.
        pub const STOP_BIT: u8 = 1 << 1;
        /// Send ACK after last read byte (default is NACK).
        #[allow(dead_code)] // Used externally via i2c_transfer_raw
        pub const ACK_LAST_READ: u8 = 1 << 2; // Default is NACK last read
        // Bits 3 reserved
        // Bits 7..4 Sequence number (optional)
    }

    // I2C_SLAVE_IN Status Flags (Byte 0 of IN report buffer)
    pub mod in_flags {
        pub const REQUEST_ERROR: u8 = 1 << 0;
        pub const NAK_RECEIVED: u8 = 1 << 1;
        pub const ARBITRATION_LOST: u8 = 1 << 2;
        pub const TIMEOUT: u8 = 1 << 3;
        // Bits 7..4 Sequence number
    }
}

// --- EDGE (GPIO/PWM/Interrupt) Related Constants ---
pub mod edge {
    // Register Addresses Group 0 (Pins E0-E15 / GPIO 0-15)
    // Note: XR22800/1 only use E0-E7 (bits 0-7) of these registers via HID.
    pub const REG_FUNC_SEL_0: u16 = 0x03C0;
    pub const REG_DIR_0: u16 = 0x03C1;
    pub const REG_SET_0: u16 = 0x03C2;
    pub const REG_CLEAR_0: u16 = 0x03C3;
    pub const REG_STATE_0: u16 = 0x03C4;
    pub const REG_TRI_STATE_0: u16 = 0x03C5;
    pub const REG_OPEN_DRAIN_0: u16 = 0x03C6;
    pub const REG_PULL_UP_0: u16 = 0x03C7;
    pub const REG_PULL_DOWN_0: u16 = 0x03C8;
    pub const REG_INTR_MASK_0: u16 = 0x03C9;
    pub const REG_INTR_POS_EDGE_0: u16 = 0x03CA;
    pub const REG_INTR_NEG_EDGE_0: u16 = 0x03CB;

    // Register Addresses Group 1 (Pins E16-E31 / GPIO 16-31) - XR22802/4 Only
    pub const REG_FUNC_SEL_1: u16 = 0x03CC;
    pub const REG_DIR_1: u16 = 0x03CD;
    pub const REG_SET_1: u16 = 0x03CE;
    pub const REG_CLEAR_1: u16 = 0x03CF;
    pub const REG_STATE_1: u16 = 0x03D0;
    pub const REG_TRI_STATE_1: u16 = 0x03D1;
    pub const REG_OPEN_DRAIN_1: u16 = 0x03D2;
    pub const REG_PULL_UP_1: u16 = 0x03D3;
    pub const REG_PULL_DOWN_1: u16 = 0x03D4;
    pub const REG_INTR_MASK_1: u16 = 0x03D5;
    pub const REG_INTR_POS_EDGE_1: u16 = 0x03D6;
    pub const REG_INTR_NEG_EDGE_1: u16 = 0x03D7;

    // PWM Register Addresses
    pub const REG_PWM0_CTRL: u16 = 0x03D8;
    pub const REG_PWM0_HIGH: u16 = 0x03D9;
    pub const REG_PWM0_LOW: u16 = 0x03DA;
    pub const REG_PWM1_CTRL: u16 = 0x03DB;
    pub const REG_PWM1_HIGH: u16 = 0x03DC;
    pub const REG_PWM1_LOW: u16 = 0x03DD;

    // PWM Control Register Bits/Masks (in EDGE_PWMx_CTRL registers)
    pub mod pwm_ctrl {
        pub const PIN_MASK: u16 = 0b0000_0000_0001_1111; // Bits 4:0
        pub const PIN_SHIFT: u8 = 0;
        pub const ENABLE_MASK: u16 = 0b0000_0000_0010_0000; // Bit 5
        #[allow(dead_code)] // Used implicitly via shift
        pub const ENABLE_SHIFT: u8 = 5;
        pub const CMD_MASK: u16 = 0b0000_0001_1100_0000; // Bits 8:6
        pub const CMD_SHIFT: u8 = 6;

        // Command values
        pub const CMD_IDLE: u16 = 0b000;
        pub const CMD_ASSERT_LOW: u16 = 0b100; // Also 111
        pub const CMD_ONE_SHOT: u16 = 0b101;
        pub const CMD_FREE_RUN: u16 = 0b110;
    }

    // PWM Period Calculation
    // Datasheet: "increments of 266.667ns". This corresponds to 60MHz / 16 = 3.75MHz clock.
    // Period = 1 / 3.75MHz = 266.666... ns
    pub const PWM_UNIT_TIME_NS: f64 = 1_000_000_000.0 / (60_000_000.0 / 16.0); // ~266.667 ns
    pub const PWM_MIN_UNITS: u16 = 1;
    pub const PWM_MAX_UNITS: u16 = 4095;
}
