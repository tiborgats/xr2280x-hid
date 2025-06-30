//! Unit tests for GPIO write reliability features
//!
//! These tests verify the new GPIO write verification and retry functionality
//! that addresses the critical reliability issue where gpio_write() operations
//! return Ok(()) but fail to actually set the hardware GPIO pin.

use std::time::Duration;
use xr2280x_hid::gpio::{GpioLevel, GpioPin, GpioWriteConfig};

#[test]
fn test_gpio_write_config_default() {
    let config = GpioWriteConfig::default();

    assert!(!config.verify_writes);
    assert_eq!(config.retry_attempts, 0);
    assert_eq!(config.retry_delay, Duration::from_millis(10));
    assert_eq!(config.operation_timeout, Duration::from_millis(1000));
}

#[test]
fn test_gpio_write_config_reliable() {
    let config = GpioWriteConfig::reliable();

    assert!(config.verify_writes);
    assert_eq!(config.retry_attempts, 3);
    assert_eq!(config.retry_delay, Duration::from_millis(20));
    assert_eq!(config.operation_timeout, Duration::from_millis(1000));
}

#[test]
fn test_gpio_write_config_fast() {
    let config = GpioWriteConfig::fast();

    // Fast mode should be identical to default
    let default_config = GpioWriteConfig::default();
    assert_eq!(config.verify_writes, default_config.verify_writes);
    assert_eq!(config.retry_attempts, default_config.retry_attempts);
    assert_eq!(config.retry_delay, default_config.retry_delay);
    assert_eq!(config.operation_timeout, default_config.operation_timeout);
}

#[test]
fn test_gpio_write_config_custom() {
    let config = GpioWriteConfig {
        verify_writes: true,
        retry_attempts: 5,
        retry_delay: Duration::from_millis(50),
        operation_timeout: Duration::from_millis(2000),
    };

    assert!(config.verify_writes);
    assert_eq!(config.retry_attempts, 5);
    assert_eq!(config.retry_delay, Duration::from_millis(50));
    assert_eq!(config.operation_timeout, Duration::from_millis(2000));
}

#[test]
fn test_gpio_write_config_clone() {
    let original = GpioWriteConfig::reliable();
    let cloned = original.clone();

    assert_eq!(original.verify_writes, cloned.verify_writes);
    assert_eq!(original.retry_attempts, cloned.retry_attempts);
    assert_eq!(original.retry_delay, cloned.retry_delay);
    assert_eq!(original.operation_timeout, cloned.operation_timeout);
}

// Hardware integration tests (require actual device)
#[cfg(test)]
mod hardware_tests {
    use super::*;
    use hidapi::HidApi;
    use std::time::Instant;
    use xr2280x_hid::{Xr2280x, gpio::GpioPull};

    // Helper to open test device
    fn open_test_device() -> Option<Xr2280x> {
        let hid_api = HidApi::new().ok()?;
        Xr2280x::device_open_first(&hid_api).ok()
    }

    #[test]
    #[ignore] // Requires hardware
    fn test_gpio_write_config_management() {
        let device = match open_test_device() {
            Some(d) => d,
            None => {
                println!("No XR2280x device found, skipping hardware test");
                return;
            }
        };

        // Test default configuration
        let default_config = device.gpio_get_write_config();
        assert!(!default_config.verify_writes);
        assert_eq!(default_config.retry_attempts, 0);

        // Test setting verification
        device.gpio_set_write_verification(true).unwrap();
        let config = device.gpio_get_write_config();
        assert!(config.verify_writes);

        // Test setting retry config
        device
            .gpio_set_retry_config(3, Duration::from_millis(30))
            .unwrap();
        let config = device.gpio_get_write_config();
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay, Duration::from_millis(30));

        // Test setting complete config
        let custom_config = GpioWriteConfig {
            verify_writes: true,
            retry_attempts: 5,
            retry_delay: Duration::from_millis(100),
            operation_timeout: Duration::from_millis(2000),
        };

        device.gpio_set_write_config(custom_config.clone()).unwrap();
        let retrieved_config = device.gpio_get_write_config();

