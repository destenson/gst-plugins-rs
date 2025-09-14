// End-to-End Pipeline Tests
// Tests real-world gst-launch-1.0 pipelines with various camera configurations

use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

#[path = "common/mod.rs"]
mod common;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PipelineTestConfig {
    pub name: String,
    pub pipeline: String,
    pub expected_result: ExpectedResult,
    pub timeout_seconds: u64,
    pub required_elements: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ExpectedResult {
    Success,
    Failure,
    NetworkDependent, // May succeed or fail based on network
}

#[allow(dead_code)]
pub struct E2EPipelineTest {
    plugin_path: Option<String>,
    test_configs: Vec<PipelineTestConfig>,
    gst_debug_level: Option<String>,
}

impl E2EPipelineTest {
    pub fn new() -> Self {
        Self {
            plugin_path: None,
            test_configs: Vec::new(),
            gst_debug_level: None,
        }
    }

    pub fn with_plugin_path(mut self, path: &str) -> Self {
        self.plugin_path = Some(path.to_string());
        self
    }

    pub fn with_debug_level(mut self, level: &str) -> Self {
        self.gst_debug_level = Some(level.to_string());
        self
    }

    pub fn load_default_test_configs(&mut self) {
        self.test_configs = vec![
            // Basic connectivity tests
            PipelineTestConfig {
                name: "Basic Invalid URL".to_string(),
                pipeline: "rtspsrc2 location=rtsp://invalid.local ! fakesink".to_string(),
                expected_result: ExpectedResult::Failure,
                timeout_seconds: 5,
                required_elements: vec!["rtspsrc2".to_string()],
            },

            // Public test stream
            PipelineTestConfig {
                name: "Wowza Public Stream".to_string(),
                pipeline: "rtspsrc2 location=rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2 ! rtph264depay ! h264parse ! fakesink sync=false".to_string(),
                expected_result: ExpectedResult::NetworkDependent,
                timeout_seconds: 15,
                required_elements: vec!["rtspsrc2".to_string(), "rtph264depay".to_string(), "h264parse".to_string()],
            },

            // Property testing
            PipelineTestConfig {
                name: "Latency Property".to_string(),
                pipeline: "rtspsrc2 location=rtsp://test.local latency=5000 ! fakesink".to_string(),
                expected_result: ExpectedResult::Failure,
                timeout_seconds: 3,
                required_elements: vec!["rtspsrc2".to_string()],
            },

            // Retry strategy testing
            PipelineTestConfig {
                name: "Retry Strategy None".to_string(),
                pipeline: "rtspsrc2 location=rtsp://test.local retry-strategy=none ! fakesink".to_string(),
                expected_result: ExpectedResult::Failure,
                timeout_seconds: 3,
                required_elements: vec!["rtspsrc2".to_string()],
            },

            PipelineTestConfig {
                name: "Retry Strategy Exponential".to_string(),
                pipeline: "rtspsrc2 location=rtsp://test.local retry-strategy=exponential max-reconnection-attempts=2 ! fakesink".to_string(),
                expected_result: ExpectedResult::Failure,
                timeout_seconds: 8,
                required_elements: vec!["rtspsrc2".to_string()],
            },

            // Transport testing
            PipelineTestConfig {
                name: "TCP Transport".to_string(),
                pipeline: "rtspsrc2 location=rtsp://test.local protocols=tcp ! fakesink".to_string(),
                expected_result: ExpectedResult::Failure,
                timeout_seconds: 5,
                required_elements: vec!["rtspsrc2".to_string()],
            },

            // Complex pipeline with multiple elements
            PipelineTestConfig {
                name: "Complete Video Pipeline".to_string(),
                pipeline: "rtspsrc2 location=rtsp://test.local ! rtph264depay ! h264parse ! queue ! fakesink sync=false".to_string(),
                expected_result: ExpectedResult::Failure,
                timeout_seconds: 5,
                required_elements: vec!["rtspsrc2".to_string(), "rtph264depay".to_string(), "h264parse".to_string(), "queue".to_string()],
            },

            // Audio/Video pipeline
            PipelineTestConfig {
                name: "Audio Video Pipeline".to_string(),
                pipeline: "rtspsrc2 location=rtsp://test.local name=src src.stream_0 ! rtph264depay ! h264parse ! fakesink sync=false src.stream_1 ! rtpopusdepay ! opusparse ! fakesink sync=false".to_string(),
                expected_result: ExpectedResult::Failure,
                timeout_seconds: 5,
                required_elements: vec!["rtspsrc2".to_string()],
            },
        ];
    }

