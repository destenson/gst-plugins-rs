use opentelemetry::{
    metrics::{Counter, Histogram},
    KeyValue,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

use super::TelemetryProvider;

pub struct PerformanceMetrics {
    provider: Arc<TelemetryProvider>,
    
    // Stream metrics
    stream_starts: Counter<u64>,
    stream_errors: Counter<u64>,
    
    // Pipeline metrics
    pipeline_startup_time: Histogram<f64>,
    pipeline_processing_latency: Histogram<f64>,
    frames_processed: Counter<u64>,
    frames_dropped: Counter<u64>,
    
    // Recording metrics
    bytes_written: Counter<u64>,
    recording_duration: Histogram<f64>,
    file_rotations: Counter<u64>,
    
    // API metrics
    api_requests: Counter<u64>,
    api_response_time: Histogram<f64>,
    api_errors: Counter<u64>,
    
    // Inference metrics
    inference_latency: Histogram<f64>,
    inference_batch_size: Histogram<u64>,
    detections_count: Counter<u64>,
    
    // Internal state for observable gauges
    current_metrics: Arc<RwLock<CurrentMetrics>>,
}

#[derive(Default)]
struct CurrentMetrics {
    active_streams: u64,
    memory_usage: u64,
    cpu_usage: f64,
    disk_usage: u64,
}

impl PerformanceMetrics {
    pub async fn new(provider: Arc<TelemetryProvider>) -> Self {
        let meter = provider.meter();
        let current_metrics = Arc::new(RwLock::new(CurrentMetrics::default()));
        
        // Stream metrics
        let stream_starts = meter
            .u64_counter("stream.starts")
            .with_description("Number of streams started")
            .build();
        let stream_errors = meter
            .u64_counter("stream.errors")
            .with_description("Number of stream errors")
            .build();
        
        // Pipeline metrics
        let pipeline_startup_time = meter
            .f64_histogram("pipeline.startup_time")
            .with_description("Pipeline startup time in seconds")
            .build();
        let pipeline_processing_latency = meter
            .f64_histogram("pipeline.processing_latency")
            .with_description("Pipeline processing latency in milliseconds")
            .build();
        let frames_processed = meter
            .u64_counter("frames.processed")
            .with_description("Total frames processed")
            .build();
        let frames_dropped = meter
            .u64_counter("frames.dropped")
            .with_description("Total frames dropped")
            .build();
        
        // Recording metrics
        let bytes_written = meter
            .u64_counter("recording.bytes_written")
            .with_description("Total bytes written to recordings")
            .build();
        let recording_duration = meter
            .f64_histogram("recording.duration")
            .with_description("Recording segment duration in seconds")
            .build();
        let file_rotations = meter
            .u64_counter("recording.file_rotations")
            .with_description("Number of file rotations")
            .build();
        
        // API metrics
        let api_requests = meter
            .u64_counter("api.requests")
            .with_description("Total API requests")
            .build();
        let api_response_time = meter
            .f64_histogram("api.response_time")
            .with_description("API response time in milliseconds")
            .build();
        let api_errors = meter
            .u64_counter("api.errors")
            .with_description("Total API errors")
            .build();
        
        // Inference metrics
        let inference_latency = meter
            .f64_histogram("inference.latency")
            .with_description("Inference latency in milliseconds")
            .build();
        let inference_batch_size = meter
            .u64_histogram("inference.batch_size")
            .with_description("Inference batch size")
            .build();
        let detections_count = meter
            .u64_counter("inference.detections")
            .with_description("Total detections")
            .build();
        
        // Set up observable gauges with callbacks
        let metrics_clone = current_metrics.clone();
        meter
            .u64_observable_gauge("stream.active")
            .with_description("Number of active streams")
            .with_callback(move |observer| {
                let metrics = metrics_clone.clone();
                let _ = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let current = metrics.read().await;
                        observer.observe(current.active_streams, &[]);
                    })
                });
            })
            .build();
        
        let metrics_clone = current_metrics.clone();
        meter
            .u64_observable_gauge("system.memory_usage")
            .with_description("Memory usage in bytes")
            .with_callback(move |observer| {
                let metrics = metrics_clone.clone();
                let _ = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let current = metrics.read().await;
                        observer.observe(current.memory_usage, &[]);
                    })
                });
            })
            .build();
        
        let metrics_clone = current_metrics.clone();
        meter
            .f64_observable_gauge("system.cpu_usage")
            .with_description("CPU usage percentage")
            .with_callback(move |observer| {
                let metrics = metrics_clone.clone();
                let _ = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let current = metrics.read().await;
                        observer.observe(current.cpu_usage, &[]);
                    })
                });
            })
            .build();
        
        let metrics_clone = current_metrics.clone();
        meter
            .u64_observable_gauge("system.disk_usage")
            .with_description("Disk usage in bytes")
            .with_callback(move |observer| {
                let metrics = metrics_clone.clone();
                let _ = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let current = metrics.read().await;
                        observer.observe(current.disk_usage, &[]);
                    })
                });
            })
            .build();
        
        Self {
            provider,
            stream_starts,
            stream_errors,
            pipeline_startup_time,
            pipeline_processing_latency,
            frames_processed,
            frames_dropped,
            bytes_written,
            recording_duration,
            file_rotations,
            api_requests,
            api_response_time,
            api_errors,
            inference_latency,
            inference_batch_size,
            detections_count,
            current_metrics,
        }
    }
    
    // Stream metrics
    pub fn record_stream_start(&self, stream_id: &str) {
        self.stream_starts.add(1, &[KeyValue::new("stream.id", stream_id.to_string())]);
    }
    
    pub fn record_stream_error(&self, stream_id: &str, error_type: &str) {
        self.stream_errors.add(
            1,
            &[
                KeyValue::new("stream.id", stream_id.to_string()),
                KeyValue::new("error.type", error_type.to_string()),
            ],
        );
    }
    
    pub async fn update_active_streams(&self, count: u64) {
        let mut metrics = self.current_metrics.write().await;
        metrics.active_streams = count;
    }
    
    // Pipeline metrics
    pub fn record_pipeline_startup(&self, duration: Duration, pipeline_id: &str) {
        self.pipeline_startup_time.record(
            duration.as_secs_f64(),
            &[KeyValue::new("pipeline.id", pipeline_id.to_string())],
        );
    }
    
    pub fn record_processing_latency(&self, latency_ms: f64, pipeline_id: &str) {
        self.pipeline_processing_latency.record(
            latency_ms,
            &[KeyValue::new("pipeline.id", pipeline_id.to_string())],
        );
    }
    
    pub fn record_frames_processed(&self, count: u64, stream_id: &str) {
        self.frames_processed.add(
            count,
            &[KeyValue::new("stream.id", stream_id.to_string())],
        );
    }
    
    pub fn record_frames_dropped(&self, count: u64, stream_id: &str) {
        self.frames_dropped.add(
            count,
            &[KeyValue::new("stream.id", stream_id.to_string())],
        );
    }
    
    // Recording metrics
    pub fn record_bytes_written(&self, bytes: u64, stream_id: &str) {
        self.bytes_written.add(
            bytes,
            &[KeyValue::new("stream.id", stream_id.to_string())],
        );
    }
    
    pub fn record_recording_duration(&self, duration: Duration, stream_id: &str) {
        self.recording_duration.record(
            duration.as_secs_f64(),
            &[KeyValue::new("stream.id", stream_id.to_string())],
        );
    }
    
    pub fn record_file_rotation(&self, stream_id: &str) {
        self.file_rotations.add(
            1,
            &[KeyValue::new("stream.id", stream_id.to_string())],
        );
    }
    
    // API metrics
    pub fn record_api_request(&self, method: &str, path: &str) {
        self.api_requests.add(
            1,
            &[
                KeyValue::new("http.method", method.to_string()),
                KeyValue::new("http.path", path.to_string()),
            ],
        );
    }
    
    pub fn record_api_response_time(&self, duration: Duration, method: &str, path: &str, status: u16) {
        self.api_response_time.record(
            duration.as_millis() as f64,
            &[
                KeyValue::new("http.method", method.to_string()),
                KeyValue::new("http.path", path.to_string()),
                KeyValue::new("http.status", status as i64),
            ],
        );
    }
    
    pub fn record_api_error(&self, method: &str, path: &str, error_type: &str) {
        self.api_errors.add(
            1,
            &[
                KeyValue::new("http.method", method.to_string()),
                KeyValue::new("http.path", path.to_string()),
                KeyValue::new("error.type", error_type.to_string()),
            ],
        );
    }
    
    // Inference metrics
    pub fn record_inference_latency(&self, latency_ms: f64, model: &str) {
        self.inference_latency.record(
            latency_ms,
            &[KeyValue::new("inference.model", model.to_string())],
        );
    }
    
    pub fn record_inference_batch(&self, batch_size: u64, model: &str) {
        self.inference_batch_size.record(
            batch_size,
            &[KeyValue::new("inference.model", model.to_string())],
        );
    }
    
    pub fn record_detections(&self, count: u64, model: &str, stream_id: &str) {
        self.detections_count.add(
            count,
            &[
                KeyValue::new("inference.model", model.to_string()),
                KeyValue::new("stream.id", stream_id.to_string()),
            ],
        );
    }
    
    // Resource metrics
    pub async fn update_resource_metrics(&self, memory: u64, cpu: f64, disk: u64) {
        let mut metrics = self.current_metrics.write().await;
        metrics.memory_usage = memory;
        metrics.cpu_usage = cpu;
        metrics.disk_usage = disk;
    }
}

