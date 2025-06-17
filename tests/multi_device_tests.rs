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

/// Helper function to check if any XR2280x devices are available
fn has_xr2280x_devices(hid_api: &HidApi) -> bool {
    !Xr2280x::enumerate_devices(hid_api)
        .unwrap_or_default()
        .is_empty()
}

#[test]
fn test_enumerate_devices_no_panic() {
    // This test should never panic, even without hardware
    let hid_api = get_hid_api();
    let result = Xr2280x::enumerate_devices(&hid_api);

    // Should succeed regardless of whether devices are present
    assert!(result.is_ok());

    let devices = result.unwrap();
    println!("Found {} XR2280x devices", devices.len());

    // Verify all returned devices have correct VID
    for device in &devices {
        assert_eq!(device.vendor_id(), xr2280x_hid::EXAR_VID);
        assert!(
            device.product_id() == xr2280x_hid::XR2280X_I2C_PID
                || device.product_id() == xr2280x_hid::XR2280X_EDGE_PID
        );
    }
}

#[test]
fn test_open_by_index_invalid_indices() {
    let hid_api = get_hid_api();
    let devices = Xr2280x::enumerate_devices(&hid_api).unwrap();

    // Test opening with out-of-range index
    let invalid_index = devices.len() + 10;
    let result = Xr2280x::open_by_index(&hid_api, invalid_index);

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::DeviceNotFoundByIndex { index, message } => {
            assert_eq!(index, invalid_index);
            assert!(message.contains("out of range"));
        }
        e => panic!("Expected DeviceNotFoundByIndex error, got: {:?}", e),
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
        e => panic!("Expected DeviceNotFoundBySerial error, got: {:?}", e),
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
        e => panic!("Expected DeviceNotFoundByPath or Hid error, got: {:?}", e),
    }
}

#[test]
#[ignore] // Requires hardware
fn test_enumerate_and_open_by_index() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::enumerate_devices(&hid_api)?;

    if devices.is_empty() {
        println!("No XR2280x devices found. Skipping hardware test.");
        return Ok(());
    }

    println!("Found {} XR2280x devices for testing", devices.len());

    // Test opening each device by index
    for (index, device_info) in devices.iter().enumerate() {
        println!(
            "Testing device {} - PID: 0x{:04X}, Serial: {:?}",
            index,
            device_info.product_id(),
            device_info.serial_number()
        );

        let device = Xr2280x::open_by_index(&hid_api, index)?;
        let opened_info = device.get_device_info()?;

        // Verify the opened device matches what we expected
        assert_eq!(opened_info.vendor_id, device_info.vendor_id());
        assert_eq!(opened_info.product_id, device_info.product_id());

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

#[test]
#[ignore] // Requires hardware
fn test_open_by_serial_number() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::enumerate_devices(&hid_api)?;

    if devices.is_empty() {
        println!("No XR2280x devices found. Skipping hardware test.");
        return Ok(());
    }

    for device_info in devices {
        if let Some(serial) = device_info.serial_number() {
            println!("Testing open by serial number: '{}'", serial);

            let device = Xr2280x::open_by_serial(&hid_api, serial)?;
            let opened_info = device.get_device_info()?;

            // Verify the opened device has the correct serial number
            assert_eq!(opened_info.serial_number.as_deref(), Some(serial));
            assert_eq!(opened_info.vendor_id, device_info.vendor_id());
            assert_eq!(opened_info.product_id, device_info.product_id());

            println!("  ✓ Successfully opened device by serial number");
        } else {
            println!("Device has no serial number, skipping serial test");
        }
    }

    Ok(())
}

#[test]
#[ignore] // Requires hardware
fn test_open_by_path() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::enumerate_devices(&hid_api)?;

    if devices.is_empty() {
        println!("No XR2280x devices found. Skipping hardware test.");
        return Ok(());
    }

    for device_info in devices {
        let path = device_info.path();
        println!("Testing open by path: {:?}", path);

        let device = Xr2280x::open_by_path(&hid_api, path)?;
        let opened_info = device.get_device_info()?;

        // Verify the opened device matches expected characteristics
        assert_eq!(opened_info.vendor_id, device_info.vendor_id());
        assert_eq!(opened_info.product_id, device_info.product_id());

        println!("  ✓ Successfully opened device by path");
    }

    Ok(())
}

#[test]
#[ignore] // Requires hardware
fn test_from_hid_device() -> Result<()> {
    let hid_api = get_hid_api();

    if !has_xr2280x_devices(&hid_api) {
        println!("No XR2280x devices found. Skipping hardware test.");
        return Ok(());
    }

    // Open a device using the traditional method
    let device_info = xr2280x_hid::find_first(&hid_api)?;
    let hid_device = hid_api.open_path(&device_info.path)?;

    // Create Xr2280x instance using from_hid_device
    let xr_device = Xr2280x::from_hid_device(hid_device)?;

    // Verify the device was properly initialized
    let info = xr_device.get_device_info()?;
    assert_eq!(info.vendor_id, xr2280x_hid::EXAR_VID);
    assert!(
        info.product_id == xr2280x_hid::XR2280X_I2C_PID
            || info.product_id == xr2280x_hid::XR2280X_EDGE_PID
    );

    // Verify capabilities are detected
    let capabilities = xr_device.get_capabilities();
    assert!(capabilities.gpio_count == 8 || capabilities.gpio_count == 32);

    println!("✓ Successfully created Xr2280x from HidDevice");
    Ok(())
}