    pub fn add_camera_specific_tests(&mut self) {
        // Axis camera tests (with placeholder IPs)
        self.test_configs.push(PipelineTestConfig {
            name: "Axis Camera".to_string(),
            pipeline: "rtspsrc2 location=rtsp://192.168.1.100/axis-media/media.amp user-id=root user-pw=password ! rtph264depay ! h264parse ! fakesink".to_string(),
            expected_result: ExpectedResult::NetworkDependent,
            timeout_seconds: 10,
            required_elements: vec!["rtspsrc2".to_string()],
        });

        // Hikvision camera tests
        self.test_configs.push(PipelineTestConfig {
            name: "Hikvision Camera".to_string(),
            pipeline: "rtspsrc2 location=rtsp://192.168.1.101:554/Streaming/Channels/101 user-id=admin user-pw=admin123 protocols=tcp ! rtph264depay ! h264parse ! fakesink".to_string(),
            expected_result: ExpectedResult::NetworkDependent,
            timeout_seconds: 10,
            required_elements: vec!["rtspsrc2".to_string()],
        });

        // Dahua camera tests
        self.test_configs.push(PipelineTestConfig {
            name: "Dahua Camera".to_string(),
            pipeline: "rtspsrc2 location=\"rtsp://192.168.1.102:554/cam/realmonitor?channel=1&subtype=0\" user-id=admin user-pw=admin ! rtph264depay ! h264parse ! fakesink".to_string(),
            expected_result: ExpectedResult::NetworkDependent,
            timeout_seconds: 10,
            required_elements: vec!["rtspsrc2".to_string()],
        });
    }

