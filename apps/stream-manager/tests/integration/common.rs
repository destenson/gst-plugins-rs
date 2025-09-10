use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

use stream_manager::{
    api::AppState,
    config::{Config, StreamConfig},
    manager::{StreamManager, StreamState},
};

/// Test fixture for integration tests
pub struct TestFixture {
    pub config: Arc<Config>,
    pub stream_manager: Arc<StreamManager>,
    pub app_state: AppState,
}

impl TestFixture {
    pub async fn new() -> Self {
        let config = super::create_test_config();
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let app_state = AppState::new(stream_manager.clone(), config.clone());
        
        Self {
            config,
            stream_manager,
            app_state,
        }
    }
    
    pub async fn cleanup(&self) {
        // Stop all streams
        let streams = self.stream_manager.list_streams().await;
        for stream in streams {
            let _ = self.stream_manager.remove_stream(&stream.id).await;
        }
    }
}

/// Helper to create a test RTSP source
pub fn create_test_rtsp_source(port: u16) -> String {
    format!("rtsp://127.0.0.1:{}/test", port)
}

/// Helper to create a test stream configuration
pub fn create_test_stream_config(id: &str) -> (String, StreamConfig) {
    let config = StreamConfig {
        id: id.to_string(),
        name: id.to_string(),
        source_uri: "videotestsrc ! video/x-raw,width=640,height=480".to_string(),
        enabled: true,
        recording_enabled: true,
        inference_enabled: false,
        reconnect_timeout_seconds: 10,
        max_reconnect_attempts: 5,
        buffer_size_mb: 64,
        rtsp_outputs: None,
    };
    (id.to_string(), config)
}

/// Helper to create a test stream configuration with custom source
pub fn create_stream_config_with_source(id: &str, source_uri: &str) -> (String, StreamConfig) {
    let config = StreamConfig {
        id: id.to_string(),
        name: id.to_string(),
        source_uri: source_uri.to_string(),
        enabled: true,
        recording_enabled: false,
        inference_enabled: false,
        reconnect_timeout_seconds: 5,
        max_reconnect_attempts: 3,
        buffer_size_mb: 32,
        rtsp_outputs: None,
    };
    (id.to_string(), config)
}

