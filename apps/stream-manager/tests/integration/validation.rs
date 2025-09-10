use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info, warn};

/// Validation result for a complete test run
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration: Duration,
    pub tests_run: usize,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub test_results: Vec<TestResult>,
    pub performance_metrics: PerformanceMetrics,
    pub resource_usage: ResourceUsage,
    pub issues_found: Vec<Issue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration: Duration,
    pub error_message: Option<String>,
    pub assertions: Vec<Assertion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Assertion {
    pub description: String,
    pub passed: bool,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub avg_stream_startup_time_ms: f64,
    pub avg_stream_teardown_time_ms: f64,
    pub max_concurrent_streams: usize,
    pub avg_latency_ms: f64,
    pub throughput_mbps: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub peak_cpu_percent: f32,
    pub avg_cpu_percent: f32,
    pub peak_memory_mb: f32,
    pub avg_memory_mb: f32,
    pub disk_usage_mb: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub description: String,
    pub test_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IssueCategory {
    Performance,
    Stability,
    Resource,
    Functionality,
    Configuration,
}

/// Validator for stream pipeline configurations
pub struct PipelineValidator {
    required_elements: Vec<String>,
    forbidden_elements: Vec<String>,
}

impl PipelineValidator {
    pub fn new() -> Self {
        Self {
            required_elements: vec![],
            forbidden_elements: vec![],
        }
    }
    
    pub fn require_element(mut self, element: &str) -> Self {
        self.required_elements.push(element.to_string());
        self
    }
    
    pub fn forbid_element(mut self, element: &str) -> Self {
        self.forbidden_elements.push(element.to_string());
        self
    }
    
    pub fn validate(&self, pipeline_str: &str) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Check required elements
        for required in &self.required_elements {
            if !pipeline_str.contains(required) {
                errors.push(format!("Missing required element: {}", required));
            }
        }
        
        // Check forbidden elements
        for forbidden in &self.forbidden_elements {
            if pipeline_str.contains(forbidden) {
                errors.push(format!("Contains forbidden element: {}", forbidden));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validator for API responses
pub struct ApiResponseValidator {
    expected_status: Option<u16>,
    required_fields: Vec<String>,
    field_validators: HashMap<String, Box<dyn Fn(&serde_json::Value) -> bool>>,
}

impl ApiResponseValidator {
    pub fn new() -> Self {
        Self {
            expected_status: None,
            required_fields: vec![],
            field_validators: HashMap::new(),
        }
    }
    
    pub fn expect_status(mut self, status: u16) -> Self {
        self.expected_status = Some(status);
        self
    }
    
    pub fn require_field(mut self, field: &str) -> Self {
        self.required_fields.push(field.to_string());
        self
    }
    
    pub fn validate_field<F>(mut self, field: &str, validator: F) -> Self
    where
        F: Fn(&serde_json::Value) -> bool + 'static,
    {
        self.field_validators.insert(field.to_string(), Box::new(validator));
        self
    }
    
    pub fn validate_response(&self, status: u16, body: &serde_json::Value) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Check status code
        if let Some(expected) = self.expected_status {
            if status != expected {
                errors.push(format!("Expected status {}, got {}", expected, status));
            }
        }
        
        // Check required fields
        for field in &self.required_fields {
            if body.get(field).is_none() {
                errors.push(format!("Missing required field: {}", field));
            }
        }
        
        // Run field validators
        for (field, validator) in &self.field_validators {
            if let Some(value) = body.get(field) {
                if !validator(value) {
                    errors.push(format!("Field validation failed: {}", field));
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validator for stream state transitions
pub struct StateTransitionValidator {
    allowed_transitions: HashMap<String, Vec<String>>,
}

impl StateTransitionValidator {
    pub fn new() -> Self {
        let mut transitions = HashMap::new();
        
        // Define allowed state transitions
        transitions.insert("Stopped".to_string(), vec!["Starting".to_string()]);
        transitions.insert("Starting".to_string(), vec!["Running".to_string(), "Error".to_string()]);
        transitions.insert("Running".to_string(), vec!["Stopping".to_string(), "Error".to_string()]);
        transitions.insert("Stopping".to_string(), vec!["Stopped".to_string()]);
        transitions.insert("Error".to_string(), vec!["Stopping".to_string(), "Starting".to_string()]);
        
        Self {
            allowed_transitions: transitions,
        }
    }
    
    pub fn is_valid_transition(&self, from: &str, to: &str) -> bool {
        if let Some(allowed) = self.allowed_transitions.get(from) {
            allowed.contains(&to.to_string())
        } else {
            false
        }
    }
    
    pub fn validate_sequence(&self, states: &[String]) -> Result<(), String> {
        if states.len() < 2 {
            return Ok(());
        }
        
        for window in states.windows(2) {
            let from = &window[0];
            let to = &window[1];
            
            if !self.is_valid_transition(from, to) {
                return Err(format!("Invalid transition: {} -> {}", from, to));
            }
        }
        
        Ok(())
    }
}

/// Performance benchmark validator
pub struct BenchmarkValidator {
    thresholds: PerformanceThresholds,
}

#[derive(Debug)]
pub struct PerformanceThresholds {
    pub max_startup_time_ms: f64,
    pub max_latency_ms: f64,
    pub min_throughput_mbps: f64,
    pub max_cpu_percent: f32,
    pub max_memory_mb: f32,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_startup_time_ms: 5000.0,
            max_latency_ms: 100.0,
            min_throughput_mbps: 10.0,
            max_cpu_percent: 80.0,
            max_memory_mb: 1024.0,
        }
    }
}

impl BenchmarkValidator {
    pub fn new() -> Self {
        Self {
            thresholds: PerformanceThresholds::default(),
        }
    }
    
    pub fn with_thresholds(mut self, thresholds: PerformanceThresholds) -> Self {
        self.thresholds = thresholds;
        self
    }
    
    pub fn validate_metrics(&self, metrics: &PerformanceMetrics) -> Vec<Issue> {
        let mut issues = Vec::new();
        
        if metrics.avg_stream_startup_time_ms > self.thresholds.max_startup_time_ms {
            issues.push(Issue {
                severity: IssueSeverity::High,
                category: IssueCategory::Performance,
                description: format!(
                    "Stream startup time {:.1}ms exceeds threshold {:.1}ms",
                    metrics.avg_stream_startup_time_ms,
                    self.thresholds.max_startup_time_ms
                ),
                test_name: None,
            });
        }
        
        if metrics.avg_latency_ms > self.thresholds.max_latency_ms {
            issues.push(Issue {
                severity: IssueSeverity::Medium,
                category: IssueCategory::Performance,
                description: format!(
                    "Average latency {:.1}ms exceeds threshold {:.1}ms",
                    metrics.avg_latency_ms,
                    self.thresholds.max_latency_ms
                ),
                test_name: None,
            });
        }
        
        if metrics.throughput_mbps < self.thresholds.min_throughput_mbps {
            issues.push(Issue {
                severity: IssueSeverity::Medium,
                category: IssueCategory::Performance,
                description: format!(
                    "Throughput {:.1}Mbps below threshold {:.1}Mbps",
                    metrics.throughput_mbps,
                    self.thresholds.min_throughput_mbps
                ),
                test_name: None,
            });
        }
        
        issues
    }
    
    pub fn validate_resources(&self, usage: &ResourceUsage) -> Vec<Issue> {
        let mut issues = Vec::new();
        
        if usage.peak_cpu_percent > self.thresholds.max_cpu_percent {
            issues.push(Issue {
                severity: IssueSeverity::High,
                category: IssueCategory::Resource,
                description: format!(
                    "Peak CPU usage {:.1}% exceeds threshold {:.1}%",
                    usage.peak_cpu_percent,
                    self.thresholds.max_cpu_percent
                ),
                test_name: None,
            });
        }
        
        if usage.peak_memory_mb > self.thresholds.max_memory_mb {
            issues.push(Issue {
                severity: IssueSeverity::High,
                category: IssueCategory::Resource,
                description: format!(
                    "Peak memory usage {:.1}MB exceeds threshold {:.1}MB",
                    usage.peak_memory_mb,
                    self.thresholds.max_memory_mb
                ),
                test_name: None,
            });
        }
        
        issues
    }
}

/// Generate a validation report from test results
pub fn generate_validation_report(
    test_results: Vec<TestResult>,
    metrics: PerformanceMetrics,
    usage: ResourceUsage,
    duration: Duration,
) -> ValidationReport {
    let tests_passed = test_results.iter().filter(|t| t.passed).count();
    let tests_failed = test_results.len() - tests_passed;
    
    // Run validators
    let benchmark_validator = BenchmarkValidator::new();
    let mut issues = benchmark_validator.validate_metrics(&metrics);
    issues.extend(benchmark_validator.validate_resources(&usage));
    
    // Add test failure issues
    for test in &test_results {
        if !test.passed {
            issues.push(Issue {
                severity: IssueSeverity::Critical,
                category: IssueCategory::Functionality,
                description: format!("Test '{}' failed: {:?}", test.name, test.error_message),
                test_name: Some(test.name.clone()),
            });
        }
    }
    
    ValidationReport {
        timestamp: chrono::Utc::now(),
        duration,
        tests_run: test_results.len(),
        tests_passed,
        tests_failed,
        test_results,
        performance_metrics: metrics,
        resource_usage: usage,
        issues_found: issues,
    }
}

/// Save validation report to file
pub fn save_report(report: &ValidationReport, path: PathBuf) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(report)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Print validation report summary
pub fn print_report_summary(report: &ValidationReport) {
    info!("=== Validation Report Summary ===");
    info!("Timestamp: {}", report.timestamp);
    info!("Duration: {:?}", report.duration);
    info!("Tests: {} run, {} passed, {} failed", 
        report.tests_run, report.tests_passed, report.tests_failed);
    
    if report.tests_failed > 0 {
        error!("Failed tests:");
        for test in &report.test_results {
            if !test.passed {
                error!("  - {}: {:?}", test.name, test.error_message);
            }
        }
    }
    
    info!("Performance Metrics:");
    info!("  Startup time: {:.1}ms", report.performance_metrics.avg_stream_startup_time_ms);
    info!("  Latency: {:.1}ms", report.performance_metrics.avg_latency_ms);
    info!("  Throughput: {:.1}Mbps", report.performance_metrics.throughput_mbps);
    
    info!("Resource Usage:");
    info!("  CPU: {:.1}% avg, {:.1}% peak", 
        report.resource_usage.avg_cpu_percent,
        report.resource_usage.peak_cpu_percent);
    info!("  Memory: {:.1}MB avg, {:.1}MB peak",
        report.resource_usage.avg_memory_mb,
        report.resource_usage.peak_memory_mb);
    
    if !report.issues_found.is_empty() {
        warn!("Issues Found:");
        for issue in &report.issues_found {
            let level = match issue.severity {
                IssueSeverity::Critical => "CRITICAL",
                IssueSeverity::High => "HIGH",
                IssueSeverity::Medium => "MEDIUM",
                IssueSeverity::Low => "LOW",
            };
            warn!("  [{}/{}] {}", level, 
                match issue.category {
                    IssueCategory::Performance => "PERF",
                    IssueCategory::Stability => "STAB",
                    IssueCategory::Resource => "RES",
                    IssueCategory::Functionality => "FUNC",
                    IssueCategory::Configuration => "CONFIG",
                },
                issue.description);
        }
    }
    
    info!("=================================");
}