        assert_eq!(retrieved_config.verify_writes, custom_config.verify_writes);
        assert_eq!(
            retrieved_config.retry_attempts,
            custom_config.retry_attempts
        );
        assert_eq!(retrieved_config.retry_delay, custom_config.retry_delay);
        assert_eq!(
            retrieved_config.operation_timeout,
            custom_config.operation_timeout
        );
    }

    #[test]
    #[ignore] // Requires hardware
    fn test_gpio_write_fast_vs_verified() {
        let device = match open_test_device() {
            Some(d) => d,
            None => {
                println!("No XR2280x device found, skipping hardware test");
                return;
            }
        };

        let pin = match GpioPin::new(0) {
            Ok(p) => p,
            Err(_) => {
                println!("Failed to create test pin");
                return;
            }
        };

        if pin.number() >= device.get_capabilities().gpio_count {
            println!("Pin {} not supported on this device", pin.number());
            return;
        }

        // Setup pin as output
        device.gpio_assign_to_edge(pin).unwrap();
        device
            .gpio_setup_output(pin, GpioLevel::Low, GpioPull::None)
            .unwrap();

        // Test fast write
        let start = Instant::now();
        device.gpio_write_fast(pin, GpioLevel::High).unwrap();
        device.gpio_write_fast(pin, GpioLevel::Low).unwrap();
        let fast_duration = start.elapsed();

        // Test verified write
        let start = Instant::now();
        device.gpio_write_verified(pin, GpioLevel::High).unwrap();
        device.gpio_write_verified(pin, GpioLevel::Low).unwrap();
        let verified_duration = start.elapsed();

        println!("Fast write duration: {fast_duration:?}");
        println!("Verified write duration: {verified_duration:?}");

        // Verified writes should be slower due to readback
        assert!(verified_duration > fast_duration);
    }

    #[test]
    #[ignore] // Requires hardware
    fn test_gpio_write_verification_accuracy() {
        let device = match open_test_device() {
            Some(d) => d,
            None => {
                println!("No XR2280x device found, skipping hardware test");
                return;
            }
        };

        let pin = match GpioPin::new(0) {
            Ok(p) => p,
            Err(_) => {
                println!("Failed to create test pin");
                return;
            }
        };

        if pin.number() >= device.get_capabilities().gpio_count {
            println!("Pin {} not supported on this device", pin.number());
            return;
        }

        // Setup pin as output
        device.gpio_assign_to_edge(pin).unwrap();
        device
            .gpio_setup_output(pin, GpioLevel::Low, GpioPull::None)
            .unwrap();

        // Test verified write with manual verification
        device.gpio_write_verified(pin, GpioLevel::High).unwrap();
        let actual_high = device.gpio_read(pin).unwrap();
        assert_eq!(actual_high, GpioLevel::High);

        device.gpio_write_verified(pin, GpioLevel::Low).unwrap();
        let actual_low = device.gpio_read(pin).unwrap();
        assert_eq!(actual_low, GpioLevel::Low);
    }

    #[test]
    #[ignore] // Requires hardware
    fn test_gpio_write_retry_behavior() {
        let device = match open_test_device() {
            Some(d) => d,
            None => {
                println!("No XR2280x device found, skipping hardware test");
                return;
            }
        };

        let pin = match GpioPin::new(0) {
            Ok(p) => p,
            Err(_) => {
                println!("Failed to create test pin");
                return;
            }
        };

        if pin.number() >= device.get_capabilities().gpio_count {
            println!("Pin {} not supported on this device", pin.number());
            return;
        }

        // Setup pin as output
        device.gpio_assign_to_edge(pin).unwrap();
        device
            .gpio_setup_output(pin, GpioLevel::Low, GpioPull::None)
            .unwrap();

        // Configure for retries
        let retry_config = GpioWriteConfig {
            verify_writes: true,
            retry_attempts: 3,
            retry_delay: Duration::from_millis(10),
            operation_timeout: Duration::from_millis(1000),
        };

        device.gpio_set_write_config(retry_config).unwrap();

        // Test multiple consecutive operations
        for i in 0..10 {
            let level = if i % 2 == 0 {
                GpioLevel::High
            } else {
                GpioLevel::Low
            };

            match device.gpio_write(pin, level) {
                Ok(()) => {
                    // Verify the write actually worked
                    let actual = device.gpio_read(pin).unwrap();
                    assert_eq!(actual, level, "Write verification failed on iteration {i}");
                }
                Err(e) => {
                    panic!("GPIO write failed on iteration {i}: {e}");
                }
            }
        }
    }

    #[test]
    #[ignore] // Requires hardware
    fn test_gpio_write_performance_impact() {
        let device = match open_test_device() {
            Some(d) => d,
            None => {
                println!("No XR2280x device found, skipping hardware test");
                return;
            }
        };

        let pin = match GpioPin::new(0) {
            Ok(p) => p,
            Err(_) => {
                println!("Failed to create test pin");
                return;
            }
        };

        if pin.number() >= device.get_capabilities().gpio_count {
            println!("Pin {} not supported on this device", pin.number());
            return;
        }

        // Setup pin as output
        device.gpio_assign_to_edge(pin).unwrap();
        device
            .gpio_setup_output(pin, GpioLevel::Low, GpioPull::None)
            .unwrap();

        const ITERATIONS: usize = 50;

        // Benchmark fast mode
        device
            .gpio_set_write_config(GpioWriteConfig::fast())
            .unwrap();
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            device.gpio_write(pin, GpioLevel::High).unwrap();
            device.gpio_write(pin, GpioLevel::Low).unwrap();
        }
        let fast_duration = start.elapsed();

        // Benchmark verified mode
        device
            .gpio_set_write_config(GpioWriteConfig::reliable())
            .unwrap();
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            device.gpio_write(pin, GpioLevel::High).unwrap();
            device.gpio_write(pin, GpioLevel::Low).unwrap();
        }
        let verified_duration = start.elapsed();

        println!("Performance comparison over {ITERATIONS} iterations:");
        println!("  Fast mode:     {fast_duration:?}");
        println!("  Verified mode: {verified_duration:?}");
        println!(
            "  Slowdown:      {:.2}x",
            verified_duration.as_secs_f64() / fast_duration.as_secs_f64()
        );

        // Verified mode should be slower but not excessively so
        let slowdown_ratio = verified_duration.as_secs_f64() / fast_duration.as_secs_f64();
        assert!(
            slowdown_ratio > 1.0,
            "Verified mode should be slower than fast mode"
        );
        assert!(
            slowdown_ratio < 10.0,
            "Verified mode shouldn't be more than 10x slower"
        );
    }

    #[test]
    #[ignore] // Requires hardware
    fn test_gpio_write_timeout_behavior() {
        let device = match open_test_device() {
            Some(d) => d,
            None => {
                println!("No XR2280x device found, skipping hardware test");
                return;
            }
        };

        let pin = match GpioPin::new(0) {
            Ok(p) => p,
            Err(_) => {
                println!("Failed to create test pin");
                return;
            }
        };

        if pin.number() >= device.get_capabilities().gpio_count {
            println!("Pin {} not supported on this device", pin.number());
            return;
        }

        // Setup pin as output
        device.gpio_assign_to_edge(pin).unwrap();
        device
            .gpio_setup_output(pin, GpioLevel::Low, GpioPull::None)
            .unwrap();

        // Configure with very short timeout to force timeout condition
        let timeout_config = GpioWriteConfig {
            verify_writes: true,
            retry_attempts: 10,
            retry_delay: Duration::from_millis(50),
            operation_timeout: Duration::from_millis(100), // Very short timeout
        };

        device.gpio_set_write_config(timeout_config).unwrap();

        let start = Instant::now();
        let result = device.gpio_write(pin, GpioLevel::High);
        let duration = start.elapsed();

        // Should either succeed quickly or timeout
        match result {
            Ok(()) => {
                assert!(
                    duration < Duration::from_millis(150),
                    "Operation took too long: {duration:?}"
                );
            }
            Err(e) => {
                // Check if it's a timeout error
                let error_string = format!("{e}");
                if error_string.contains("timed out") {
                    // This is expected with the short timeout
                    assert!(
                        duration >= Duration::from_millis(90),
                        "Timeout should respect the configured timeout"
                    );
                } else {
                    // Other errors are also acceptable in this test
                    println!("Got non-timeout error (also acceptable): {e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod mock_tests {
    use super::*;

    // Test configuration validation
    #[test]
    fn test_config_validation() {
        // Test that configurations can be constructed with various values
        let configs = vec![
            GpioWriteConfig {
                verify_writes: true,
                retry_attempts: 0,
                retry_delay: Duration::from_millis(1),
                operation_timeout: Duration::from_millis(100),
            },
            GpioWriteConfig {
                verify_writes: false,
                retry_attempts: 100,
                retry_delay: Duration::from_secs(1),
                operation_timeout: Duration::from_secs(10),
            },
        ];

        for config in configs {
            // Just verify we can create and clone these configurations
            let _cloned = config.clone();
            let _debug_str = format!("{config:?}");
        }
    }

    #[test]
    fn test_gpio_level_debug_format() {
        // Test that GpioLevel can be formatted for error messages
        let high = GpioLevel::High;
        let low = GpioLevel::Low;

        let high_str = format!("{high:?}");
        let low_str = format!("{low:?}");

        assert_eq!(high_str, "High");
        assert_eq!(low_str, "Low");
    }
}
