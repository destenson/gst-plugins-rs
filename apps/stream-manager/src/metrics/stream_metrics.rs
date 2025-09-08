use super::MetricsRegistry;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, error};

pub struct StreamMetricsCollector {
    registry: Arc<MetricsRegistry>,
    stream_stats: Arc<std::sync::RwLock<HashMap<String, StreamMetrics>>>,
}

#[derive(Debug, Clone)]
pub struct StreamMetrics {
    pub stream_id: String,
    pub source_type: String,
    pub frames_processed: u64,
    pub bytes_processed: u64,
    pub dropped_frames: u64,
    pub reconnect_count: u32,
    pub current_bitrate: f64,
    pub current_fps: f64,
    pub average_latency_ms: f64,
    pub buffer_usage_percent: f64,
    pub last_update: std::time::Instant,
    pub start_time: std::time::Instant,
}

impl Default for StreamMetrics {
    fn default() -> Self {
        let now = std::time::Instant::now();
        Self {
            stream_id: String::new(),
            source_type: "unknown".to_string(),
            frames_processed: 0,
            bytes_processed: 0,
            dropped_frames: 0,
            reconnect_count: 0,
            current_bitrate: 0.0,
            current_fps: 0.0,
            average_latency_ms: 0.0,
            buffer_usage_percent: 0.0,
            last_update: now,
            start_time: now,
        }
    }
}