    pub async fn test_single_pipeline(
        &self,
        config: &PipelineTestConfig,
    ) -> Result<PipelineTestResult, Box<dyn std::error::Error>> {
        println!("Testing pipeline: {}", config.name);
        println!("Command: gst-launch-1.0 {}", config.pipeline);

        // Check required elements first
        for element in &config.required_elements {
            if !self.check_element_available(element)? {
                return Ok(PipelineTestResult {
                    config: config.clone(),
                    success: false,
                    error_message: Some(format!("Required element '{}' not available", element)),
                    execution_time: Duration::from_secs(0),
                    stdout: String::new(),
                    stderr: String::new(),
                });
            }
        }

        let start_time = std::time::Instant::now();

        let mut cmd = Command::new("gst-launch-1.0");
        cmd.args(&["--quiet", &format!("--timeout={}", config.timeout_seconds)]);

        // Add pipeline arguments
        for arg in config.pipeline.split_whitespace() {
            cmd.arg(arg);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Set environment variables
        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        if let Some(debug_level) = &self.gst_debug_level {
            cmd.env("GST_DEBUG", debug_level);
        }

        // Execute with timeout
        let output = timeout(
            Duration::from_secs(config.timeout_seconds + 5),
            tokio::task::spawn_blocking(move || cmd.output()),
        )
        .await??;

        let execution_time = start_time.elapsed();
        let output = output?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Analyze result based on expected outcome
        let success = match config.expected_result {
            ExpectedResult::Success => output.status.success(),
            ExpectedResult::Failure => !output.status.success(),
            ExpectedResult::NetworkDependent => {
                // Success or reasonable network failure
                output.status.success()
                    || stderr.contains("Could not resolve")
                    || stderr.contains("Connection refused")
                    || stderr.contains("Network is unreachable")
                    || stderr.contains("No route to host")
                    || stderr.contains("Temporary failure in name resolution")
            }
        };

        let error_message = if !success && !stderr.is_empty() {
            Some(stderr.clone())
        } else {
            None
        };

        Ok(PipelineTestResult {
            config: config.clone(),
            success,
            error_message,
            execution_time,
            stdout,
            stderr,
        })
    }

    fn check_element_available(
        &self,
        element_name: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut cmd = Command::new("gst-inspect-1.0");
        cmd.args(&["--exists", element_name]);

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        let output = cmd.output()?;
        Ok(output.status.success())
    }

    pub async fn run_all_pipeline_tests(&self) -> Vec<PipelineTestResult> {
        let mut results = Vec::new();

        println!("🚀 Running {} pipeline tests...\n", self.test_configs.len());

        for (i, config) in self.test_configs.iter().enumerate() {
            println!(
                "Test {}/{}: {}",
                i + 1,
                self.test_configs.len(),
                config.name
            );

            match self.test_single_pipeline(config).await {
                Ok(result) => {
                    if result.success {
                        println!("✓ PASSED");
                    } else {
                        println!("✗ FAILED");
                        if let Some(error) = &result.error_message {
                            // Only show first few lines of error to keep output clean
                            let error_lines: Vec<&str> = error.lines().take(3).collect();
                            for line in error_lines {
                                if !line.trim().is_empty() {
                                    println!("  {}", line);
                                }
                            }
                        }
                    }
                    results.push(result);
                }
                Err(e) => {
                    println!("✗ ERROR: {}", e);
                    results.push(PipelineTestResult {
                        config: config.clone(),
                        success: false,
                        error_message: Some(e.to_string()),
                        execution_time: Duration::from_secs(0),
                        stdout: String::new(),
                        stderr: String::new(),
                    });
                }
            }

            println!(
                "  Execution time: {:?}\n",
                results.last().unwrap().execution_time
            );
        }

        results
    }

    pub fn generate_test_report(&self, results: &[PipelineTestResult]) -> String {
        let mut report = String::from("# End-to-End Pipeline Test Report\n\n");

        let total = results.len();
        let passed = results.iter().filter(|r| r.success).count();
        let failed = total - passed;

        report.push_str(&format!("**Total Tests**: {}\n", total));
        report.push_str(&format!(
            "**Passed**: {} ({:.1}%)\n",
            passed,
            (passed as f64 / total as f64) * 100.0
        ));
        report.push_str(&format!(
            "**Failed**: {} ({:.1}%)\n\n",
            failed,
            (failed as f64 / total as f64) * 100.0
        ));

        report.push_str("## Test Results\n\n");

        for result in results {
            report.push_str(&format!("### {}\n", result.config.name));
            report.push_str(&format!("**Pipeline**: `{}`\n", result.config.pipeline));
            report.push_str(&format!(
                "**Expected**: {:?}\n",
                result.config.expected_result
            ));
            report.push_str(&format!(
                "**Result**: {}\n",
                if result.success {
                    "✅ PASSED"
                } else {
                    "❌ FAILED"
                }
            ));
            report.push_str(&format!(
                "**Execution Time**: {:?}\n",
                result.execution_time
            ));

            if let Some(error) = &result.error_message {
                report.push_str(&format!("**Error**: ```\n{}\n```\n", error));
            }

            if !result.stdout.is_empty() {
                report.push_str(&format!("**Output**: ```\n{}\n```\n", result.stdout));
            }

            report.push_str("\n");
        }

        report
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PipelineTestResult {
    pub config: PipelineTestConfig,
    pub success: bool,
    pub error_message: Option<String>,
    pub execution_time: Duration,
    pub stdout: String,
    pub stderr: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::common::{check_gstreamer_available, find_gst_plugin_path};

    #[tokio::test]
    async fn test_basic_pipeline_validation() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping pipeline tests");
            return;
        }

        let mut tester = E2EPipelineTest::new();
        if let Some(path) = find_gst_plugin_path() {
            tester = tester.with_plugin_path(&path);
        }

        // Load basic test configurations
        tester.load_default_test_configs();

        // Run a subset of tests
        let basic_tests: Vec<_> = tester
            .test_configs
            .iter()
            .filter(|config| config.name.contains("Basic") || config.name.contains("Property"))
            .cloned()
            .collect();

        for config in basic_tests {
            let result = tester.test_single_pipeline(&config).await;
            match result {
                Ok(test_result) => {
                    println!(
                        "Test '{}': {}",
                        config.name,
                        if test_result.success {
                            "PASSED"
                        } else {
                            "FAILED"
                        }
                    );
                    if !test_result.success && test_result.error_message.is_some() {
                        println!(
                            "  Error: {}",
                            test_result
                                .error_message
                                .as_ref()
                                .unwrap()
                                .lines()
                                .next()
                                .unwrap_or("Unknown error")
                        );
                    }
                }
                Err(e) => {
                    println!("Test '{}' failed to execute: {}", config.name, e);
                }
            }
        }
    }

    #[tokio::test]
    #[ignore] // Only run when explicitly requested
    async fn test_full_pipeline_suite() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping pipeline tests");
            return;
        }

