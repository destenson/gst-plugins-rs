use prometheus::{
    Registry, Counter, CounterVec, Gauge, GaugeVec, HistogramVec,
    Encoder, TextEncoder, Opts, HistogramOpts,
};
use std::sync::Arc;

pub mod collector;
pub mod stream_metrics;

#[cfg(test)]
pub mod tests;

pub use collector::MetricsCollector;
pub use stream_metrics::StreamMetricsCollector;

pub struct MetricsRegistry {
    registry: Registry,
    
    // Stream metrics
    pub streams_total: Gauge,
    pub streams_active: Gauge,
    pub streams_failed: Gauge,
    pub stream_frames_processed: CounterVec,
    pub stream_bytes_processed: CounterVec,
    pub stream_dropped_frames: CounterVec,
    pub stream_reconnect_count: GaugeVec,
    pub stream_bitrate: GaugeVec,
    pub stream_fps: GaugeVec,
    pub stream_latency_ms: HistogramVec,
    pub stream_buffer_usage: GaugeVec,
    
    // Recording metrics
    pub recording_segments_total: CounterVec,
    pub recording_bytes_written: CounterVec,
    pub recording_duration_seconds: GaugeVec,
    pub recording_errors: CounterVec,
    
    
    // Application metrics
    pub app_uptime_seconds: Gauge,
    pub app_errors_total: CounterVec,
    pub app_requests_total: CounterVec,
    pub app_request_duration_seconds: HistogramVec,
    
    // GStreamer pipeline metrics
    pub pipeline_state_changes: CounterVec,
    pub pipeline_bus_messages: CounterVec,
    pub pipeline_element_count: GaugeVec,
    
    // Internal state
    app_start_time: std::time::Instant,
}

impl MetricsRegistry {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        // Stream metrics
        let streams_total = Gauge::with_opts(
            Opts::new("stream_manager_streams_total", "Total number of configured streams")
        )?;
        
        let streams_active = Gauge::with_opts(
            Opts::new("stream_manager_streams_active", "Number of currently active streams")
        )?;
        
        let streams_failed = Gauge::with_opts(
            Opts::new("stream_manager_streams_failed", "Number of failed streams")
        )?;
        
        let stream_frames_processed = CounterVec::new(
            Opts::new("stream_manager_frames_processed_total", "Total frames processed per stream"),
            &["stream_id", "source_type"]
        )?;
        
        let stream_bytes_processed = CounterVec::new(
            Opts::new("stream_manager_bytes_processed_total", "Total bytes processed per stream"),
            &["stream_id", "source_type"]
        )?;
        
        let stream_dropped_frames = CounterVec::new(
            Opts::new("stream_manager_dropped_frames_total", "Total dropped frames per stream"),
            &["stream_id", "reason"]
        )?;
        
        let stream_reconnect_count = GaugeVec::new(
            Opts::new("stream_manager_reconnect_count", "Number of reconnection attempts per stream"),
            &["stream_id"]
        )?;
        
        let stream_bitrate = GaugeVec::new(
            Opts::new("stream_manager_bitrate_bps", "Current bitrate in bits per second"),
            &["stream_id", "media_type"]
        )?;
        
        let stream_fps = GaugeVec::new(
            Opts::new("stream_manager_fps", "Current frames per second"),
            &["stream_id"]
        )?;
        