pub struct OperationTimer {
    start: Instant,
    name: String,
}

impl OperationTimer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
        }
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    
    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed().as_millis() as f64
    }
}

impl Drop for OperationTimer {
    fn drop(&mut self) {
        debug!(
            "Operation '{}' completed in {:?}",
            self.name,
            self.elapsed()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::TelemetryConfig;
    
    #[tokio::test]
    async fn test_performance_metrics_creation() {
        let config = TelemetryConfig {
            console_exporter: true,
            otlp_endpoint: None,
            ..Default::default()
        };
        
        let provider = TelemetryProvider::new(config).await.unwrap();
        let metrics = PerformanceMetrics::new(provider).await;
        
        // Test recording some metrics
        metrics.record_stream_start("test-stream");
        metrics.record_frames_processed(100, "test-stream");
        metrics.update_active_streams(1).await;
    }
    
    #[tokio::test]
    async fn test_operation_timer() {
        let timer = OperationTimer::new("test_operation");
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        assert!(timer.elapsed_ms() >= 10.0);
    }
    
    #[tokio::test]
    async fn test_resource_metrics_update() {
        let config = TelemetryConfig {
            console_exporter: true,
            otlp_endpoint: None,
            ..Default::default()
        };
        
        let provider = TelemetryProvider::new(config).await.unwrap();
        let metrics = PerformanceMetrics::new(provider).await;
        
        metrics.update_resource_metrics(1024 * 1024, 25.5, 1024 * 1024 * 1024).await;
        
        let current = metrics.current_metrics.read().await;
        assert_eq!(current.memory_usage, 1024 * 1024);
        assert_eq!(current.cpu_usage, 25.5);
        assert_eq!(current.disk_usage, 1024 * 1024 * 1024);
    }
}