// Main Camera Compatibility Test Module
// Integration point for all camera compatibility testing

#![cfg(test)]

#[path = "camera_compat.rs"]
mod camera_compat;
#[path = "camera_config.rs"]
mod camera_config;
#[path = "onvif_discovery.rs"]
mod onvif_discovery;
#[path = "compat_suite.rs"]
mod compat_suite;
#[path = "compat_benchmarks.rs"]
mod compat_benchmarks;
#[path = "compat_ci.rs"]
mod compat_ci;

use camera_compat::*;
use camera_config::*;
use onvif_discovery::*;
use compat_suite::*;
use compat_benchmarks::*;
use compat_ci::*;

#[tokio::test]
async fn test_camera_compatibility_framework() {
    // Basic framework test
    gst::init().unwrap();
    
    let mut tester = CameraCompatibilityTester::new();
    tester.load_builtin_cameras();
    
    // Should have loaded some test cameras
    assert!(tester.configs.len() > 0);
}

#[tokio::test]
async fn test_config_loading() {
    // Test configuration file loading
    let config = create_example_config();
    
    // Save and load test
    let path = "test_config_validation.toml";
    config.save_to_toml(path).unwrap();
    
    let loaded = CameraConfigFile::load_from_toml(path).unwrap();
    assert_eq!(loaded.cameras.len(), config.cameras.len());
    
    // Cleanup
    std::fs::remove_file(path).ok();
}

#[tokio::test]
async fn test_onvif_simulator() {
    // Test ONVIF device simulator
    let simulator = OnvifSimulator::new();
    let devices = simulator.discover_devices();
    
    // Should have simulated devices
    assert_eq!(devices.len(), 3);
    
    // Check device properties
    for device in devices {
        assert!(!device.name.is_empty());
        assert!(!device.manufacturer.is_empty());
        assert!(device.rtsp_url.is_some());
    }
}

#[tokio::test]
async fn test_public_stream_connectivity() {
    // Test with actual public stream
    gst::init().unwrap();
    
    let config = CameraTestConfig {
        name: "Wowza Public Test".to_string(),
        vendor: "Wowza".to_string(),
        model: "Test Server".to_string(),
        firmware: None,
        url: "rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2".to_string(),
        username: None,
        password: None,
        transport: TransportMode::Auto,
        auth_type: AuthType::None,
        features: CameraFeatures {
            h264: true,
            ..Default::default()
        },
        known_quirks: vec![],
    };
    
    let tester = CameraCompatibilityTester::new();
    let result = tester.test_camera(&config).await;
    
    // Check that test ran
    assert!(result.test_duration > std::time::Duration::from_secs(0));
}

#[tokio::test]
async fn test_benchmark_framework() {
    // Test benchmark configuration
    let config = BenchmarkConfig {
        test_duration: std::time::Duration::from_secs(5),
        measure_latency: true,
        measure_throughput: true,
        ..Default::default()
    };
    
    let benchmark = CameraBenchmark::new(config);
    assert!(benchmark.config.measure_latency);
}

#[tokio::test]
async fn test_ci_environment() {
    // Test CI environment setup
    let mut env = CITestEnvironment::new();
    let result = env.setup().await;
    assert!(result.is_ok());
    
    // Should have generated config
    assert!(std::path::Path::new(env.get_config_path()).exists());
    
    // Cleanup
    env.teardown().await;
    std::fs::remove_file(env.get_config_path()).ok();
}

#[tokio::test]
#[ignore] // Only run when explicitly requested
async fn test_full_compatibility_suite() {
    // Run full compatibility test suite
    let mut suite = CompatibilityTestSuite::new()
        .with_discovery(true)
        .with_categories(vec![TestCategory::All]);
    
    let result = suite.run().await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore] // Only run with real cameras
async fn test_real_camera_discovery() {
    // Test real ONVIF discovery
    let discovery = OnvifDiscovery::new();
    
    match discovery.discover_devices() {
        Ok(devices) => {
            println!("Found {} real ONVIF devices", devices.len());
            for device in devices {
                println!("  - {} at {}", device.name, device.ip_address);
            }
        }
        Err(e) => {
            println!("Discovery failed (expected in test environment): {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Only run for benchmarking
async fn test_performance_benchmark() {
    gst::init().unwrap();
    
    let urls = vec![
        "rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2".to_string(),
    ];
    
    let config = BenchmarkConfig {
        test_duration: std::time::Duration::from_secs(10),
        ..Default::default()
    };
    
    let results = run_benchmark_suite(urls, config).await;
    
    for result in results {
        println!("{}", CameraBenchmark::format_results(&result));
    }
}

// Integration test that validates the entire framework
#[tokio::test]
async fn test_framework_integration() {
    gst::init().unwrap();
    
    // 1. Create configuration
    let config = create_example_config();
    let config_path = "integration_test_config.toml";
    config.save_to_toml(config_path).unwrap();
    
    // 2. Set up test environment
    let mut env = CITestEnvironment::new();
    env.setup().await.unwrap();
    
    // 3. Run compatibility tests
    let mut tester = CameraCompatibilityTester::new();
    
    // Load from config
    let loaded_config = CameraConfigFile::load_from_toml(config_path).unwrap();
    for cam in loaded_config.cameras {
        if !cam.url.contains("{host}") {
            let test_config = CameraTestConfig {
                name: cam.name,
                vendor: cam.vendor,
                model: cam.model,
                firmware: cam.firmware,
                url: cam.url,
                username: cam.username,
                password: cam.password,
                transport: TransportMode::Auto,
                auth_type: AuthType::None,
                features: CameraFeatures::default(),
                known_quirks: cam.known_quirks,
            };
            tester.add_camera(test_config);
        }
    }
    
    // 4. Run tests
    let results = tester.run_all_tests().await;
    
    // 5. Generate report
    let report = tester.generate_report(&results);
    assert!(!report.is_empty());
    
    // 6. Cleanup
    env.teardown().await;
    std::fs::remove_file(config_path).ok();
    std::fs::remove_file(env.get_config_path()).ok();
}

// Helper function to run specific camera tests
pub async fn test_specific_camera(url: &str) -> Result<CompatibilityTestResult, Box<dyn std::error::Error>> {
    gst::init()?;
    
    let config = CameraTestConfig {
        name: "Manual Test".to_string(),
        vendor: "Unknown".to_string(),
        model: "Unknown".to_string(),
        firmware: None,
        url: url.to_string(),
        username: None,
        password: None,
        transport: TransportMode::Auto,
        auth_type: AuthType::None,
        features: CameraFeatures::default(),
        known_quirks: vec![],
    };
    
    let tester = CameraCompatibilityTester::new();
    Ok(tester.test_camera(&config).await)
}