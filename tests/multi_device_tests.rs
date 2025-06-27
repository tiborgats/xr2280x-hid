//! Integration tests for multi-device selection functionality.
//!
//! These tests verify the new multi-device selection methods work correctly
//! with actual XR2280x hardware. Most tests are marked with #[ignore] and
//! require hardware to be connected.

use hidapi::HidApi;
use std::ffi::CString;
use xr2280x_hid::{Error, Result, Xr2280x};

/// Helper function to get HidApi instance for testing
fn get_hid_api() -> HidApi {
    HidApi::new().expect("Failed to initialize HID API")
}

#[test]
fn test_enumerate_devices_no_panic() {
    // This test should never panic, even without hardware
    let hid_api = get_hid_api();
    let result = Xr2280x::device_enumerate(&hid_api);

    // Should succeed regardless of whether devices are present
    assert!(result.is_ok());

    let devices = result.unwrap();
    println!("Found {} XR2280x hardware devices", devices.len());

    // Basic validation of hardware device info structure
    for (i, device_info) in devices.iter().enumerate() {
        println!(
            "Hardware Device {}: VID=0x{:04X}, Serial={:?}",
            i, device_info.vid, device_info.serial_number
        );
        assert_eq!(device_info.vid, xr2280x_hid::EXAR_VID);

        // Check that at least one interface is available
        assert!(device_info.i2c_interface.is_some() || device_info.edge_interface.is_some());
    }
}

#[test]
fn test_open_by_index_invalid_indices() {
    let hid_api = get_hid_api();
    let devices = Xr2280x::device_enumerate(&hid_api).unwrap();

    // Test opening with out-of-range index
    let invalid_index = devices.len() + 10;
    let result = Xr2280x::open_by_index(&hid_api, invalid_index);

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::DeviceNotFoundByIndex { index, message } => {
            assert_eq!(index, invalid_index);
            assert!(message.contains("Index out of range"));
        }
        e => panic!("Expected DeviceNotFoundByIndex error, got: {e:?}"),
    }
}

#[test]
fn test_open_by_serial_nonexistent() {
    let hid_api = get_hid_api();

    // Try to open device with non-existent serial number
    let fake_serial = "DEFINITELY_NONEXISTENT_SERIAL_12345";
    let result = Xr2280x::open_by_serial(&hid_api, fake_serial);

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::DeviceNotFoundBySerial { serial, message } => {
            assert_eq!(serial, fake_serial);
            assert!(message.contains("No XR2280x device found"));
        }
        e => panic!("Expected DeviceNotFoundBySerial error, got: {e:?}"),
    }
}

#[test]
fn test_open_by_path_invalid() {
    let hid_api = get_hid_api();

    // Try to open device with invalid path
    let invalid_path = CString::new("/dev/nonexistent_hidraw999").unwrap();
    let result = Xr2280x::open_by_path(&hid_api, &invalid_path);

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::DeviceNotFoundByPath { path, message: _ } => {
            assert!(path.contains("nonexistent_hidraw999"));
        }
        Error::Hid(_) => {
            // hidapi might return HidError instead, which is also acceptable
        }
        e => panic!("Expected DeviceNotFoundByPath or Hid error, got: {e:?}"),
    }
}

#[test]
#[ignore] // Requires hardware
fn test_enumerate_and_open_by_index() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::device_enumerate(&hid_api)?;

    if devices.is_empty() {
        println!("No XR2280x hardware devices found. Skipping hardware test.");
        return Ok(());
    }

    println!(
        "Found {} XR2280x hardware devices for testing",
        devices.len()
    );

    // Test opening each device by index
    for (index, device_info) in devices.iter().enumerate() {
        println!(
            "Testing hardware device {} - VID: 0x{:04X}, Serial: {:?}",
            index,
            device_info.vid,
            device_info.serial_number.as_deref().unwrap_or("N/A")
        );

        let device = Xr2280x::open_by_index(&hid_api, index)?;
        let opened_info = device.get_device_info();

        // Verify the opened device matches what we expected
        assert_eq!(opened_info.vendor_id, device_info.vid);
        assert_eq!(opened_info.serial_number, device_info.serial_number);

        // Verify capabilities are detected
        let capabilities = device.get_capabilities();
        assert!(capabilities.gpio_count == 8 || capabilities.gpio_count == 32);

        println!(
            "  ✓ Successfully opened device with {} GPIO pins",
            capabilities.gpio_count
        );
    }

    Ok(())
}

#[ignore]
#[test]
fn test_open_by_serial_number() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::device_enumerate(&hid_api)?;

    if devices.is_empty() {
        println!("No XR2280x hardware devices found. Skipping hardware test.");
        return Ok(());
    }

    for device_info in devices {
        if let Some(serial) = &device_info.serial_number {
            println!("Testing serial number: {serial}");

            // Open by serial number
            let device = Xr2280x::open_by_serial(&hid_api, serial)?;
            let opened_info = device.get_device_info();

            // Verify serial number matches
            assert_eq!(opened_info.serial_number.as_deref(), Some(serial.as_str()));

            println!("  Successfully opened device with serial: {serial}");
        } else {
            println!("Device has no serial number, skipping serial test");
        }
    }

    Ok(())
}