#[test]
#[ignore] // Requires hardware
fn test_consistency_between_methods() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::enumerate_devices(&hid_api)?;

    if devices.is_empty() {
        println!("No XR2280x devices found. Skipping hardware test.");
        return Ok(());
    }

    let device_info = devices[0];

    // Open the same device using different methods
    let device1 = Xr2280x::open_by_index(&hid_api, 0)?;
    let device2 = Xr2280x::open_by_path(&hid_api, device_info.path())?;

    // Compare device info
    let info1 = device1.get_device_info()?;
    let info2 = device2.get_device_info()?;

    assert_eq!(info1.vendor_id, info2.vendor_id);
    assert_eq!(info1.product_id, info2.product_id);
    assert_eq!(info1.serial_number, info2.serial_number);

    // Compare capabilities
    let cap1 = device1.get_capabilities();
    let cap2 = device2.get_capabilities();

    assert_eq!(cap1.gpio_count, cap2.gpio_count);

    // If device has serial number, test serial opening too
    if let Some(serial) = device_info.serial_number() {
        let device3 = Xr2280x::open_by_serial(&hid_api, serial)?;
        let info3 = device3.get_device_info()?;
        let cap3 = device3.get_capabilities();

        assert_eq!(info1.vendor_id, info3.vendor_id);
        assert_eq!(info1.product_id, info3.product_id);
        assert_eq!(info1.serial_number, info3.serial_number);
        assert_eq!(cap1.gpio_count, cap3.gpio_count);
    }

    println!("✓ All opening methods return consistent device information");
    Ok(())
}

#[test]
#[ignore] // Requires hardware
fn test_multiple_device_handling() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::enumerate_devices(&hid_api)?;

    println!("Testing with {} devices", devices.len());

    if devices.is_empty() {
        println!("No XR2280x devices found. Test passes trivially.");
        return Ok(());
    }

    if devices.len() == 1 {
        println!("Only one device found. Testing single device scenario.");

        // Test that index 0 works
        let device = Xr2280x::open_by_index(&hid_api, 0)?;
        let _info = device.get_device_info()?;

        // Test that index 1 fails appropriately
        let result = Xr2280x::open_by_index(&hid_api, 1);
        assert!(matches!(result, Err(Error::DeviceNotFoundByIndex { .. })));

        println!("✓ Single device scenario handled correctly");
    } else {
        println!("Multiple devices found. Testing multi-device scenario.");

        // Test that we can open each device by index
        for i in 0..devices.len() {
            let device = Xr2280x::open_by_index(&hid_api, i)?;
            let _info = device.get_device_info()?;
            println!("  ✓ Device {} opened successfully", i);
        }

        // Test that out-of-range index fails
        let result = Xr2280x::open_by_index(&hid_api, devices.len());
        assert!(matches!(result, Err(Error::DeviceNotFoundByIndex { .. })));

        println!("✓ Multi-device scenario handled correctly");
    }

    Ok(())
}

#[test]
#[ignore] // Requires hardware with specific setup
fn test_device_interface_detection() -> Result<()> {
    let hid_api = get_hid_api();
    let devices = Xr2280x::enumerate_devices(&hid_api)?;

    if devices.is_empty() {
        println!("No XR2280x devices found. Skipping interface detection test.");
        return Ok(());
    }

    for (index, _device_info) in devices.iter().enumerate() {
        let device = Xr2280x::open_by_index(&hid_api, index)?;
        let info = device.get_device_info()?;

        match info.product_id {
            xr2280x_hid::XR2280X_I2C_PID => {
                println!("Device {} is I2C interface", index);
                // Test I2C-specific functionality
                match device.i2c_set_speed_khz(100) {
                    Ok(()) => println!("  ✓ I2C speed setting works"),
                    Err(e) => println!("  ⚠ I2C speed setting failed: {}", e),
                }
            }
            xr2280x_hid::XR2280X_EDGE_PID => {
                println!("Device {} is EDGE interface", index);
                let capabilities = device.get_capabilities();
                println!("  ✓ Supports {} GPIO pins", capabilities.gpio_count);

                // Test basic GPIO read (may fail if pin not configured, which is fine)
                if let Ok(pin) = xr2280x_hid::gpio::GpioPin::new(0) {
                    match device.gpio_read(pin) {
                        Ok(level) => println!("  ✓ GPIO pin 0 read: {:?}", level),
                        Err(_) => println!("  ⚠ GPIO pin 0 read failed (may need configuration)"),
                    }
                }
            }
            _ => {
                println!(
                    "Device {} has unexpected PID: 0x{:04X}",
                    index, info.product_id
                );
            }
        }
    }

    Ok(())
}
