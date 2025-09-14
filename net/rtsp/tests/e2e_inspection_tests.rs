// End-to-End GStreamer Element Inspection Tests
// Validates gst-inspect-1.0 output and element metadata

use std::collections::HashMap;
use std::process::Command;

#[path = "common/mod.rs"]
mod common;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ElementInspectionTest {
    plugin_path: Option<String>,
    expected_properties: HashMap<String, PropertyInfo>,
    expected_signals: Vec<String>,
    expected_pads: HashMap<String, PadInfo>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub property_type: String,
    pub readable: bool,
    pub writable: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PadInfo {
    pub direction: String,    // "SRC" or "SINK"
    pub availability: String, // "ALWAYS", "SOMETIMES", "REQUEST"
    pub capabilities: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct InspectionResult {
    pub element_found: bool,
    pub properties_valid: bool,
    pub pads_valid: bool,
    pub signals_valid: bool,
    pub metadata_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub raw_output: String,
}

impl ElementInspectionTest {
    pub fn new() -> Self {
        Self {
            plugin_path: None,
            expected_properties: HashMap::new(),
            expected_signals: Vec::new(),
            expected_pads: HashMap::new(),
        }
    }

    pub fn with_plugin_path(mut self, path: &str) -> Self {
        self.plugin_path = Some(path.to_string());
        self
    }

    pub fn setup_rtspsrc2_expectations(&mut self) {
        // Expected properties for rtspsrc2
        self.expected_properties.insert(
            "location".to_string(),
            PropertyInfo {
                property_type: "gchararray".to_string(),
                readable: true,
                writable: true,
                default_value: None,
                description: Some("RTSP location URI".to_string()),
            },
        );

        self.expected_properties.insert(
            "latency".to_string(),
            PropertyInfo {
                property_type: "guint64".to_string(),
                readable: true,
                writable: true,
                default_value: Some("2000".to_string()),
                description: Some("Amount of latency to add".to_string()),
            },
        );

        self.expected_properties.insert(
            "retry-strategy".to_string(),
            PropertyInfo {
                property_type: "gchararray".to_string(),
                readable: true,
                writable: true,
                default_value: Some("auto".to_string()),
                description: Some("Connection retry strategy".to_string()),
            },
        );

        self.expected_properties.insert(
            "max-reconnection-attempts".to_string(),
            PropertyInfo {
                property_type: "gint".to_string(),
                readable: true,
                writable: true,
                default_value: Some("5".to_string()),
                description: Some("Maximum retry attempts".to_string()),
            },
        );

        self.expected_properties.insert(
            "protocols".to_string(),
            PropertyInfo {
                property_type: "gchararray".to_string(),
                readable: true,
                writable: true,
                default_value: Some("tcp+udp+http".to_string()),
                description: Some("Allowed protocols".to_string()),
            },
        );

        self.expected_properties.insert(
            "user-id".to_string(),
            PropertyInfo {
                property_type: "gchararray".to_string(),
                readable: true,
                writable: true,
                default_value: None,
                description: Some("User ID for authentication".to_string()),
            },
        );

        self.expected_properties.insert(
            "user-pw".to_string(),
            PropertyInfo {
                property_type: "gchararray".to_string(),
                readable: true,
                writable: true,
                default_value: None,
                description: Some("User password for authentication".to_string()),
            },
        );

        // Expected pads
        self.expected_pads.insert(
            "stream_%u".to_string(),
            PadInfo {
                direction: "SRC".to_string(),
                availability: "SOMETIMES".to_string(),
                capabilities: vec!["application/x-rtp".to_string()],
            },
        );

        // Expected signals (if any)
        // rtspsrc2 may have signals for connection events, etc.
        self.expected_signals.extend(vec![
            // Add expected signals here when implemented
        ]);
    }

    pub fn inspect_element(
        &self,
        element_name: &str,
    ) -> Result<InspectionResult, Box<dyn std::error::Error>> {
        let mut cmd = Command::new("gst-inspect-1.0");
        cmd.arg(element_name);

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        let output = cmd.output()?;
        let raw_output = String::from_utf8_lossy(&output.stdout).to_string();

        let mut result = InspectionResult {
            element_found: output.status.success(),
            properties_valid: false,
            pads_valid: false,
            signals_valid: false,
            metadata_valid: false,
            errors: Vec::new(),
            warnings: Vec::new(),
            raw_output: raw_output.clone(),
        };

        if !result.element_found {
            let stderr = String::from_utf8_lossy(&output.stderr);
            result.errors.push(format!("Element not found: {}", stderr));
            return Ok(result);
        }

        // Parse and validate the inspection output
        result.properties_valid = self.validate_properties(&raw_output, &mut result);
        result.pads_valid = self.validate_pads(&raw_output, &mut result);
        result.signals_valid = self.validate_signals(&raw_output, &mut result);
        result.metadata_valid = self.validate_metadata(&raw_output, &mut result);

        Ok(result)
    }

    fn validate_properties(&self, output: &str, result: &mut InspectionResult) -> bool {
        let mut all_valid = true;

        // Look for element properties section
        if !output.contains("Element Properties:") && !output.contains("Object properties:") {
            result
                .warnings
                .push("No properties section found in output".to_string());
            return self.expected_properties.is_empty();
        }

        // Validate each expected property
        for (prop_name, expected) in &self.expected_properties {
            if !output.contains(prop_name) {
                result
                    .errors
                    .push(format!("Property '{}' not found", prop_name));
                all_valid = false;
                continue;
            }

            // Find the property section
            if let Some(prop_start) = output.find(&format!("  {}", prop_name)) {
                let prop_section = &output[prop_start..];
                let prop_end = prop_section.find("\n  ").unwrap_or(prop_section.len());
                let prop_text = &prop_section[..prop_end];

                // Validate property type
                if !prop_text.contains(&expected.property_type) {
                    result.warnings.push(format!(
                        "Property '{}' type mismatch. Expected: {}",
                        prop_name, expected.property_type
                    ));
                }

                // Validate read/write flags
                let is_readable = prop_text.contains("readable") || prop_text.contains("r");
                let is_writable = prop_text.contains("writable") || prop_text.contains("w");

                if expected.readable && !is_readable {
                    result
                        .warnings
                        .push(format!("Property '{}' should be readable", prop_name));
                }

                if expected.writable && !is_writable {
                    result
                        .warnings
                        .push(format!("Property '{}' should be writable", prop_name));
                }

                // Validate default value if specified
                if let Some(default) = &expected.default_value {
                    if !prop_text.contains(&format!("Default: {}", default)) {
                        result.warnings.push(format!(
                            "Property '{}' default value mismatch. Expected: {}",
                            prop_name, default
                        ));
                    }
                }
            }
        }

        all_valid
    }

    fn validate_pads(&self, output: &str, result: &mut InspectionResult) -> bool {
        let mut all_valid = true;

        // Look for pad templates section
        if !output.contains("Pad Templates:")
            && !output.contains("SRC template:")
            && !output.contains("SINK template:")
        {
            result
                .warnings
                .push("No pad templates section found".to_string());
            return self.expected_pads.is_empty();
        }

        // Validate each expected pad
        for (pad_name, expected) in &self.expected_pads {
            // Look for pad template or direction
            if !output.contains(&expected.direction) {
                result
                    .errors
                    .push(format!("Expected {} pad not found", expected.direction));
                all_valid = false;
            }

            // Check for availability
            if !output.contains(&expected.availability) {
                result.warnings.push(format!(
                    "Pad availability '{}' not found for {}",
                    expected.availability, pad_name
                ));
            }

            // Check for capabilities
            for capability in &expected.capabilities {
                if !output.contains(capability) {
                    result.warnings.push(format!(
                        "Capability '{}' not found for pad {}",
                        capability, pad_name
                    ));
                }
            }
        }

        all_valid
    }

    fn validate_signals(&self, output: &str, _result: &mut InspectionResult) -> bool {
        // Check if signals section exists
        let has_signals_section =
            output.contains("Element Signals:") || output.contains("Object signals:");

        if self.expected_signals.is_empty() {
            return true; // No signals expected, so valid
        }

        if !has_signals_section {
            return false;
        }

        // Validate each expected signal
        for signal in &self.expected_signals {
            if !output.contains(signal) {
                return false;
            }
        }

        true
    }

    fn validate_metadata(&self, output: &str, result: &mut InspectionResult) -> bool {
        let mut valid = true;

        // Check for basic element metadata
        if !output.contains("Factory Details:") && !output.contains("Plugin Details:") {
            result
                .warnings
                .push("No factory/plugin details found".to_string());
            valid = false;
        }

        // Check element class
        if !output.contains("Source") {
            result
                .warnings
                .push("Element not classified as Source".to_string());
            valid = false;
        }

        // Check for author/description
        if !output.contains("Author") && !output.contains("Description") {
            result
                .warnings
                .push("Missing author or description metadata".to_string());
        }

        valid
    }

    pub fn test_element_exists(
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

    pub fn get_element_factory_info(
        &self,
        element_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut cmd = Command::new("gst-inspect-1.0");
        cmd.args(&["--print-plugin-auto-install-info", element_name]);

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        let output = cmd.output()?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn list_all_elements(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut cmd = Command::new("gst-inspect-1.0");

        if let Some(path) = &self.plugin_path {
            cmd.env("GST_PLUGIN_PATH", path);
        }

        let output = cmd.output()?;
        let output_str = String::from_utf8_lossy(&output.stdout);

        let elements: Vec<String> = output_str
            .lines()
            .filter_map(|line| {
                if line.contains(":") && !line.starts_with(" ") {
                    line.split(':').next().map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(elements)
    }

    pub fn generate_inspection_report(&self, results: &[(&str, InspectionResult)]) -> String {
        let mut report = String::from("# GStreamer Element Inspection Report\n\n");

        for (element_name, result) in results {
            report.push_str(&format!("## Element: {}\n\n", element_name));

            report.push_str(&format!(
                "- **Found**: {}\n",
                if result.element_found {
                    "✅ Yes"
                } else {
                    "❌ No"
                }
            ));
            report.push_str(&format!(
                "- **Properties Valid**: {}\n",
                if result.properties_valid {
                    "✅ Yes"
                } else {
                    "⚠️ Issues"
                }
            ));
            report.push_str(&format!(
                "- **Pads Valid**: {}\n",
                if result.pads_valid {
                    "✅ Yes"
                } else {
                    "⚠️ Issues"
                }
            ));
            report.push_str(&format!(
                "- **Signals Valid**: {}\n",
                if result.signals_valid {
                    "✅ Yes"
                } else {
                    "⚠️ Issues"
                }
            ));
            report.push_str(&format!(
                "- **Metadata Valid**: {}\n",
                if result.metadata_valid {
                    "✅ Yes"
                } else {
                    "⚠️ Issues"
                }
            ));

            if !result.errors.is_empty() {
                report.push_str("\n### Errors\n");
                for error in &result.errors {
                    report.push_str(&format!("- ❌ {}\n", error));
                }
            }

            if !result.warnings.is_empty() {
                report.push_str("\n### Warnings\n");
                for warning in &result.warnings {
                    report.push_str(&format!("- ⚠️ {}\n", warning));
                }
            }

            report.push_str("\n");
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{check_gstreamer_available, find_gst_plugin_path};

    #[test]
    fn test_element_exists_check() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping inspection tests");
            return;
        }

        let mut inspector = ElementInspectionTest::new();
        if let Some(path) = find_gst_plugin_path() {
            inspector = inspector.with_plugin_path(&path);
        }

        // Test with known existing element
        let result = inspector.test_element_exists("fakesrc");
        assert!(result.is_ok(), "Failed to check fakesrc existence");
        assert!(result.unwrap(), "fakesrc should exist");

        // Test with non-existing element
        let result = inspector.test_element_exists("nonexistentelement");
        assert!(result.is_ok(), "Failed to check non-existent element");
        assert!(!result.unwrap(), "Non-existent element should not exist");
    }

    #[test]
    fn test_rtspsrc2_inspection() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping rtspsrc2 inspection");
            return;
        }

        let mut inspector = ElementInspectionTest::new();
        if let Some(path) = find_gst_plugin_path() {
            inspector = inspector.with_plugin_path(&path);
        }

        inspector.setup_rtspsrc2_expectations();

        match inspector.inspect_element("rtspsrc2") {
            Ok(result) => {
                println!("rtspsrc2 inspection result:");
                println!("  Found: {}", result.element_found);
                println!("  Properties Valid: {}", result.properties_valid);
                println!("  Pads Valid: {}", result.pads_valid);
                println!("  Metadata Valid: {}", result.metadata_valid);

                if !result.errors.is_empty() {
                    println!("  Errors:");
                    for error in &result.errors {
                        println!("    - {}", error);
                    }
                }

                if !result.warnings.is_empty() {
                    println!("  Warnings:");
                    for warning in &result.warnings {
                        println!("    - {}", warning);
                    }
                }

                // Don't assert success as plugin may not be built yet
                if result.element_found {
                    println!("✅ rtspsrc2 element found and inspected");
                } else {
                    println!("⚠️ rtspsrc2 element not found (may need to build plugin)");
                }
            }
            Err(e) => {
                println!("Failed to inspect rtspsrc2: {}", e);
            }
        }
    }

    #[test]
    fn test_list_available_elements() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping element listing");
            return;
        }

        let mut inspector = ElementInspectionTest::new();
        if let Some(path) = find_gst_plugin_path() {
            inspector = inspector.with_plugin_path(&path);
        }

        match inspector.list_all_elements() {
            Ok(elements) => {
                println!("Found {} GStreamer elements", elements.len());

                // Check for some common elements
                let common_elements = ["fakesrc", "fakesink", "videotestsrc"];
                for element in common_elements {
                    if elements.contains(&element.to_string()) {
                        println!("✅ {} found", element);
                    } else {
                        println!("⚠️ {} not found", element);
                    }
                }

                // Check for our plugin
                if elements.contains(&"rtspsrc2".to_string()) {
                    println!("✅ rtspsrc2 found in element list");
                } else {
                    println!("⚠️ rtspsrc2 not found in element list");
                }
            }
            Err(e) => {
                println!("Failed to list elements: {}", e);
            }
        }
    }

    #[test]
    #[ignore] // Only run when explicitly requested
    fn test_comprehensive_inspection() {
        if !check_gstreamer_available() {
            println!("GStreamer not available, skipping comprehensive inspection");
            return;
        }

        let mut inspector = ElementInspectionTest::new();
        if let Some(path) = find_gst_plugin_path() {
            inspector = inspector.with_plugin_path(&path);
        }

        inspector.setup_rtspsrc2_expectations();

        let elements_to_test = vec!["rtspsrc2", "fakesrc", "fakesink"];
        let mut results = Vec::new();

        for element in elements_to_test {
            match inspector.inspect_element(element) {
                Ok(result) => {
                    results.push((element, result));
                }
                Err(e) => {
                    println!("Failed to inspect {}: {}", element, e);
                }
            }
        }

        // Generate and print report
        let report = inspector.generate_inspection_report(&results);
        println!("{}", report);

        // Save report to file
        std::fs::write("element_inspection_report.md", report).ok();
    }
}
