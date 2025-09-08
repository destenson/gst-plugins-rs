use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddStreamRequest {
    pub id: String,
    pub uri: String,
    pub stream_type: StreamType,
    pub recording_enabled: Option<bool>,
    pub inference_enabled: Option<bool>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamType {
    Rtsp,
    File,
    Http,
    Udp,
    Test,
}

impl From<AddStreamRequest> for crate::config::StreamConfig {
    fn from(req: AddStreamRequest) -> Self {
        crate::config::StreamConfig {
            id: req.id.clone(),
            name: req.id,
            source_uri: req.uri,
            enabled: true,
            recording_enabled: req.recording_enabled.unwrap_or(true),
            inference_enabled: req.inference_enabled.unwrap_or(false),
            reconnect_timeout_seconds: 5,
            max_reconnect_attempts: 10,
            buffer_size_mb: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponse {
    pub id: String,
    pub uri: String,
    pub stream_type: StreamType,
    pub status: StreamStatus,
    pub recording_enabled: bool,
    pub inference_enabled: bool,
    pub statistics: Option<StreamStatistics>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(String),
    Reconnecting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStatistics {
    pub frames_processed: u64,
    pub bytes_processed: u64,
    pub dropped_frames: u64,
    pub bitrate: f64,
    pub fps: f64,
    pub latency_ms: f64,
    pub uptime_seconds: u64,
    pub last_frame_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub version: String,
    pub uptime_seconds: u64,
    pub active_streams: usize,
    pub system_metrics: Option<SystemMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: u64,
    pub disk_usage_gb: f64,
    pub network_throughput_mbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub status_code: u16,
    pub timestamp: String,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub streams: Vec<StreamMetrics>,
    pub system: SystemMetrics,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetrics {
    pub stream_id: String,
    pub status: StreamStatus,
    pub statistics: StreamStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateRequest {
    pub section: String,
    pub values: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationRequest {
    pub operation: BatchOperation,
    pub stream_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchOperation {
    Start,
    Stop,
    Remove,
    EnableRecording,
    DisableRecording,
    EnableInference,
    DisableInference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResponse {
    pub successful: Vec<String>,
    pub failed: Vec<OperationFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationFailure {
    pub stream_id: String,
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_stream_request_serialization() {
        let request = AddStreamRequest {
            id: "test-stream".to_string(),
            uri: "rtsp://example.com/stream".to_string(),
            stream_type: StreamType::Rtsp,
            recording_enabled: Some(true),
            inference_enabled: Some(false),
            metadata: None,
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: AddStreamRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request.id, deserialized.id);
        assert_eq!(request.uri, deserialized.uri);
    }
    
    #[test]
    fn test_stream_config_conversion() {
        let request = AddStreamRequest {
            id: "test-stream".to_string(),
            uri: "rtsp://example.com/stream".to_string(),
            stream_type: StreamType::Rtsp,
            recording_enabled: Some(true),
            inference_enabled: Some(false),
            metadata: Some(HashMap::new()),
        };
        
        let config: crate::config::StreamConfig = request.into();
        assert_eq!(config.id, "test-stream");
        assert_eq!(config.source_uri, "rtsp://example.com/stream");
        assert!(config.recording_enabled);
        assert!(!config.inference_enabled);
    }
}