        let mut tester = E2EPipelineTest::new();
        if let Some(path) = find_gst_plugin_path() {
            tester = tester.with_plugin_path(&path);
        }

        tester.load_default_test_configs();
        tester.add_camera_specific_tests();

        let results = tester.run_all_pipeline_tests().await;

        // Generate and print report
        let report = tester.generate_test_report(&results);
        println!("{}", report);

        // Save report to file
        std::fs::write("e2e_pipeline_test_report.md", report).ok();

        // Check that at least some basic tests passed
        let basic_passed = results
            .iter()
            .filter(|r| r.config.name.contains("Basic") || r.config.name.contains("Property"))
            .any(|r| r.success);

        assert!(
            basic_passed,
            "At least some basic pipeline tests should pass"
        );
    }

    #[tokio::test]
    async fn test_element_availability() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping element tests");
            return;
        }

        let mut tester = E2EPipelineTest::new();
        if let Some(path) = find_gst_plugin_path() {
            tester = tester.with_plugin_path(&path);
        }

        // Test availability of core elements
        let elements = vec!["rtspsrc2", "rtph264depay", "h264parse", "fakesink"];

        for element in elements {
            match tester.check_element_available(element) {
                Ok(available) => {
                    println!(
                        "Element '{}': {}",
                        element,
                        if available {
                            "Available"
                        } else {
                            "Not Available"
                        }
                    );
                }
                Err(e) => {
                    println!("Failed to check element '{}': {}", element, e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_public_stream_pipeline() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping public stream test");
            return;
        }

        let mut tester = E2EPipelineTest::new();
        if let Some(path) = find_gst_plugin_path() {
            tester = tester.with_plugin_path(&path);
        }

        let config = PipelineTestConfig {
            name: "Public Stream Test".to_string(),
            pipeline: "rtspsrc2 location=rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2 ! rtph264depay ! h264parse ! fakesink sync=false".to_string(),
            expected_result: ExpectedResult::NetworkDependent,
            timeout_seconds: 10,
            required_elements: vec!["rtspsrc2".to_string(), "rtph264depay".to_string()],
        };

        let result = tester.test_single_pipeline(&config).await;
        match result {
            Ok(test_result) => {
                println!(
                    "Public stream test: {}",
                    if test_result.success {
                        "PASSED"
                    } else {
                        "FAILED (may be expected)"
                    }
                );
                if !test_result.stderr.is_empty() {
                    println!(
                        "stderr: {}",
                        test_result.stderr.lines().next().unwrap_or("")
                    );
                }
            }
            Err(e) => {
                println!("Public stream test error: {}", e);
            }
        }
    }
}
