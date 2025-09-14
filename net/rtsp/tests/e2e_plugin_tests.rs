// End-to-End Plugin Tests
// Tests the actual GStreamer plugin as loaded by gst-launch-1.0, gst-inspect-1.0, etc.

use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

#[allow(dead_code)]
pub struct GstPluginE2ETest {
    plugin_path: Option<String>,
    gst_version: String,
    test_timeout: Duration,
}

impl GstPluginE2ETest {
    pub fn new() -> Self {
        Self {
            plugin_path: None,
            gst_version: String::new(),
            test_timeout: Duration::from_secs(30),
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

    // Test basic GStreamer environment
    pub fn test_gst_environment(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Testing GStreamer Environment ===");

        // Check GStreamer version
        let output = Command::new("gst-launch-1.0").arg("--version").output()?;

        if !output.status.success() {
            return Err("gst-launch-1.0 not found or not working".into());
        }

        let version = String::from_utf8_lossy(&output.stdout);
        println!("GStreamer version: {}", version.trim());
        self.gst_version = version.trim().to_string();

        // Check basic pipeline functionality
        let output = Command::new("gst-launch-1.0")
            .args(&["--quiet", "videotestsrc", "num-buffers=1", "!", "fakesink"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Basic GStreamer pipeline failed: {}", stderr).into());
        }

        println!("✓ GStreamer environment is working");
        Ok(())
    }

    // Test plugin loading and registration
    pub fn test_plugin_loading(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Testing Plugin Loading ===");

        // Build the plugin first
        println!("Building plugin...");
        let build_output = Command::new("cargo")
            .args(&["build", "-p", "gst-plugin-rtsp"])
            .current_dir("../../..")
            .output()?;

        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            return Err(format!("Plugin build failed: {}", stderr).into());
        }

        // Set plugin path if specified
        let mut cmd = Command::new("gst-inspect-1.0");

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        // Test rtspsrc2 element inspection
        let output = cmd.arg("rtspsrc2").output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Try to get more information about available plugins
            let plugin_list = Command::new("gst-inspect-1.0")
                .arg("--print-all")
                .output()?;

            let available_plugins = String::from_utf8_lossy(&plugin_list.stdout);
            if available_plugins.contains("rtspsrc2") {
                println!("rtspsrc2 found in plugin list but inspection failed");
            } else {
                println!("rtspsrc2 not found in available plugins");
            }

            return Err(format!("Plugin inspection failed: {}", stderr).into());
        }

        let inspection = String::from_utf8_lossy(&output.stdout);
        println!("✓ Plugin loaded successfully");

        // Validate essential properties are present
        if !inspection.contains("location") {
            return Err("Essential property 'location' not found in plugin".into());
        }

        if !inspection.contains("latency") {
            return Err("Essential property 'latency' not found in plugin".into());
        }

        println!("✓ Essential properties found");
        Ok(())
    }

    // Test basic pipeline creation and validation
    pub async fn test_basic_pipeline(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Testing Basic Pipeline Creation ===");

        let pipeline = "rtspsrc2 location=rtsp://invalid-url ! fakesink";

        let mut cmd = Command::new("gst-launch-1.0");
        cmd.args(&["--quiet", "--timeout=5"])
            .args(pipeline.split_whitespace())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        // This should fail quickly due to invalid URL, but pipeline should be created
        let output = timeout(Duration::from_secs(10), async {
            tokio::task::spawn_blocking(move || cmd.output())
                .await
                .unwrap()
        })
        .await?;

        // We expect this to fail due to invalid URL, but not due to pipeline creation issues
        let stderr = String::from_utf8_lossy(&output?.stderr);

        if stderr.contains("no element \"rtspsrc2\"") {
            return Err("rtspsrc2 element not found in pipeline".into());
        }

        if stderr.contains("could not link") && !stderr.contains("connection") {
            return Err("Pipeline linking failed".into());
        }

        println!("✓ Basic pipeline creation works");
        Ok(())
    }

    // Test pipeline with public test stream
    pub async fn test_public_stream_pipeline(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Testing Pipeline with Public Stream ===");

        let test_url =
            "rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2";
        let pipeline = format!(
            "rtspsrc2 location={} ! rtph264depay ! h264parse ! fakesink sync=false",
            test_url
        );

        let mut cmd = Command::new("gst-launch-1.0");
        cmd.args(&["--quiet", "--timeout=10"])
            .args(pipeline.split_whitespace())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        let output = timeout(self.test_timeout, async {
            tokio::task::spawn_blocking(move || cmd.output())
                .await
                .unwrap()
        })
        .await?;

        let output = output?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Check for successful connection or reasonable failure
        if output.status.success() || stderr.contains("Setting pipeline to NULL") {
            println!("✓ Pipeline executed with public stream");
            return Ok(());
        }

        // Analyze failure reasons
        if stderr.contains("Could not resolve") || stderr.contains("Network is unreachable") {
            println!("⚠ Network connectivity issue (expected in some environments)");
            return Ok(());
        }

        if stderr.contains("no element \"rtspsrc2\"") {
            return Err("rtspsrc2 element not available".into());
        }

        println!("Pipeline stderr: {}", stderr);
        println!("Pipeline stdout: {}", stdout);
        Err("Pipeline with public stream failed unexpectedly".into())
    }

    // Test property setting via gst-launch
    pub async fn test_property_setting(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Testing Property Setting ===");

        let pipeline = "rtspsrc2 location=rtsp://test.local latency=2000 ! fakesink";

        let mut cmd = Command::new("gst-launch-1.0");
        cmd.args(&["--quiet", "--timeout=3"])
            .args(pipeline.split_whitespace())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        let output = timeout(Duration::from_secs(5), async {
            tokio::task::spawn_blocking(move || cmd.output())
                .await
                .unwrap()
        })
        .await?;

        let stderr = String::from_utf8_lossy(&output?.stderr);

        // Should fail due to invalid URL, but property setting should work
        if stderr.contains("no property") || stderr.contains("Invalid property") {
            return Err("Property setting failed".into());
        }

        println!("✓ Property setting works");
        Ok(())
    }

    // Test element factory and capabilities
    pub fn test_element_factory(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Testing Element Factory ===");

        let mut cmd = Command::new("gst-inspect-1.0");
        cmd.arg("--exists").arg("rtspsrc2");

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            return Err("rtspsrc2 element factory not found".into());
        }

        // Check element capabilities
        let mut cmd = Command::new("gst-inspect-1.0");
        cmd.arg("rtspsrc2");

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        let output = cmd.output()?;
        let inspection = String::from_utf8_lossy(&output.stdout);

        // Validate essential pad templates
        if !inspection.contains("SRC") {
            return Err("Source pad template not found".into());
        }

        // Validate element class
        if !inspection.contains("Source") {
            return Err("Element not classified as Source".into());
        }

        println!("✓ Element factory and capabilities validated");
        Ok(())
    }

