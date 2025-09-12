// Comprehensive Camera Compatibility Test Suite
// Main test orchestration for camera compatibility testing

// Include the test modules inline since they're in the same tests directory
#[path = "camera_compat.rs"]
mod camera_compat;
#[path = "camera_config.rs"]
mod camera_config;
#[path = "onvif_discovery.rs"]
mod onvif_discovery;

use camera_compat::*;
use camera_config::*;
use onvif_discovery::*;

use gst::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

#[allow(dead_code)]
pub struct CompatibilityTestSuite {
    tester: CameraCompatibilityTester,
    test_categories: Vec<TestCategory>,
    config_file: Option<String>,
    use_discovery: bool,
    generate_report: bool,
    output_dir: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum TestCategory {
    BasicConnectivity,
    StreamFormats,
    Features,
    Reliability,
    Performance,
    All,
}

impl CompatibilityTestSuite {
    pub fn new() -> Self {
        Self {
            tester: CameraCompatibilityTester::new(),
            test_categories: vec![TestCategory::All],
            config_file: None,
            use_discovery: false,
            generate_report: true,
            output_dir: "test-results".to_string(),
        }
    }

    pub fn with_config_file(mut self, path: &str) -> Self {
        self.config_file = Some(path.to_string());
        self
    }

    pub fn with_discovery(mut self, enabled: bool) -> Self {
        self.use_discovery = enabled;
        self
    }

    pub fn with_categories(mut self, categories: Vec<TestCategory>) -> Self {
        self.test_categories = categories;
        self
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Camera Compatibility Test Suite ===\n");

        // Initialize GStreamer
        gst::init()?;

        // Load cameras from various sources
        self.load_cameras()?;

        // Run compatibility tests
        let results = self.tester.run_all_tests().await;

        // Generate and save report
        if self.generate_report {
            self.save_report(&results)?;
        }

        // Print summary
        self.print_summary(&results);

        Ok(())
    }

    fn load_cameras(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load from config file if specified
        if let Some(config_path) = &self.config_file {
            println!("Loading cameras from config: {}", config_path);

            let config = if config_path.ends_with(".toml") {
                CameraConfigFile::load_from_toml(config_path)?
            } else if config_path.ends_with(".json") {
                CameraConfigFile::load_from_json(config_path)?
            } else {
                return Err("Unsupported config file format".into());
            };

            for cam_config in config.cameras {
                self.tester.add_camera(self.convert_config(cam_config));
            }
        }

        // Discover cameras via ONVIF if enabled
        if self.use_discovery {
            println!("Discovering ONVIF cameras...");

            // Use simulator for testing
            let simulator = OnvifSimulator::new();
            let devices = simulator.discover_devices();

            println!("Found {} ONVIF devices", devices.len());

            for device in devices {
                self.tester.add_camera(self.convert_onvif_device(device));
            }
        }

        // Load built-in test cameras
        self.tester.load_builtin_cameras();

        Ok(())
    }

    fn convert_config(&self, config: CameraConfig) -> CameraTestConfig {
        CameraTestConfig {
            name: config.name,
            vendor: config.vendor,
            model: config.model,
            firmware: config.firmware,
            url: config.url,
            username: config.username,
            password: config.password,
            transport: match config.transport.as_str() {
                "tcp" => TransportMode::Tcp,
                "udp" => TransportMode::Udp,
                "http" => TransportMode::Http,
                _ => TransportMode::Auto,
            },
            auth_type: match config.auth_type.as_str() {
                "basic" => AuthType::Basic,
                "digest" => AuthType::Digest,
                "onvif" => AuthType::OnvifToken,
                _ => AuthType::None,
            },
            features: CameraFeatures {
                h264: config.features.h264,
                h265: config.features.h265,
                mjpeg: config.features.mjpeg,
                audio: config.features.audio,
                ptz: config.features.ptz,
                onvif: config.features.onvif,
                backchannel: config.features.backchannel,
                events: config.features.events,
                seeking: config.features.seeking,
            },
            known_quirks: config.known_quirks,
        }
    }

    fn convert_onvif_device(&self, device: OnvifDevice) -> CameraTestConfig {
        CameraTestConfig {
            name: device.name,
            vendor: device.manufacturer,
            model: device.model,
            firmware: device.firmware,
            url: device
                .rtsp_url
                .unwrap_or_else(|| format!("rtsp://{}:554/", device.ip_address)),
            username: Some("admin".to_string()),
            password: Some("admin".to_string()),
            transport: TransportMode::Auto,
            auth_type: AuthType::Digest,
            features: CameraFeatures {
                h264: true,
                h265: false,
                audio: device.capabilities.media,
                ptz: device.capabilities.ptz,
                onvif: true,
                events: device.capabilities.events,
                ..Default::default()
            },
            known_quirks: Vec::new(),
        }
    }

