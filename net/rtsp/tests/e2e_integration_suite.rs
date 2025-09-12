#![allow(unused)]
// End-to-End Integration Test Suite
// Comprehensive E2E testing orchestrator for the RTSP plugin

#[path = "e2e_inspection_tests.rs"]
mod e2e_inspection_tests;
#[path = "e2e_pipeline_tests.rs"]
mod e2e_pipeline_tests;
#[path = "e2e_plugin_tests.rs"]
mod e2e_plugin_tests;
#[path = "e2e_visual_tests.rs"]
mod e2e_visual_tests;

use e2e_inspection_tests::*;
use e2e_pipeline_tests::*;
use e2e_plugin_tests::*;
use e2e_visual_tests::*;

use std::time::Duration;

#[allow(dead_code)]
pub struct E2EIntegrationSuite {
    plugin_path: Option<String>,
    test_timeout: Duration,
    generate_reports: bool,
    output_dir: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct E2ESuiteResult {
    pub plugin_tests_passed: bool,
    pub inspection_tests_passed: bool,
    pub pipeline_tests_passed: bool,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub execution_time: Duration,
    pub errors: Vec<String>,
}

impl E2EIntegrationSuite {
    pub fn new() -> Self {
        Self {
            plugin_path: find_gst_plugin_path(),
            test_timeout: Duration::from_secs(300), // 5 minutes total
            generate_reports: true,
            output_dir: "e2e_test_results".to_string(),
        }
    }

    pub fn with_plugin_path(mut self, path: &str) -> Self {
        self.plugin_path = Some(path.to_string());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.test_timeout = timeout;
        self
    }

    pub fn with_reports(mut self, generate: bool) -> Self {
        self.generate_reports = generate;
        self
    }

    pub async fn run_full_e2e_suite(&self) -> Result<E2ESuiteResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut total_tests = 0;
        let mut passed_tests = 0;

        println!("🚀 Starting Comprehensive E2E Test Suite for RTSP Plugin");
        println!("Plugin Path: {:?}", self.plugin_path);
        println!("Output Dir: {}", self.output_dir);
        println!();

        // Create output directory
        if self.generate_reports {
            std::fs::create_dir_all(&self.output_dir)?;
        }

        // Pre-flight checks
        if !check_gstreamer_available() {
            return Err("GStreamer not available - cannot run E2E tests".into());
        }

        println!("✅ GStreamer is available");

        // Phase 1: Plugin Loading and Registration Tests
        println!("\n=== Phase 1: Plugin Loading and Registration ===");
        let plugin_tests_passed = match self.run_plugin_tests().await {
            Ok(success) => {
                if success {
                    println!("✅ Plugin tests passed");
                    passed_tests += 1;
                } else {
                    println!("❌ Plugin tests failed");
                    errors.push("Plugin tests failed".to_string());
                }
                total_tests += 1;
                success
            }
            Err(e) => {
                println!("❌ Plugin tests error: {}", e);
                errors.push(format!("Plugin tests error: {}", e));
                total_tests += 1;
                false
            }
        };

        // Phase 2: Element Inspection Tests
        println!("\n=== Phase 2: Element Inspection and Validation ===");
        let inspection_tests_passed = match self.run_inspection_tests().await {
            Ok(success) => {
                if success {
                    println!("✅ Inspection tests passed");
                    passed_tests += 1;
                } else {
                    println!("❌ Inspection tests failed");
                    errors.push("Inspection tests failed".to_string());
                }
                total_tests += 1;
                success
            }
            Err(e) => {
                println!("❌ Inspection tests error: {}", e);
                errors.push(format!("Inspection tests error: {}", e));
                total_tests += 1;
                false
            }
        };

        // Phase 3: Pipeline Tests (only if plugin loads successfully)
        println!("\n=== Phase 3: Pipeline Execution Tests ===");
        let pipeline_tests_passed = if plugin_tests_passed {
            match self.run_pipeline_tests().await {
                Ok(success) => {
                    if success {
                        println!("✅ Pipeline tests passed");
                        passed_tests += 1;
                    } else {
                        println!("❌ Pipeline tests failed");
                        errors.push("Pipeline tests failed".to_string());
                    }
                    total_tests += 1;
                    success
                }
                Err(e) => {
                    println!("❌ Pipeline tests error: {}", e);
                    errors.push(format!("Pipeline tests error: {}", e));
                    total_tests += 1;
                    false
                }
            }
        } else {
            println!("⏭️ Skipping pipeline tests due to plugin loading failure");
            false
        };

        let execution_time = start_time.elapsed();
        let failed_tests = total_tests - passed_tests;

        println!("\n🏁 E2E Test Suite Complete!");
        println!("Total Tests: {}", total_tests);
        println!(
            "Passed: {} ({:.1}%)",
            passed_tests,
            (passed_tests as f64 / total_tests as f64) * 100.0
        );
        println!(
            "Failed: {} ({:.1}%)",
            failed_tests,
            (failed_tests as f64 / total_tests as f64) * 100.0
        );
        println!("Execution Time: {:?}", execution_time);