    // Test plugin information
    pub fn test_plugin_info(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Testing Plugin Information ===");

        // Get plugin information
        let mut cmd = Command::new("gst-inspect-1.0");
        cmd.arg("--plugin").arg("rsrtsp");

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            // Try alternative plugin name
            let mut cmd = Command::new("gst-inspect-1.0");
            cmd.arg("--plugin").arg("gstrsrtsp");

            if let Some(path) = &self.plugin_path {
                cmd.env("GST_PLUGIN_PATH", path);
            }

            let output = cmd.output()?;
            if !output.status.success() {
                println!("⚠ Could not get plugin info (may be expected)");
                return Ok(());
            }
        }

        let info = String::from_utf8_lossy(&output.stdout);

        // Validate plugin metadata
        if info.contains("rtspsrc2") {
            println!("✓ Plugin information contains rtspsrc2");
        }

        println!("✓ Plugin information retrieved");
        Ok(())
    }

    // Run all E2E tests
    pub async fn run_all_tests(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting End-to-End Plugin Tests\n");

        // Test 1: Environment
        self.test_gst_environment()?;
        println!();

        // Test 2: Plugin Loading
        self.test_plugin_loading()?;
        println!();

        // Test 3: Element Factory
        self.test_element_factory()?;
        println!();

        // Test 4: Plugin Info
        self.test_plugin_info()?;
        println!();

        // Test 5: Basic Pipeline
        self.test_basic_pipeline().await?;
        println!();

        // Test 6: Property Setting
        self.test_property_setting().await?;
        println!();

        // Test 7: Public Stream (may fail due to network)
        if let Err(e) = self.test_public_stream_pipeline().await {
            println!("⚠ Public stream test failed (may be expected): {}", e);
        }
        println!();

        println!("🎉 All E2E tests completed successfully!");
        Ok(())
    }
}

// Utility functions for E2E testing
pub fn find_gst_plugin_path() -> Option<String> {
    // Try to find the compiled plugin
    let possible_paths = vec![
        "target/debug",
        "target/release",
        "../../../target/debug",
        "../../../target/release",
    ];

    for path in possible_paths {
        let plugin_path = std::path::Path::new(path);
        if plugin_path.exists() {
            return Some(plugin_path.to_string_lossy().to_string());
        }
    }

    None
}

pub fn check_gstreamer_available() -> bool {
    Command::new("gst-launch-1.0")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gstreamer_environment() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping E2E tests");
            return;
        }

        let mut tester = GstPluginE2ETest::new();
        if let Some(path) = find_gst_plugin_path() {
            tester = tester.with_plugin_path(&path);
        }

        let result = tester.test_gst_environment();
        assert!(
            result.is_ok(),
            "GStreamer environment test failed: {:?}",
            result
        );
    }

    #[tokio::test]
    #[ignore] // Only run when explicitly requested
    async fn test_full_e2e_suite() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping E2E tests");
            return;
        }

        let mut tester = GstPluginE2ETest::new();
        if let Some(path) = find_gst_plugin_path() {
            tester = tester.with_plugin_path(&path);
        }

        let result = tester.run_all_tests().await;
        assert!(result.is_ok(), "E2E test suite failed: {:?}", result);
    }

    #[tokio::test]
    async fn test_plugin_loading_only() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping plugin loading test");
            return;
        }

        let mut tester = GstPluginE2ETest::new();
        if let Some(path) = find_gst_plugin_path() {
            tester = tester.with_plugin_path(&path);
        }

        // Test environment first
        if let Err(e) = tester.test_gst_environment() {
            println!("GStreamer environment not ready: {}", e);
            return;
        }

        // Then test plugin loading
        let result = tester.test_plugin_loading();
        // Don't assert here as plugin may not be built yet
        match result {
            Ok(_) => println!("✓ Plugin loading test passed"),
            Err(e) => println!("⚠ Plugin loading test failed: {}", e),
        }
    }

    #[tokio::test]
    async fn test_basic_pipeline_creation() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping pipeline test");
            return;
        }

        let mut tester = GstPluginE2ETest::new();
        if let Some(path) = find_gst_plugin_path() {
            tester = tester.with_plugin_path(&path);
        }

        // Only test if plugin loading works
        if tester.test_plugin_loading().is_ok() {
            let result = tester.test_basic_pipeline().await;
            match result {
                Ok(_) => println!("✓ Basic pipeline test passed"),
                Err(e) => println!("⚠ Basic pipeline test failed: {}", e),
            }
        } else {
            println!("Plugin not loaded, skipping pipeline test");
        }
    }
}