    fn save_report(
        &self,
        results: &[CompatibilityTestResult],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create output directory
        std::fs::create_dir_all(&self.output_dir)?;

        // Generate report
        let report = self.tester.generate_report(results);

        // Save report with timestamp
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let report_path =
            Path::new(&self.output_dir).join(format!("compatibility_report_{}.md", timestamp));

        std::fs::write(&report_path, report)?;
        println!("\nReport saved to: {}", report_path.display());

        // Also save as JSON for programmatic access
        let json_path =
            Path::new(&self.output_dir).join(format!("compatibility_results_{}.json", timestamp));

        let json_results = serde_json::to_string_pretty(results)?;
        std::fs::write(&json_path, json_results)?;
        println!("JSON results saved to: {}", json_path.display());

        Ok(())
    }

    fn print_summary(&self, results: &[CompatibilityTestResult]) {
        println!("\n=== Test Summary ===\n");

        let total = results.len();
        let passed = results
            .iter()
            .filter(|r| matches!(r.connectivity, TestStatus::Pass))
            .count();
        let failed = results
            .iter()
            .filter(|r| matches!(r.connectivity, TestStatus::Fail(_)))
            .count();
        let skipped = results
            .iter()
            .filter(|r| matches!(r.connectivity, TestStatus::Skip(_)))
            .count();

        println!("Total cameras tested: {}", total);
        println!(
            "  Passed: {} ({:.1}%)",
            passed,
            (passed as f64 / total as f64) * 100.0
        );
        println!(
            "  Failed: {} ({:.1}%)",
            failed,
            (failed as f64 / total as f64) * 100.0
        );
        println!(
            "  Skipped: {} ({:.1}%)",
            skipped,
            (skipped as f64 / total as f64) * 100.0
        );

        // Group by vendor
        let mut vendor_stats: HashMap<String, (usize, usize, usize)> = HashMap::new();
        for result in results {
            let entry = vendor_stats
                .entry(result.camera.vendor.clone())
                .or_insert((0, 0, 0));

            match result.connectivity {
                TestStatus::Pass => entry.0 += 1,
                TestStatus::Fail(_) => entry.1 += 1,
                TestStatus::Skip(_) => entry.2 += 1,
                TestStatus::NotSupported => entry.2 += 1,
            }
        }

        println!("\nBy Vendor:");
        for (vendor, (pass, fail, skip)) in vendor_stats {
            println!(
                "  {}: {} passed, {} failed, {} skipped",
                vendor, pass, fail, skip
            );
        }

        // Performance summary for successful tests
        let successful: Vec<_> = results
            .iter()
            .filter(|r| matches!(r.connectivity, TestStatus::Pass))
            .collect();

        if !successful.is_empty() {
            println!("\nPerformance Summary:");

            let avg_connection_time: Duration = successful
                .iter()
                .map(|r| r.performance.connection_time)
                .sum::<Duration>()
                / successful.len() as u32;

            println!("  Average connection time: {:?}", avg_connection_time);

            let min_connection = successful
                .iter()
                .map(|r| r.performance.connection_time)
                .min()
                .unwrap_or_default();

            let max_connection = successful
                .iter()
                .map(|r| r.performance.connection_time)
                .max()
                .unwrap_or_default();

            println!("  Min connection time: {:?}", min_connection);
            println!("  Max connection time: {:?}", max_connection);
        }
    }
}

// Specialized test functions for different categories
pub async fn test_basic_connectivity(camera: &CameraTestConfig) -> TestStatus {
    gst::init().ok();

    let pipeline_str = format!("rtspsrc2 location={} latency=100 ! fakesink", camera.url);

    match gst::parse::launch(&pipeline_str) {
        Ok(pipeline) => {
            let pipeline = pipeline.downcast::<gst::Pipeline>().unwrap();
            pipeline.set_state(gst::State::Playing).ok();

            let (res, _, _) = pipeline.state(Some(gst::ClockTime::from_seconds(10)));
            pipeline.set_state(gst::State::Null).ok();

            match res {
                Ok(_) => TestStatus::Pass,
                Err(e) => TestStatus::Fail(format!("Connection failed: {:?}", e)),
            }
        }
        Err(e) => TestStatus::Fail(format!("Pipeline creation failed: {}", e)),
    }
}

pub async fn test_reconnection(_camera: &CameraTestConfig) -> TestStatus {
    // Test reconnection logic
    TestStatus::Skip("Reconnection test not implemented".to_string())
}

pub async fn test_timeout_handling(_camera: &CameraTestConfig) -> TestStatus {
    // Test timeout scenarios
    TestStatus::Skip("Timeout test not implemented".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_suite_initialization() {
        let suite = CompatibilityTestSuite::new();
        assert!(suite.generate_report);
        assert!(!suite.use_discovery);
    }

    #[tokio::test]
    async fn test_suite_with_simulated_devices() {
        let mut suite = CompatibilityTestSuite::new()
            .with_discovery(true)
            .with_categories(vec![TestCategory::BasicConnectivity]);

        // This will use simulated devices
        let result = suite.run().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_example_config() {
        // Generate example configuration file
        let config = create_example_config();
        config.save_to_toml("camera_test_config.toml").unwrap();

        println!("Example configuration saved to camera_test_config.toml");
    }
}