        let stream_latency_ms = HistogramVec::new(
            HistogramOpts::new("stream_manager_latency_milliseconds", "Stream latency distribution")
                .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0]),
            &["stream_id"]
        )?;
        
        let stream_buffer_usage = GaugeVec::new(
            Opts::new("stream_manager_buffer_usage_percent", "Buffer usage percentage"),
            &["stream_id", "buffer_type"]
        )?;
        
        // Recording metrics
        let recording_segments_total = CounterVec::new(
            Opts::new("stream_manager_recording_segments_total", "Total recording segments created"),
            &["stream_id", "format"]
        )?;
        
        let recording_bytes_written = CounterVec::new(
            Opts::new("stream_manager_recording_bytes_written_total", "Total bytes written to recordings"),
            &["stream_id", "storage_path"]
        )?;
        
        let recording_duration_seconds = GaugeVec::new(
            Opts::new("stream_manager_recording_duration_seconds", "Current recording duration"),
            &["stream_id"]
        )?;
        
        let recording_errors = CounterVec::new(
            Opts::new("stream_manager_recording_errors_total", "Total recording errors"),
            &["stream_id", "error_type"]
        )?;
        
        
        // Application metrics
        let app_uptime_seconds = Gauge::with_opts(
            Opts::new("stream_manager_uptime_seconds", "Application uptime in seconds")
        )?;
        
        let app_errors_total = CounterVec::new(
            Opts::new("stream_manager_errors_total", "Total application errors"),
            &["component", "severity"]
        )?;
        
        let app_requests_total = CounterVec::new(
            Opts::new("stream_manager_api_requests_total", "Total API requests"),
            &["method", "endpoint", "status"]
        )?;
        
        let app_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new("stream_manager_api_request_duration_seconds", "API request duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
            &["method", "endpoint"]
        )?;
        
        // GStreamer pipeline metrics
        let pipeline_state_changes = CounterVec::new(
            Opts::new("stream_manager_pipeline_state_changes_total", "Pipeline state changes"),
            &["stream_id", "from_state", "to_state"]
        )?;
        
        let pipeline_bus_messages = CounterVec::new(
            Opts::new("stream_manager_pipeline_bus_messages_total", "Pipeline bus messages"),
            &["stream_id", "message_type"]
        )?;
        
        let pipeline_element_count = GaugeVec::new(
            Opts::new("stream_manager_pipeline_elements", "Number of elements in pipeline"),
            &["stream_id"]
        )?;
        
        // Register all metrics
        registry.register(Box::new(streams_total.clone()))?;
        registry.register(Box::new(streams_active.clone()))?;
        registry.register(Box::new(streams_failed.clone()))?;
        registry.register(Box::new(stream_frames_processed.clone()))?;
        registry.register(Box::new(stream_bytes_processed.clone()))?;
        registry.register(Box::new(stream_dropped_frames.clone()))?;
        registry.register(Box::new(stream_reconnect_count.clone()))?;
        registry.register(Box::new(stream_bitrate.clone()))?;
        registry.register(Box::new(stream_fps.clone()))?;
        registry.register(Box::new(stream_latency_ms.clone()))?;
        registry.register(Box::new(stream_buffer_usage.clone()))?;
        
        registry.register(Box::new(recording_segments_total.clone()))?;
        registry.register(Box::new(recording_bytes_written.clone()))?;
        registry.register(Box::new(recording_duration_seconds.clone()))?;
        registry.register(Box::new(recording_errors.clone()))?;
        
        
        registry.register(Box::new(app_uptime_seconds.clone()))?;
        registry.register(Box::new(app_errors_total.clone()))?;
        registry.register(Box::new(app_requests_total.clone()))?;
        registry.register(Box::new(app_request_duration_seconds.clone()))?;
        
        registry.register(Box::new(pipeline_state_changes.clone()))?;
        registry.register(Box::new(pipeline_bus_messages.clone()))?;
        registry.register(Box::new(pipeline_element_count.clone()))?;
        
        
        Ok(Self {
            registry,
            streams_total,
            streams_active,
            streams_failed,
            stream_frames_processed,
            stream_bytes_processed,
            stream_dropped_frames,
            stream_reconnect_count,
            stream_bitrate,
            stream_fps,
            stream_latency_ms,
            stream_buffer_usage,
            recording_segments_total,
            recording_bytes_written,
            recording_duration_seconds,
            recording_errors,
            app_uptime_seconds,
            app_errors_total,
            app_requests_total,
            app_request_duration_seconds,
            pipeline_state_changes,
            pipeline_bus_messages,
            pipeline_element_count,
            app_start_time: std::time::Instant::now(),
        })
    }
    
    pub fn update_app_uptime(&self) {
        let uptime = self.app_start_time.elapsed().as_secs() as f64;
        self.app_uptime_seconds.set(uptime);
    }
    
    pub fn export_prometheus(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        String::from_utf8(buffer).map_err(|e| prometheus::Error::Msg(e.to_string()))
    }
    
}