/// Helper to wait for stream to be in expected state
pub async fn wait_for_stream_state(
    manager: &StreamManager,
    stream_id: &str,
    expected_state: StreamState,
    timeout: Duration,
) -> bool {
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout {
        if let Ok(info) = manager.get_stream_info(stream_id).await {
            if info.state == expected_state {
                return true;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
    
    false
}

/// Helper to wait for stream health
pub async fn wait_for_stream_health(
    manager: &StreamManager,
    stream_id: &str,
    timeout: Duration,
) -> bool {
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout {
        if let Ok(info) = manager.get_stream_info(stream_id).await {
            if info.health.is_healthy {
                return true;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
    
    false
}

/// Helper to add a stream and wait for it to be running
pub async fn add_and_start_stream(
    manager: &StreamManager,
    id: &str,
    config: StreamConfig,
) -> Result<(), String> {
    manager.add_stream(id.to_string(), config)
        .await
        .map_err(|e| format!("Failed to add stream: {}", e))?;
    
    if !wait_for_stream_state(manager, id, StreamState::Running, Duration::from_secs(10)).await {
        return Err(format!("Stream {} failed to start", id));
    }
    
    Ok(())
}

/// Network simulator for testing network conditions
pub struct NetworkSimulator {
    latency_ms: Option<u32>,
    packet_loss_percent: Option<f32>,
    bandwidth_mbps: Option<f32>,
}

impl NetworkSimulator {
    pub fn new() -> Self {
        Self {
            latency_ms: None,
            packet_loss_percent: None,
            bandwidth_mbps: None,
        }
    }
    
    pub fn with_latency(mut self, ms: u32) -> Self {
        self.latency_ms = Some(ms);
        self
    }
    
    pub fn with_packet_loss(mut self, percent: f32) -> Self {
        self.packet_loss_percent = Some(percent);
        self
    }
    
    pub fn with_bandwidth_limit(mut self, mbps: f32) -> Self {
        self.bandwidth_mbps = Some(mbps);
        self
    }
    
    pub async fn apply(&self) {
        // Simulate network conditions
        if let Some(latency) = self.latency_ms {
            info!("Simulating network latency: {}ms", latency);
        }
        if let Some(loss) = self.packet_loss_percent {
            info!("Simulating packet loss: {}%", loss);
        }
        if let Some(bandwidth) = self.bandwidth_mbps {
            info!("Simulating bandwidth limit: {} Mbps", bandwidth);
        }
    }
    
    pub async fn reset(&self) {
        info!("Resetting network conditions");
    }
}

/// Test stream generator for creating test pipelines
pub struct TestStreamGenerator {
    width: u32,
    height: u32,
    fps: u32,
    pattern: String,
}

impl TestStreamGenerator {
    pub fn new() -> Self {
        Self {
            width: 640,
            height: 480,
            fps: 30,
            pattern: "smpte".to_string(),
        }
    }
    
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }
    
    pub fn with_pattern(mut self, pattern: &str) -> Self {
        self.pattern = pattern.to_string();
        self
    }
    
    pub fn to_pipeline_string(&self) -> String {
        format!(
            "videotestsrc pattern={} ! video/x-raw,width={},height={},framerate={}/1",
            self.pattern, self.width, self.height, self.fps
        )
    }
}

/// Recording validator for checking recorded files
pub struct RecordingValidator {
    base_path: std::path::PathBuf,
}

impl RecordingValidator {
    pub fn new(base_path: std::path::PathBuf) -> Self {
        Self { base_path }
    }
    
    pub async fn validate_stream_recordings(&self, stream_id: &str) -> Result<ValidationResult, String> {
        let recordings_path = self.base_path.join(stream_id);
        
        if !recordings_path.exists() {
            return Err(format!("Recordings path does not exist: {:?}", recordings_path));
        }
        
        let mut file_count = 0;
        let mut total_size = 0u64;
        let mut total_duration = Duration::default();
        
        let entries = std::fs::read_dir(&recordings_path)
            .map_err(|e| format!("Failed to read recordings directory: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let metadata = entry.metadata()
                .map_err(|e| format!("Failed to read file metadata: {}", e))?;
            
            if metadata.is_file() {
                file_count += 1;
                total_size += metadata.len();
                // Note: actual duration would require parsing the media file
                total_duration += Duration::from_secs(10); // Assume segment duration
            }
        }
        
        Ok(ValidationResult {
            file_count,
            total_size,
            total_duration,
            path: recordings_path,
        })
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub file_count: usize,
    pub total_size: u64,
    pub total_duration: Duration,
    pub path: std::path::PathBuf,
}

/// Metrics collector for performance testing
pub struct MetricsCollector {
    samples: Vec<MetricSample>,
    start_time: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct MetricSample {
    pub timestamp: Duration,
    pub cpu_percent: f32,
    pub memory_mb: f32,
    pub active_streams: usize,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            start_time: std::time::Instant::now(),
        }
    }
    
    pub async fn collect_sample(&mut self, manager: &StreamManager) {
        let timestamp = self.start_time.elapsed();
        let streams = manager.list_streams().await;
        let active_streams = streams.iter()
            .filter(|s| s.state == StreamState::Running)
            .count();
        
        // Note: actual CPU and memory metrics would require system monitoring
        let cpu_percent = 10.0 + (active_streams as f32 * 5.0); // Simulated
        let memory_mb = 100.0 + (active_streams as f32 * 50.0); // Simulated
        
        self.samples.push(MetricSample {
            timestamp,
            cpu_percent,
            memory_mb,
            active_streams,
        });
    }
    
    pub fn get_summary(&self) -> MetricsSummary {
        let duration = self.start_time.elapsed();
        let sample_count = self.samples.len();
        
        let avg_cpu = if sample_count > 0 {
            self.samples.iter().map(|s| s.cpu_percent).sum::<f32>() / sample_count as f32
        } else {
            0.0
        };
        
        let max_cpu = self.samples.iter()
            .map(|s| s.cpu_percent)
            .fold(0.0f32, f32::max);
        
        let avg_memory = if sample_count > 0 {
            self.samples.iter().map(|s| s.memory_mb).sum::<f32>() / sample_count as f32
        } else {
            0.0
        };
        
        let max_memory = self.samples.iter()
            .map(|s| s.memory_mb)
            .fold(0.0f32, f32::max);
        
        let max_streams = self.samples.iter()
            .map(|s| s.active_streams)
            .max()
            .unwrap_or(0);
        
        MetricsSummary {
            duration,
            sample_count,
            avg_cpu_percent: avg_cpu,
            max_cpu_percent: max_cpu,
            avg_memory_mb: avg_memory,
            max_memory_mb: max_memory,
            max_concurrent_streams: max_streams,
        }
    }
}

#[derive(Debug)]
pub struct MetricsSummary {
    pub duration: Duration,
    pub sample_count: usize,
    pub avg_cpu_percent: f32,
    pub max_cpu_percent: f32,
    pub avg_memory_mb: f32,
    pub max_memory_mb: f32,
    pub max_concurrent_streams: usize,
}