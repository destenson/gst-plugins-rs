#![allow(unused)]
// Camera Compatibility Testing Framework
// Tests rtspsrc2 against various real and simulated IP cameras

use gst::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraTestConfig {
    pub name: String,
    pub vendor: String,
    pub model: String,
    pub firmware: Option<String>,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub transport: TransportMode,
    pub auth_type: AuthType,
    pub features: CameraFeatures,
    pub known_quirks: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransportMode {
    Tcp,
    Udp,
    UdpMulticast,
    Http,
    Auto,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuthType {
    None,
    Basic,
    Digest,
    OnvifToken,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraFeatures {
    pub h264: bool,
    pub h265: bool,
    pub mjpeg: bool,
    pub audio: bool,
    pub ptz: bool,
    pub onvif: bool,
    pub backchannel: bool,
    pub events: bool,
    pub seeking: bool,
}

impl Default for CameraFeatures {
    fn default() -> Self {
        Self {
            h264: true,
            h265: false,
            mjpeg: false,
            audio: false,
            ptz: false,
            onvif: false,
            backchannel: false,
            events: false,
            seeking: false,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompatibilityTestResult {
    pub camera: CameraTestConfig,
    pub connectivity: TestStatus,
    pub stream_formats: HashMap<String, TestStatus>,
    pub features: HashMap<String, TestStatus>,
    pub performance: PerformanceMetrics,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub test_duration: Duration,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TestStatus {
    Pass,
    Fail(String),
    Skip(String),
    NotSupported,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceMetrics {
    pub connection_time: Duration,
    pub first_frame_time: Duration,
    pub average_latency: Duration,
    pub dropped_frames: u64,
    pub bitrate: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            connection_time: Duration::from_secs(0),
            first_frame_time: Duration::from_secs(0),
            average_latency: Duration::from_secs(0),
            dropped_frames: 0,
            bitrate: 0,
        }
    }
}

#[allow(dead_code)]
pub struct CameraCompatibilityTester {
    pub configs: Vec<CameraTestConfig>,
    pub results: Arc<Mutex<Vec<CompatibilityTestResult>>>,
}

impl CameraCompatibilityTester {
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            results: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_camera(&mut self, config: CameraTestConfig) {
        self.configs.push(config);
    }

    pub fn load_builtin_cameras(&mut self) {
        // Add known test cameras
        self.configs.extend(vec![
            // Public test streams
            CameraTestConfig {
                name: "Wowza Test Stream".to_string(),
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
                known_quirks: vec!["Static looping video".to_string()],
            },
            // Axis camera pattern
            CameraTestConfig {
                name: "Axis M3045-V".to_string(),
                vendor: "Axis".to_string(),
                model: "M3045-V".to_string(),
                firmware: Some("9.80.1".to_string()),
                url: "rtsp://{host}/axis-media/media.amp".to_string(),
                username: Some("root".to_string()),
                password: Some("pass".to_string()),
                transport: TransportMode::Auto,
                auth_type: AuthType::Digest,
                features: CameraFeatures {
                    h264: true,
                    h265: true,
                    audio: true,
                    ptz: false,
                    onvif: true,
                    ..Default::default()
                },
                known_quirks: vec![
                    "High default bitrate".to_string(),
                    "Requires digest auth".to_string(),
                ],
            },
            // Hikvision pattern
            CameraTestConfig {
                name: "Hikvision DS-2CD2132F".to_string(),
                vendor: "Hikvision".to_string(),
                model: "DS-2CD2132F".to_string(),
                firmware: Some("V5.4.5".to_string()),
                url: "rtsp://{host}:554/Streaming/Channels/101".to_string(),
                username: Some("admin".to_string()),
                password: Some("admin123".to_string()),
                transport: TransportMode::Tcp,
                auth_type: AuthType::Digest,
                features: CameraFeatures {
                    h264: true,
                    h265: true,
                    audio: true,
                    onvif: true,
                    ..Default::default()
                },
                known_quirks: vec![
                    "Channel IDs: 101=main, 102=sub".to_string(),
                    "Requires ONVIF account setup".to_string(),
                ],
            },
            // Dahua pattern
            CameraTestConfig {
                name: "Dahua IPC-HFW4431E".to_string(),
                vendor: "Dahua".to_string(),
                model: "IPC-HFW4431E".to_string(),
                firmware: Some("2.800.0000000.16.R".to_string()),
                url: "rtsp://{host}:554/cam/realmonitor?channel=1&subtype=0".to_string(),
                username: Some("admin".to_string()),
                password: Some("admin".to_string()),
                transport: TransportMode::Auto,
                auth_type: AuthType::Digest,
                features: CameraFeatures {
                    h264: true,
                    h265: false,
                    audio: true,
                    onvif: true,
                    ptz: false,
                    ..Default::default()
                },
                known_quirks: vec![
                    "subtype: 0=main, 1=sub".to_string(),
                    "Port 8080 for ONVIF".to_string(),
                ],
            },
        ]);
    }

    pub async fn test_camera(&self, config: &CameraTestConfig) -> CompatibilityTestResult {
        let start_time = Instant::now();
        let mut result = CompatibilityTestResult {
            camera: config.clone(),
            connectivity: TestStatus::Skip("Not tested".to_string()),
            stream_formats: HashMap::new(),
            features: HashMap::new(),
            performance: PerformanceMetrics::default(),
            errors: Vec::new(),
            warnings: Vec::new(),
            test_duration: Duration::from_secs(0),
        };

        // Test connectivity
        result.connectivity = self.test_connectivity(config).await;

        // Test stream formats
        if matches!(result.connectivity, TestStatus::Pass) {
            if config.features.h264 {
                result.stream_formats.insert(
                    "H264".to_string(),
                    self.test_stream_format(config, "h264").await,
                );
            }
            if config.features.h265 {
                result.stream_formats.insert(
                    "H265".to_string(),
                    self.test_stream_format(config, "h265").await,
                );
            }
            if config.features.mjpeg {
                result.stream_formats.insert(
                    "MJPEG".to_string(),
                    self.test_stream_format(config, "mjpeg").await,
                );
            }

            // Test features
            if config.features.audio {
                result
                    .features
                    .insert("Audio".to_string(), self.test_audio(config).await);
            }
            if config.features.ptz {
                result
                    .features
                    .insert("PTZ".to_string(), self.test_ptz(config).await);
            }
            if config.features.onvif {
                result
                    .features
                    .insert("ONVIF".to_string(), self.test_onvif(config).await);
            }
            if config.features.backchannel {
                result.features.insert(
                    "Backchannel".to_string(),
                    self.test_backchannel(config).await,
                );
            }

            // Test performance
            result.performance = self.test_performance(config).await;
        }

        result.test_duration = start_time.elapsed();
        result
    }

    async fn test_connectivity(&self, config: &CameraTestConfig) -> TestStatus {
        // Skip placeholder URLs
        if config.url.contains("{host}") {
            return TestStatus::Skip("Placeholder URL".to_string());
        }

        gst::init().ok();

        let pipeline_str = format!("rtspsrc2 location={} latency=100 ! fakesink", config.url);

        match gst::parse::launch(&pipeline_str) {
            Ok(pipeline) => {
                let pipeline = pipeline.downcast::<gst::Pipeline>().unwrap();

                if let Some(rtspsrc) = pipeline.by_name("rtspsrc2") {
                    if let Some(username) = &config.username {
                        rtspsrc.set_property("user-id", username);
                    }
                    if let Some(password) = &config.password {
                        rtspsrc.set_property("user-pw", password);
                    }
                }

                pipeline.set_state(gst::State::Playing).ok();

                // Wait for state change
                let (res, _, _) = pipeline.state(Some(gst::ClockTime::from_seconds(10)));

                pipeline.set_state(gst::State::Null).ok();

                match res {
                    Ok(gst::StateChangeSuccess::Success) => TestStatus::Pass,
                    Ok(gst::StateChangeSuccess::Async) => TestStatus::Pass,
                    _ => TestStatus::Fail("Failed to connect".to_string()),
                }
            }
            Err(e) => TestStatus::Fail(format!("Pipeline creation failed: {}", e)),
        }
    }

    async fn test_stream_format(&self, _config: &CameraTestConfig, format: &str) -> TestStatus {
        // TODO: Implement format-specific testing
        TestStatus::Skip(format!("{} testing not implemented", format))
    }

    async fn test_audio(&self, _config: &CameraTestConfig) -> TestStatus {
        TestStatus::Skip("Audio testing not implemented".to_string())
    }

    async fn test_ptz(&self, _config: &CameraTestConfig) -> TestStatus {
        TestStatus::Skip("PTZ testing not implemented".to_string())
    }

    async fn test_onvif(&self, _config: &CameraTestConfig) -> TestStatus {
        TestStatus::Skip("ONVIF testing not implemented".to_string())
    }

    async fn test_backchannel(&self, _config: &CameraTestConfig) -> TestStatus {
        TestStatus::Skip("Backchannel testing not implemented".to_string())
    }

    async fn test_performance(&self, config: &CameraTestConfig) -> PerformanceMetrics {
        let mut metrics = PerformanceMetrics::default();

        // Skip placeholder URLs
        if config.url.contains("{host}") {
            return metrics;
        }

        let connection_start = Instant::now();

        // Measure connection time
        let pipeline_str = format!(
            "rtspsrc2 location={} name=src ! fakesink sync=false",
            config.url
        );

        if let Ok(pipeline) = gst::parse::launch(&pipeline_str) {
            let pipeline = pipeline.downcast::<gst::Pipeline>().unwrap();

            if pipeline.set_state(gst::State::Playing).is_ok() {
                // Wait for first buffer
                let bus = pipeline.bus().unwrap();
                let timeout = gst::ClockTime::from_seconds(10);

                if let Some(msg) = bus.timed_pop_filtered(
                    timeout,
                    &[gst::MessageType::StateChanged, gst::MessageType::Error],
                ) {
                    match msg.view() {
                        gst::MessageView::StateChanged(state_changed) => {
                            if state_changed.old() == gst::State::Ready
                                && state_changed.current() == gst::State::Playing
                            {
                                metrics.connection_time = connection_start.elapsed();
                            }
                        }
                        _ => {}
                    }
                }
            }

            pipeline.set_state(gst::State::Null).ok();
        }

        metrics
    }

    pub async fn run_all_tests(&mut self) -> Vec<CompatibilityTestResult> {
        let mut all_results = Vec::new();

        for config in self.configs.clone() {
            println!("Testing camera: {} - {}", config.vendor, config.model);
            let result = self.test_camera(&config).await;
            all_results.push(result);
        }

        all_results
    }

    pub fn generate_report(&self, results: &[CompatibilityTestResult]) -> String {
        let mut report = String::from("# Camera Compatibility Test Report\n\n");
        report.push_str(&format!("Generated: {}\n", "Test Report"));
        report.push_str(&format!("Total cameras tested: {}\n\n", results.len()));

        report.push_str("## Summary\n\n");

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

        report.push_str(&format!("- Passed: {}\n", passed));
        report.push_str(&format!("- Failed: {}\n", failed));
        report.push_str(&format!("- Skipped: {}\n\n", skipped));

        report.push_str("## Detailed Results\n\n");

        for result in results {
            report.push_str(&format!(
                "### {} - {}\n",
                result.camera.vendor, result.camera.model
            ));
            report.push_str(&format!("- **Name**: {}\n", result.camera.name));
            if let Some(fw) = &result.camera.firmware {
                report.push_str(&format!("- **Firmware**: {}\n", fw));
            }
            report.push_str(&format!("- **Connectivity**: {:?}\n", result.connectivity));

            if !result.stream_formats.is_empty() {
                report.push_str("- **Stream Formats**:\n");
                for (format, status) in &result.stream_formats {
                    report.push_str(&format!("  - {}: {:?}\n", format, status));
                }
            }

            if !result.features.is_empty() {
                report.push_str("- **Features**:\n");
                for (feature, status) in &result.features {
                    report.push_str(&format!("  - {}: {:?}\n", feature, status));
                }
            }

            if result.performance.connection_time > Duration::from_secs(0) {
                report.push_str("- **Performance**:\n");
                report.push_str(&format!(
                    "  - Connection time: {:?}\n",
                    result.performance.connection_time
                ));
                if result.performance.first_frame_time > Duration::from_secs(0) {
                    report.push_str(&format!(
                        "  - First frame: {:?}\n",
                        result.performance.first_frame_time
                    ));
                }
            }

            if !result.camera.known_quirks.is_empty() {
                report.push_str("- **Known Quirks**:\n");
                for quirk in &result.camera.known_quirks {
                    report.push_str(&format!("  - {}\n", quirk));
                }
            }

            if !result.errors.is_empty() {
                report.push_str("- **Errors**:\n");
                for error in &result.errors {
                    report.push_str(&format!("  - {}\n", error));
                }
            }

            report.push_str(&format!(
                "- **Test Duration**: {:?}\n\n",
                result.test_duration
            ));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_public_stream() {
        gst::init().unwrap();

        let mut tester = CameraCompatibilityTester::new();

        // Test with Wowza public stream
        let config = CameraTestConfig {
            name: "Wowza Test".to_string(),
            vendor: "Wowza".to_string(),
            model: "Test Server".to_string(),
            firmware: None,
            url:
                "rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2"
                    .to_string(),
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

        tester.add_camera(config);
        let results = tester.run_all_tests().await;

        assert_eq!(results.len(), 1);

        // Generate report
        let report = tester.generate_report(&results);
        println!("{}", report);
    }

    #[tokio::test]
    async fn test_builtin_cameras() {
        gst::init().unwrap();

        let mut tester = CameraCompatibilityTester::new();
        tester.load_builtin_cameras();

        // Should have loaded multiple camera configs
        assert!(tester.configs.len() > 3);

        // Run tests (most will be skipped due to placeholder URLs)
        let results = tester.run_all_tests().await;

        // Generate report
        let report = tester.generate_report(&results);
        println!("{}", report);

        // Save report to file
        std::fs::write("camera_compatibility_report.md", report).ok();
    }
}