#[ignore]
#[test]
fn test_open_by_path() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::device_enumerate(&hid_api)?;

    if devices.is_empty() {
        println!("No XR2280x hardware devices found. Skipping hardware test.");
        return Ok(());
    }

    for device_info in devices {
        // Test opening by I2C interface path if available
        if let Some(i2c_interface) = &device_info.i2c_interface {
            let path = &i2c_interface.path;
            println!("Testing I2C interface path: {path:?}");

            let device = Xr2280x::open_by_path(&hid_api, path)?;
            let opened_info = device.get_device_info();

            // Basic validation
            assert_eq!(opened_info.vendor_id, device_info.vid);

            println!("  Successfully opened device at I2C path: {path:?}");
        }

        // Test opening by EDGE interface path if available
        if let Some(edge_interface) = &device_info.edge_interface {
            let path = &edge_interface.path;
            println!("Testing EDGE interface path: {path:?}");

            let device = Xr2280x::open_by_path(&hid_api, path)?;
            let opened_info = device.get_device_info();

            // Basic validation
            assert_eq!(opened_info.vendor_id, device_info.vid);

            println!("  Successfully opened device at EDGE path: {path:?}");
        }
    }

    Ok(())
}

#[test]
#[ignore] // Requires hardware
fn test_consistency_between_methods() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::device_enumerate(&hid_api)?;

    if devices.is_empty() {
        println!("No XR2280x hardware devices found. Skipping hardware test.");
        return Ok(());
    }

    let device_info = &devices[0];

    // Open the same device using different methods
    let device1 = Xr2280x::open_by_index(&hid_api, 0)?;

    // Compare device info and capabilities
    let info1 = device1.get_device_info();
    let cap1 = device1.get_capabilities();

    // If device has serial number, test serial opening too
    if let Some(serial) = &device_info.serial_number {
        let device2 = Xr2280x::open_by_serial(&hid_api, serial)?;
        let info2 = device2.get_device_info();
        let cap2 = device2.get_capabilities();

        assert_eq!(info1.vendor_id, info2.vendor_id);
        assert_eq!(info1.serial_number, info2.serial_number);
        assert_eq!(cap1.gpio_count, cap2.gpio_count);

        println!("✓ Index and serial opening methods return consistent information");
    } else {
        println!("Device has no serial number, skipping serial consistency test");
    }

    println!("✓ Hardware device opening methods are consistent");
    Ok(())
}

#[test]
#[ignore] // Requires hardware
fn test_multiple_device_handling() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::device_enumerate(&hid_api)?;

    println!("Testing with {} hardware devices", devices.len());

    if devices.is_empty() {
        println!("No XR2280x hardware devices found. Test passes trivially.");
        return Ok(());
    }

    if devices.len() == 1 {
        println!("Only one hardware device found. Testing single device scenario.");

        // Test that index 0 works
        let device = Xr2280x::open_by_index(&hid_api, 0)?;
        let _info = device.get_device_info();

        // Test that index 1 fails appropriately
        let result = Xr2280x::open_by_index(&hid_api, 1);
        assert!(matches!(result, Err(Error::DeviceNotFoundByIndex { .. })));

        println!("✓ Single hardware device scenario handled correctly");
    } else {
        println!("Multiple hardware devices found. Testing multi-device scenario.");

        // Test that we can open each device by index
        for i in 0..devices.len() {
            let device = Xr2280x::open_by_index(&hid_api, i)?;
            let _info = device.get_device_info();
            println!("  ✓ Hardware device {i} opened successfully");
        }

        // Test that out-of-range index fails
        let result = Xr2280x::open_by_index(&hid_api, devices.len());
        assert!(matches!(result, Err(Error::DeviceNotFoundByIndex { .. })));

        println!("✓ Multi-device scenario handled correctly");
    }

    Ok(())
}

#[test]
#[ignore] // Requires hardware
fn test_hardware_device_interfaces() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::device_enumerate(&hid_api)?;

    if devices.is_empty() {
        println!("No XR2280x hardware devices found. Skipping interface test.");
        return Ok(());
    }

    for (index, device_info) in devices.iter().enumerate() {
        println!("Testing hardware device {index}");

        let device = Xr2280x::device_open(&hid_api, device_info)?;
        let capabilities = device.get_capabilities();

        println!("  Serial: {:?}", device_info.serial_number);
        println!("  GPIO Count: {}", capabilities.gpio_count);
        println!("  I2C Interface: {}", device_info.i2c_interface.is_some());
        println!("  EDGE Interface: {}", device_info.edge_interface.is_some());

        // Test I2C functionality if available
        if device_info.i2c_interface.is_some() {
            match device.i2c_set_speed_khz(100) {
                Ok(()) => println!("  ✓ I2C functionality works"),
                Err(e) => println!("  ⚠ I2C functionality failed: {e}"),
            }
        }

        // Test basic GPIO functionality if available
        if device_info.edge_interface.is_some() {
            if let Ok(pin) = xr2280x_hid::gpio::GpioPin::new(0) {
                match device.gpio_read(pin) {
                    Ok(level) => println!("  ✓ GPIO read works: {level:?}"),
                    Err(_) => println!("  ⚠ GPIO read failed (may need configuration)"),
                }
            }
        }
    }

    Ok(())
}
