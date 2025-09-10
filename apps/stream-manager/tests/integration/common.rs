use actix_web::test;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

use stream_manager::{
    api::AppState,
    manager::{StreamManager, stream_manager::config::StreamConfig},
    Config,
};

/// Test fixture for integration tests
pub struct TestFixture {
    pub config: Arc<Config>,
    pub stream_manager: Arc<StreamManager>,
    pub app_state: AppState,
    pub test_server: Option<test::TestServer>,
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
            test_server: None,
        }
    }
    
    pub async fn with_server(mut self) -> Self {
        use actix_web::{web, App};
        use stream_manager::api::routes;
        
        let server = test::start(move || {
            App::new()
                .app_data(web::Data::new(self.app_state.clone()))
                .configure(routes::configure_routes)
        });
        
        self.test_server = Some(server);
        self
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
pub fn create_test_stream_config(id: &str) -> StreamConfig {
    StreamConfig {
        id: id.to_string(),
        source_url: "videotestsrc ! video/x-raw,width=640,height=480".to_string(),
        source_type: stream_manager::config::SourceType::Test,
        recording: Some(stream_manager::config::RecordingConfig {
            enabled: true,
            base_path: std::path::PathBuf::from("/tmp/test-recordings"),
            segment_duration: Duration::from_secs(10),
            max_segments: Some(10),
            format: stream_manager::config::RecordingFormat::Mp4,
        }),
        inference: None,
        rtsp_outputs: vec![],
    }
}

/// Helper to wait for stream to be in expected state
pub async fn wait_for_stream_state(
    manager: &StreamManager,
    stream_id: &str,
    expected_state: stream_manager::manager::StreamState,
    timeout: Duration,
) -> bool {
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout {
        if let Some(info) = manager.get_stream_info(stream_id).await {
            if info.state == expected_state {
                return true;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
    
    false
}

/// Helper to simulate network conditions
pub struct NetworkSimulator {
    latency_ms: u32,
    packet_loss_percent: f32,
    bandwidth_limit_mbps: Option<f32>,
}

impl NetworkSimulator {
    pub fn new() -> Self {
        Self {
            latency_ms: 0,
            packet_loss_percent: 0.0,
            bandwidth_limit_mbps: None,
        }
    }
    
    pub fn with_latency(mut self, ms: u32) -> Self {
        self.latency_ms = ms;
        self
    }
    
    pub fn with_packet_loss(mut self, percent: f32) -> Self {
        self.packet_loss_percent = percent;
        self
    }
    
    pub fn with_bandwidth_limit(mut self, mbps: f32) -> Self {
        self.bandwidth_limit_mbps = Some(mbps);
        self
    }
    
    pub async fn apply(&self) {
        // In a real implementation, this would use tc (traffic control) on Linux
        // or similar tools to simulate network conditions
        info!(
            "Simulating network: latency={}ms, loss={}%, bandwidth={:?}Mbps",
            self.latency_ms, self.packet_loss_percent, self.bandwidth_limit_mbps
        );
    }
    
    pub async fn reset(&self) {
        info!("Resetting network conditions to normal");
    }
}

/// Helper to generate test video streams
pub struct TestStreamGenerator {
    pattern: String,
    width: u32,
    height: u32,
    fps: u32,
}

impl TestStreamGenerator {
    pub fn new() -> Self {
        Self {
            pattern: "smpte".to_string(),
            width: 1280,
            height: 720,
            fps: 30,
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

/// Helper to validate recording files
pub struct RecordingValidator {
    base_path: std::path::PathBuf,
}

impl RecordingValidator {
    pub fn new(base_path: std::path::PathBuf) -> Self {
        Self { base_path }
    }
    
    pub async fn validate_stream_recordings(&self, stream_id: &str) -> Result<ValidationResult, String> {
        let stream_path = self.base_path.join(stream_id);
        
        if !stream_path.exists() {
            return Err(format!("Recording directory not found: {:?}", stream_path));
        }
        
        let mut files = Vec::new();
        let mut total_size = 0u64;
        
        for entry in std::fs::read_dir(&stream_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            
            if metadata.is_file() {
                total_size += metadata.len();
                files.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        
        Ok(ValidationResult {
            stream_id: stream_id.to_string(),
            file_count: files.len(),
            total_size_bytes: total_size,
            files,
        })
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub stream_id: String,
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub files: Vec<String>,
}

/// Performance metrics collector
pub struct MetricsCollector {
    start_time: std::time::Instant,
    samples: Vec<MetricSample>,
}

#[derive(Debug, Clone)]
pub struct MetricSample {
    pub timestamp: Duration,
    pub cpu_percent: f32,
    pub memory_mb: f32,
    pub active_streams: usize,
    pub latency_ms: Option<f32>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            samples: Vec::new(),
        }
    }
    
    pub async fn collect_sample(&mut self, manager: &StreamManager) {
        let streams = manager.list_streams().await;
        let active_count = streams
            .iter()
            .filter(|s| matches!(s.state, stream_manager::manager::StreamState::Running))
            .count();
        
        // In a real implementation, we'd collect actual CPU and memory metrics
        let sample = MetricSample {
            timestamp: self.start_time.elapsed(),
            cpu_percent: 10.0 + (active_count as f32 * 5.0), // Mock data
            memory_mb: 100.0 + (active_count as f32 * 50.0), // Mock data
            active_streams: active_count,
            latency_ms: None,
        };
        
        self.samples.push(sample);
    }
    
    pub fn get_summary(&self) -> MetricsSummary {
        if self.samples.is_empty() {
            return MetricsSummary::default();
        }
        
        let avg_cpu = self.samples.iter().map(|s| s.cpu_percent).sum::<f32>() / self.samples.len() as f32;
        let max_cpu = self.samples.iter().map(|s| s.cpu_percent).fold(0.0, f32::max);
        let avg_memory = self.samples.iter().map(|s| s.memory_mb).sum::<f32>() / self.samples.len() as f32;
        let max_memory = self.samples.iter().map(|s| s.memory_mb).fold(0.0, f32::max);
        let max_streams = self.samples.iter().map(|s| s.active_streams).max().unwrap_or(0);
        
        MetricsSummary {
            duration: self.start_time.elapsed(),
            sample_count: self.samples.len(),
            avg_cpu_percent: avg_cpu,
            max_cpu_percent: max_cpu,
            avg_memory_mb: avg_memory,
            max_memory_mb: max_memory,
            max_concurrent_streams: max_streams,
        }
    }
}

#[derive(Debug, Default)]
pub struct MetricsSummary {
    pub duration: Duration,
    pub sample_count: usize,
    pub avg_cpu_percent: f32,
    pub max_cpu_percent: f32,
    pub avg_memory_mb: f32,
    pub max_memory_mb: f32,
    pub max_concurrent_streams: usize,
}