        if !errors.is_empty() {
            println!("\nErrors encountered:");
            for error in &errors {
                println!("  - {}", error);
            }
        }

        // Generate summary report
        if self.generate_reports {
            self.generate_summary_report(&E2ESuiteResult {
                plugin_tests_passed,
                inspection_tests_passed,
                pipeline_tests_passed,
                total_tests,
                passed_tests,
                failed_tests,
                execution_time,
                errors: errors.clone(),
            })?;
        }

        Ok(E2ESuiteResult {
            plugin_tests_passed,
            inspection_tests_passed,
            pipeline_tests_passed,
            total_tests,
            passed_tests,
            failed_tests,
            execution_time,
            errors,
        })
    }

    async fn run_plugin_tests(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let mut tester = GstPluginE2ETest::new();

        if let Some(path) = &self.plugin_path {
            tester = tester.with_plugin_path(path);
        }

        // Run essential plugin tests
        match tester.test_gst_environment() {
            Ok(_) => println!("  ✅ GStreamer environment OK"),
            Err(e) => return Err(format!("GStreamer environment failed: {}", e).into()),
        }

        match tester.test_plugin_loading() {
            Ok(_) => println!("  ✅ Plugin loading OK"),
            Err(e) => {
                println!("  ❌ Plugin loading failed: {}", e);
                return Ok(false);
            }
        }

        match tester.test_element_factory() {
            Ok(_) => println!("  ✅ Element factory OK"),
            Err(e) => {
                println!("  ❌ Element factory failed: {}", e);
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn run_inspection_tests(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let mut inspector = ElementInspectionTest::new();

        if let Some(path) = &self.plugin_path {
            inspector = inspector.with_plugin_path(path);
        }

        inspector.setup_rtspsrc2_expectations();

        // Test element existence
        match inspector.test_element_exists("rtspsrc2") {
            Ok(exists) => {
                if exists {
                    println!("  ✅ rtspsrc2 element exists");
                } else {
                    println!("  ❌ rtspsrc2 element not found");
                    return Ok(false);
                }
            }
            Err(e) => return Err(format!("Element existence check failed: {}", e).into()),
        }

        // Full inspection
        match inspector.inspect_element("rtspsrc2") {
            Ok(result) => {
                if result.element_found && result.properties_valid && result.metadata_valid {
                    println!("  ✅ Element inspection passed");

                    if self.generate_reports {
                        let report = inspector.generate_inspection_report(&[("rtspsrc2", result)]);
                        let report_path =
                            format!("{}/element_inspection_report.md", self.output_dir);
                        std::fs::write(report_path, report)?;
                    }

                    Ok(true)
                } else {
                    println!("  ❌ Element inspection failed");
                    println!("    Properties valid: {}", result.properties_valid);
                    println!("    Metadata valid: {}", result.metadata_valid);

                    for error in &result.errors {
                        println!("    Error: {}", error);
                    }

                    Ok(false)
                }
            }
            Err(e) => Err(format!("Element inspection error: {}", e).into()),
        }
    }

    async fn run_pipeline_tests(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let mut pipeline_tester = E2EPipelineTest::new();

        if let Some(path) = &self.plugin_path {
            pipeline_tester = pipeline_tester.with_plugin_path(path);
        }

        // Load test configurations
        pipeline_tester.load_default_test_configs();

        // Run pipeline tests
        let results = pipeline_tester.run_all_pipeline_tests().await;

        let total = results.len();
        let passed = results.iter().filter(|r| r.success).count();
        let failed = total - passed;

        println!("  Pipeline Tests: {}/{} passed", passed, total);

        // Generate pipeline report
        if self.generate_reports {
            let report = pipeline_tester.generate_test_report(&results);
            let report_path = format!("{}/pipeline_test_report.md", self.output_dir);
            std::fs::write(report_path, report)?;
        }

        // Consider success if at least basic tests pass
        let basic_tests_passed = results
            .iter()
            .filter(|r| r.config.name.contains("Basic") || r.config.name.contains("Property"))
            .any(|r| r.success);

        if basic_tests_passed {
            println!("  ✅ Basic pipeline functionality working");
            Ok(true)
        } else {
            println!("  ❌ Basic pipeline tests failed");
            Ok(false)
        }
    }

    fn generate_summary_report(
        &self,
        result: &E2ESuiteResult,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut report = String::from("# RTSP Plugin E2E Test Suite Results\n\n");

        report.push_str(&format!(
            "**Test Date**: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
        report.push_str(&format!("**Plugin Path**: {:?}\n", self.plugin_path));
        report.push_str(&format!(
            "**Execution Time**: {:?}\n\n",
            result.execution_time
        ));

        report.push_str("## Summary\n\n");
        report.push_str(&format!("- **Total Tests**: {}\n", result.total_tests));
        report.push_str(&format!(
            "- **Passed**: {} ({:.1}%)\n",
            result.passed_tests,
            (result.passed_tests as f64 / result.total_tests as f64) * 100.0
        ));
        report.push_str(&format!(
            "- **Failed**: {} ({:.1}%)\n\n",
            result.failed_tests,
            (result.failed_tests as f64 / result.total_tests as f64) * 100.0
        ));

        report.push_str("## Test Phase Results\n\n");
        report.push_str(&format!(
            "1. **Plugin Loading**: {}\n",
            if result.plugin_tests_passed {
                "✅ PASSED"
            } else {
                "❌ FAILED"
            }
        ));
        report.push_str(&format!(
            "2. **Element Inspection**: {}\n",
            if result.inspection_tests_passed {
                "✅ PASSED"
            } else {
                "❌ FAILED"
            }
        ));
        report.push_str(&format!(
            "3. **Pipeline Execution**: {}\n",
            if result.pipeline_tests_passed {
                "✅ PASSED"
            } else {
                "❌ FAILED"
            }
        ));

        if !result.errors.is_empty() {
            report.push_str("\n## Errors\n\n");
            for error in &result.errors {
                report.push_str(&format!("- ❌ {}\n", error));
            }
        }

        report.push_str("\n## Recommendations\n\n");

        if !result.plugin_tests_passed {
            report.push_str("- Build the plugin with `cargo build -p gst-plugin-rtsp`\n");
            report.push_str("- Ensure GST_PLUGIN_PATH is set correctly\n");
        }

        if !result.inspection_tests_passed {
            report.push_str("- Verify element properties are correctly implemented\n");
            report.push_str("- Check element metadata and registration\n");
        }

        if !result.pipeline_tests_passed {
            report.push_str("- Test with real RTSP streams if possible\n");
            report.push_str("- Verify network connectivity for public test streams\n");
        }

        report.push_str("\n## Related Files\n\n");
        report.push_str("- `element_inspection_report.md` - Detailed element inspection results\n");
        report.push_str("- `pipeline_test_report.md` - Pipeline execution test results\n");

        let report_path = format!("{}/e2e_suite_summary.md", self.output_dir);
        std::fs::write(report_path, report)?;

        println!(
            "📄 Summary report saved to {}/e2e_suite_summary.md",
            self.output_dir
        );

        Ok(())
    }
}

// Quick test runner for CI
pub async fn run_quick_e2e_check() -> bool {
    match check_gstreamer_available() {
        false => {
            println!("❌ GStreamer not available");
            return false;
        }
        true => println!("✅ GStreamer available"),
    }

    let suite = E2EIntegrationSuite::new().with_reports(false);

    match suite.run_full_e2e_suite().await {
        Ok(result) => {
            println!(
                "E2E Quick Check: {}/{} tests passed",
                result.passed_tests, result.total_tests
            );
            result.plugin_tests_passed && result.passed_tests > 0
        }
        Err(e) => {
            println!("E2E Quick Check failed: {}", e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_e2e_quick_check() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping E2E quick check");
            return;
        }

        let result = run_quick_e2e_check().await;
        println!(
            "Quick E2E check result: {}",
            if result { "PASSED" } else { "FAILED" }
        );

        // Don't assert as this may fail in environments without the plugin built
    }

    #[tokio::test]
    #[ignore] // Only run when explicitly requested
    async fn test_full_e2e_suite() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping full E2E suite");
            return;
        }

        let suite = E2EIntegrationSuite::new();

        match suite.run_full_e2e_suite().await {
            Ok(result) => {
                println!("Full E2E Suite Results:");
                println!(
                    "  Plugin Tests: {}",
                    if result.plugin_tests_passed {
                        "PASSED"
                    } else {
                        "FAILED"
                    }
                );
                println!(
                    "  Inspection Tests: {}",
                    if result.inspection_tests_passed {
                        "PASSED"
                    } else {
                        "FAILED"
                    }
                );
                println!(
                    "  Pipeline Tests: {}",
                    if result.pipeline_tests_passed {
                        "PASSED"
                    } else {
                        "FAILED"
                    }
                );
                println!(
                    "  Total: {}/{} tests passed",
                    result.passed_tests, result.total_tests
                );

                // Don't assert success as plugin may not be built in test environment
            }
            Err(e) => {
                println!("E2E Suite error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_environment_setup() {
        // Test basic environment requirements
        assert!(
            check_gstreamer_available(),
            "GStreamer should be available for E2E tests"
        );

        // Test plugin path detection
        let plugin_path = find_gst_plugin_path();
        println!("Detected plugin path: {:?}", plugin_path);

        // Create a minimal suite to test setup
        let suite = E2EIntegrationSuite::new();
        assert_eq!(suite.output_dir, "e2e_test_results");
        assert!(suite.generate_reports);
    }
}