impl StreamMetricsCollector {
    pub fn new(registry: Arc<MetricsRegistry>) -> Self {
        Self {
            registry,
            stream_stats: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }
    
    pub fn update_stream_metrics(&self, stream_id: &str, metrics: StreamMetrics) {
        let mut stats = self.stream_stats.write().unwrap();
        
        // Get previous metrics if they exist
        let prev = stats.get(stream_id);
        
        // Calculate deltas for counter metrics
        if let Some(prev_metrics) = prev {
            let frames_delta = metrics.frames_processed.saturating_sub(prev_metrics.frames_processed);
            let bytes_delta = metrics.bytes_processed.saturating_sub(prev_metrics.bytes_processed);
            let dropped_delta = metrics.dropped_frames.saturating_sub(prev_metrics.dropped_frames);
            
            // Update counter metrics (they should only increase)
            if frames_delta > 0 {
                self.registry.stream_frames_processed
                    .with_label_values(&[stream_id, &metrics.source_type])
                    .inc_by(frames_delta as f64);
            }
            
            if bytes_delta > 0 {
                self.registry.stream_bytes_processed
                    .with_label_values(&[stream_id, &metrics.source_type])
                    .inc_by(bytes_delta as f64);
            }
            
            if dropped_delta > 0 {
                self.registry.stream_dropped_frames
                    .with_label_values(&[stream_id, "unknown"])
                    .inc_by(dropped_delta as f64);
            }
        } else {
            // First time seeing this stream, set initial values
            if metrics.frames_processed > 0 {
                self.registry.stream_frames_processed
                    .with_label_values(&[stream_id, &metrics.source_type])
                    .inc_by(metrics.frames_processed as f64);
            }
            
            if metrics.bytes_processed > 0 {
                self.registry.stream_bytes_processed
                    .with_label_values(&[stream_id, &metrics.source_type])
                    .inc_by(metrics.bytes_processed as f64);
            }
            
            if metrics.dropped_frames > 0 {
                self.registry.stream_dropped_frames
                    .with_label_values(&[stream_id, "unknown"])
                    .inc_by(metrics.dropped_frames as f64);
            }
        }
        
        // Update gauge metrics
        self.registry.stream_reconnect_count
            .with_label_values(&[stream_id])
            .set(metrics.reconnect_count as f64);
        
        self.registry.stream_bitrate
            .with_label_values(&[stream_id, "combined"])
            .set(metrics.current_bitrate);
        
        self.registry.stream_fps
            .with_label_values(&[stream_id])
            .set(metrics.current_fps);
        
        // Update histogram metrics
        if metrics.average_latency_ms > 0.0 {
            self.registry.stream_latency_ms
                .with_label_values(&[stream_id])
                .observe(metrics.average_latency_ms);
        }
        
        // Update buffer usage
        self.registry.stream_buffer_usage
            .with_label_values(&[stream_id, "default"])
            .set(metrics.buffer_usage_percent);
        
        // Store the updated metrics
        stats.insert(stream_id.to_string(), metrics);
        
        debug!("Updated metrics for stream: {}", stream_id);
    }
    
    pub fn remove_stream(&self, stream_id: &str) {
        let mut stats = self.stream_stats.write().unwrap();
        stats.remove(stream_id);
        debug!("Removed metrics for stream: {}", stream_id);
    }
    
    pub fn get_stream_metrics(&self, stream_id: &str) -> Option<StreamMetrics> {
        let stats = self.stream_stats.read().unwrap();
        stats.get(stream_id).cloned()
    }
    
    pub fn get_all_stream_metrics(&self) -> Vec<(String, StreamMetrics)> {
        let stats = self.stream_stats.read().unwrap();
        stats.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
    
    pub fn calculate_aggregate_metrics(&self) -> AggregateMetrics {
        let stats = self.stream_stats.read().unwrap();
        
        let mut total_frames = 0u64;
        let mut total_bytes = 0u64;
        let mut total_dropped = 0u64;
        let mut total_bitrate = 0.0;
        let mut active_streams = 0usize;
        
        for (_, metrics) in stats.iter() {
            total_frames += metrics.frames_processed;
            total_bytes += metrics.bytes_processed;
            total_dropped += metrics.dropped_frames;
            
            if metrics.current_fps > 0.0 {
                active_streams += 1;
                total_bitrate += metrics.current_bitrate;
            }
        }
        
        AggregateMetrics {
            total_streams: stats.len(),
            active_streams,
            total_frames_processed: total_frames,
            total_bytes_processed: total_bytes,
            total_dropped_frames: total_dropped,
            average_bitrate: if active_streams > 0 {
                total_bitrate / active_streams as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct AggregateMetrics {
    pub total_streams: usize,
    pub active_streams: usize,
    pub total_frames_processed: u64,
    pub total_bytes_processed: u64,
    pub total_dropped_frames: u64,
    pub average_bitrate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stream_metrics_update() {
        let registry = Arc::new(MetricsRegistry::new().unwrap());
        let collector = StreamMetricsCollector::new(registry.clone());
        
        let mut metrics = StreamMetrics::default();
        metrics.stream_id = "test_stream".to_string();
        metrics.source_type = "rtsp".to_string();
        metrics.frames_processed = 100;
        metrics.bytes_processed = 1024 * 1024;
        metrics.current_fps = 30.0;
        metrics.current_bitrate = 2_000_000.0;
        
        collector.update_stream_metrics("test_stream", metrics);
        
        let retrieved = collector.get_stream_metrics("test_stream");
        assert!(retrieved.is_some());
        
        let metrics = retrieved.unwrap();
        assert_eq!(metrics.frames_processed, 100);
        assert_eq!(metrics.current_fps, 30.0);
    }
    
    #[test]
    fn test_aggregate_metrics() {
        let registry = Arc::new(MetricsRegistry::new().unwrap());
        let collector = StreamMetricsCollector::new(registry);
        
        // Add stream 1
        let mut metrics1 = StreamMetrics::default();
        metrics1.stream_id = "stream1".to_string();
        metrics1.frames_processed = 100;
        metrics1.bytes_processed = 1000;
        metrics1.current_fps = 30.0;
        metrics1.current_bitrate = 1_000_000.0;
        collector.update_stream_metrics("stream1", metrics1);
        
        // Add stream 2
        let mut metrics2 = StreamMetrics::default();
        metrics2.stream_id = "stream2".to_string();
        metrics2.frames_processed = 200;
        metrics2.bytes_processed = 2000;
        metrics2.current_fps = 25.0;
        metrics2.current_bitrate = 2_000_000.0;
        collector.update_stream_metrics("stream2", metrics2);
        
        let aggregate = collector.calculate_aggregate_metrics();
        assert_eq!(aggregate.total_streams, 2);
        assert_eq!(aggregate.active_streams, 2);
        assert_eq!(aggregate.total_frames_processed, 300);
        assert_eq!(aggregate.total_bytes_processed, 3000);
        assert_eq!(aggregate.average_bitrate, 1_500_000.0);
    }
}