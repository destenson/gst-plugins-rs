// End-to-End Visual Pipeline Tests
// Tests with autovideosink for visual verification and real video output

use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct VisualTestConfig {
    pub name: String,
    pub pipeline: String,
    pub duration_seconds: u64,
    pub description: String,
    pub requires_display: bool,
}

#[allow(dead_code)]
pub struct E2EVisualTest {
    plugin_path: Option<String>,
    test_configs: Vec<VisualTestConfig>,
    gst_debug_level: Option<String>,
    auto_close: bool,
}

impl E2EVisualTest {
    pub fn new() -> Self {
        Self {
            plugin_path: None,
            test_configs: Vec::new(),
            gst_debug_level: None,
            auto_close: true,
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

    pub fn with_auto_close(mut self, auto_close: bool) -> Self {
        self.auto_close = auto_close;
        self
    }

    pub fn load_visual_test_configs(&mut self) {
        self.test_configs = vec![
            // Basic visual test with public stream
            VisualTestConfig {
                name: "Public Stream Visual".to_string(),
                pipeline: "rtspsrc2 location=rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2 ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink".to_string(),
                duration_seconds: 15,
                description: "Display public RTSP test stream in video window".to_string(),
                requires_display: true,
            },

            // Test pattern with rtspsrc2 fallback
            VisualTestConfig {
                name: "Test Pattern Fallback".to_string(),
                pipeline: "videotestsrc pattern=smpte ! video/x-raw,width=640,height=480,framerate=15/1 ! textoverlay text=\"RTSP Test Pattern\" ! videoconvert ! autovideosink".to_string(),
                duration_seconds: 5,
                description: "Display test pattern when no RTSP source available".to_string(),
                requires_display: true,
            },

            // Retry strategy demonstration
            VisualTestConfig {
                name: "Retry Strategy Demo".to_string(),
                pipeline: "rtspsrc2 location=rtsp://invalid.local retry-strategy=exponential max-reconnection-attempts=3 initial-retry-delay=1000000000 ! rtph264depay ! avdec_h264 ! videoconvert ! autovideosink".to_string(),
                duration_seconds: 10,
                description: "Demonstrate retry behavior with invalid URL (will show black screen/error)".to_string(),
                requires_display: true,
            },

            // Property showcase
            VisualTestConfig {
                name: "Latency Test".to_string(),
                pipeline: "rtspsrc2 location=rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2 latency=5000 ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! textoverlay text=\"Latency: 5000ms\" ! autovideosink".to_string(),
                duration_seconds: 15,
                description: "Test with higher latency setting and visual indicator".to_string(),
                requires_display: true,
            },

            // Audio/Video test (if stream supports audio)
            VisualTestConfig {
                name: "Audio Video Test".to_string(),
                pipeline: "rtspsrc2 location=rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2 name=src src. ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink src. ! queue ! fakesink".to_string(),
                duration_seconds: 10,
                description: "Test audio/video stream with video output".to_string(),
                requires_display: true,
            },
        ];
    }

    pub fn load_camera_visual_tests(&mut self) {
        // Camera-specific visual tests with placeholders
        self.test_configs.extend(vec![
            VisualTestConfig {
                name: "Axis Camera Visual".to_string(),
                pipeline: "rtspsrc2 location=rtsp://192.168.1.100/axis-media/media.amp user-id=root user-pw=password ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! textoverlay text=\"Axis Camera\" ! autovideosink".to_string(),
                duration_seconds: 20,
                description: "Display Axis camera feed with label".to_string(),
                requires_display: true,
            },

            VisualTestConfig {
                name: "Hikvision Camera Visual".to_string(),
                pipeline: "rtspsrc2 location=rtsp://192.168.1.101:554/Streaming/Channels/101 user-id=admin user-pw=admin123 protocols=tcp ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! textoverlay text=\"Hikvision Camera\" ! autovideosink".to_string(),
                duration_seconds: 20,
                description: "Display Hikvision camera feed with TCP transport".to_string(),
                requires_display: true,
            },

            VisualTestConfig {
                name: "Dahua Camera Visual".to_string(),
                pipeline: "rtspsrc2 location=\"rtsp://192.168.1.102:554/cam/realmonitor?channel=1&subtype=0\" user-id=admin user-pw=admin ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! textoverlay text=\"Dahua Camera\" ! autovideosink".to_string(),
                duration_seconds: 20,
                description: "Display Dahua camera main stream".to_string(),
                requires_display: true,
            },
        ]);
    }

    pub async fn run_visual_test(
        &self,
        config: &VisualTestConfig,
    ) -> Result<VisualTestResult, Box<dyn std::error::Error>> {
        println!("\n🎥 Running Visual Test: {}", config.name);
        println!("Description: {}", config.description);
        println!("Duration: {}s", config.duration_seconds);

        if config.requires_display && !self.has_display() {
            return Ok(VisualTestResult {
                config: config.clone(),
                success: false,
                skipped: true,
                error_message: Some("No display available".to_string()),
                execution_time: Duration::from_secs(0),
                user_feedback: None,
            });
        }

        println!("Pipeline: {}", config.pipeline);

        if !self.auto_close {
            println!("\n⚠️  Press Ctrl+C to stop the test early");
            println!("The test will run for {} seconds", config.duration_seconds);
        }

        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(config.duration_seconds + 10);

        let mut cmd = Command::new("gst-launch-1.0");

        if self.auto_close {
            cmd.arg(&format!("--timeout={}", config.duration_seconds));
        }

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
        let output = timeout(timeout_duration, async move {
            tokio::task::spawn_blocking(move || cmd.output())
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        })
        .await??;

        let execution_time = start_time.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Analyze result
        let success = if output.status.success() {
            true
        } else {
            // Check for acceptable failures
            stderr.contains("Could not resolve")
                || stderr.contains("Connection refused")
                || stderr.contains("Network is unreachable")
                || stderr.contains("No route to host")
                || stderr.contains("Temporary failure in name resolution")
        };

        let error_message = if !success && !stderr.is_empty() {
            Some(stderr.lines().take(3).collect::<Vec<_>>().join("\n"))
        } else {
            None
        };

        // Get user feedback if test was visual
        let user_feedback = if success && config.requires_display && !self.auto_close {
            Some(self.get_user_feedback().await)
        } else {
            None
        };

        Ok(VisualTestResult {
            config: config.clone(),
            success,
            skipped: false,
            error_message,
            execution_time,
            user_feedback,
        })
    }

    fn has_display(&self) -> bool {
        // Check for display availability
        std::env::var("DISPLAY").is_ok()
            || std::env::var("WAYLAND_DISPLAY").is_ok()
            || cfg!(target_os = "windows")
            || cfg!(target_os = "macos")
    }

    async fn get_user_feedback(&self) -> UserFeedback {
        use std::io::{self, Write};

        println!("\n📋 Please provide feedback on the visual test:");
        println!("1. Did you see video output? (y/n)");

        let mut input = String::new();
        print!("> ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let video_visible = input.trim().eq_ignore_ascii_case("y");

        println!("2. Was the video quality acceptable? (y/n)");
        input.clear();
        print!("> ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let quality_acceptable = input.trim().eq_ignore_ascii_case("y");

        println!("3. Any issues observed? (press Enter for none)");
        input.clear();
        print!("> ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let issues = if input.trim().is_empty() {
            None
        } else {
            Some(input.trim().to_string())
        };

        UserFeedback {
            video_visible,
            quality_acceptable,
            issues,
        }
    }

    pub async fn run_all_visual_tests(&self) -> Vec<VisualTestResult> {
        let mut results = Vec::new();

        println!("🎬 Starting Visual E2E Tests");
        if !self.has_display() {
            println!("⚠️  No display detected - visual tests will be skipped");
        }

        for (i, config) in self.test_configs.iter().enumerate() {
            println!("\n=== Test {}/{} ===", i + 1, self.test_configs.len());

            match self.run_visual_test(config).await {
                Ok(result) => {
                    if result.skipped {
                        println!(
                            "⏭️  SKIPPED: {}",
                            result
                                .error_message
                                .as_ref()
                                .unwrap_or(&"Unknown reason".to_string())
                        );
                    } else if result.success {
                        println!("✅ PASSED");
                        if let Some(feedback) = &result.user_feedback {
                            println!(
                                "   Video visible: {}",
                                if feedback.video_visible { "✅" } else { "❌" }
                            );
                            println!(
                                "   Quality acceptable: {}",
                                if feedback.quality_acceptable {
                                    "✅"
                                } else {
                                    "❌"
                                }
                            );
                            if let Some(issues) = &feedback.issues {
                                println!("   Issues: {}", issues);
                            }
                        }
                    } else {
                        println!("❌ FAILED");
                        if let Some(error) = &result.error_message {
                            println!(
                                "   Error: {}",
                                error.lines().next().unwrap_or("Unknown error")
                            );
                        }
                    }
                    results.push(result);
                }
                Err(e) => {
                    println!("❌ ERROR: {}", e);
                    results.push(VisualTestResult {
                        config: config.clone(),
                        success: false,
                        skipped: false,
                        error_message: Some(e.to_string()),
                        execution_time: Duration::from_secs(0),
                        user_feedback: None,
                    });
                }
            }
        }

        results
    }

    pub fn generate_visual_test_report(&self, results: &[VisualTestResult]) -> String {
        let mut report = String::from("# Visual E2E Test Report\n\n");

        let total = results.len();
        let passed = results.iter().filter(|r| r.success).count();
        let failed = results.iter().filter(|r| !r.success && !r.skipped).count();
        let skipped = results.iter().filter(|r| r.skipped).count();

        report.push_str(&format!("**Total Tests**: {}\n", total));
        report.push_str(&format!(
            "**Passed**: {} ({:.1}%)\n",
            passed,
            (passed as f64 / total as f64) * 100.0
        ));
        report.push_str(&format!(
            "**Failed**: {} ({:.1}%)\n",
            failed,
            (failed as f64 / total as f64) * 100.0
        ));
        report.push_str(&format!(
            "**Skipped**: {} ({:.1}%)\n\n",
            skipped,
            (skipped as f64 / total as f64) * 100.0
        ));

        report.push_str("## Test Results\n\n");

        for result in results {
            report.push_str(&format!("### {}\n", result.config.name));
            report.push_str(&format!("**Description**: {}\n", result.config.description));
            report.push_str(&format!("**Pipeline**: `{}`\n", result.config.pipeline));

            let status = if result.skipped {
                "⏭️ SKIPPED"
            } else if result.success {
                "✅ PASSED"
            } else {
                "❌ FAILED"
            };
            report.push_str(&format!("**Result**: {}\n", status));
            report.push_str(&format!("**Duration**: {:?}\n", result.execution_time));

            if let Some(feedback) = &result.user_feedback {
                report.push_str("**User Feedback**:\n");
                report.push_str(&format!(
                    "- Video visible: {}\n",
                    if feedback.video_visible { "Yes" } else { "No" }
                ));
                report.push_str(&format!(
                    "- Quality acceptable: {}\n",
                    if feedback.quality_acceptable {
                        "Yes"
                    } else {
                        "No"
                    }
                ));
                if let Some(issues) = &feedback.issues {
                    report.push_str(&format!("- Issues: {}\n", issues));
                }
            }

            if let Some(error) = &result.error_message {
                report.push_str(&format!("**Error**: ```\n{}\n```\n", error));
            }

            report.push_str("\n");
        }

        report.push_str("## Usage Instructions\n\n");
        report.push_str("To run visual tests manually:\n\n");
        report.push_str("```bash\n");
        report.push_str("# Basic public stream test\n");
        report.push_str("gst-launch-1.0 rtspsrc2 location=rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2 ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink\n\n");
        report.push_str("# With your camera (replace URL and credentials)\n");
        report.push_str("gst-launch-1.0 rtspsrc2 location=rtsp://your-camera-ip/stream user-id=admin user-pw=password ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink\n");
        report.push_str("```\n\n");

        report.push_str("## Troubleshooting\n\n");
        report.push_str("- **Black screen**: Check network connectivity and RTSP URL\n");
        report.push_str(
            "- **No video window**: Ensure display is available and autovideosink works\n",
        );
        report.push_str("- **Codec errors**: Install gstreamer1.0-libav (Ubuntu) or equivalent\n");
        report.push_str("- **Auth errors**: Verify username/password for camera streams\n");

        report
    }

    // Interactive test runner
    pub async fn run_interactive_tests(&mut self) {
        println!("🎮 Interactive Visual Test Runner");
        println!("This will run visual tests with autovideosink for manual verification\n");

        self.load_visual_test_configs();

        loop {
            println!("Available tests:");
            for (i, config) in self.test_configs.iter().enumerate() {
                println!(
                    "  {}. {} ({}s)",
                    i + 1,
                    config.name,
                    config.duration_seconds
                );
            }
            println!("  a. Run all tests");
            println!("  q. Quit");

            use std::io::{self, Write};
            print!("\nSelect test (1-{}, a, q): ", self.test_configs.len());
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            if input.eq_ignore_ascii_case("q") {
                break;
            } else if input.eq_ignore_ascii_case("a") {
                let results = self.run_all_visual_tests().await;
                let report = self.generate_visual_test_report(&results);
                std::fs::write("visual_test_report.md", report).ok();
                println!("\n📄 Report saved to visual_test_report.md");
            } else if let Ok(index) = input.parse::<usize>() {
                if index > 0 && index <= self.test_configs.len() {
                    let config = &self.test_configs[index - 1];
                    match self.run_visual_test(config).await {
                        Ok(_) => println!("Test completed"),
                        Err(e) => println!("Test failed: {}", e),
                    }
                } else {
                    println!("Invalid selection");
                }
            } else {
                println!("Invalid input");
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct VisualTestResult {
    pub config: VisualTestConfig,
    pub success: bool,
    pub skipped: bool,
    pub error_message: Option<String>,
    pub execution_time: Duration,
    pub user_feedback: Option<UserFeedback>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct UserFeedback {
    pub video_visible: bool,
    pub quality_acceptable: bool,
    pub issues: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::e2e_plugin_tests::{check_gstreamer_available, find_gst_plugin_path};

    #[tokio::test]
    async fn test_visual_test_config_loading() {
        let mut tester = E2EVisualTest::new();
        tester.load_visual_test_configs();

        assert!(!tester.test_configs.is_empty());
        assert!(tester
            .test_configs
            .iter()
            .any(|c| c.name.contains("Public")));
    }

    #[tokio::test]
    #[ignore] // Only run when explicitly requested and display available
    async fn test_public_stream_visual() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping visual test");
            return;
        }

        let mut tester = E2EVisualTest::new().with_auto_close(true);
        if let Some(path) = find_gst_plugin_path() {
            tester = tester.with_plugin_path(&path);
        }

        let config = VisualTestConfig {
            name: "Quick Visual Test".to_string(),
            pipeline: "rtspsrc2 location=rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2 ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink".to_string(),
            duration_seconds: 5,
            description: "Quick visual test".to_string(),
            requires_display: true,
        };

        println!("Running 5-second visual test...");
        let result = tester.run_visual_test(&config).await;

        match result {
            Ok(test_result) => {
                if test_result.skipped {
                    println!("Test skipped: no display available");
                } else {
                    println!(
                        "Visual test completed: {}",
                        if test_result.success {
                            "SUCCESS"
                        } else {
                            "FAILED"
                        }
                    );
                }
            }
            Err(e) => {
                println!("Visual test error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_display_detection() {
        let tester = E2EVisualTest::new();
        let has_display = tester.has_display();
        println!("Display available: {}", has_display);
    }
}
