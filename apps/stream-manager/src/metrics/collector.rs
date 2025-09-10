#![allow(unused)]
use super::MetricsRegistry;
use crate::manager::{StreamManager, StreamEvent, StreamStatistics};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};

pub struct MetricsCollector {
    registry: Arc<MetricsRegistry>,
    stream_manager: Arc<StreamManager>,
    update_interval: Duration,
    system_update_interval: Duration,
}

impl MetricsCollector {
    pub fn new(
        stream_manager: Arc<StreamManager>,
        config: Option<&crate::config::MetricsConfig>,
    ) -> Result<Self, prometheus::Error> {
        let registry = Arc::new(MetricsRegistry::new()?);
        
        let (update_interval, system_update_interval) = if let Some(config) = config {
            (
                Duration::from_secs(config.collection_interval_seconds),
                Duration::from_secs(config.system_metrics_interval_seconds),
            )
        } else {
            (Duration::from_secs(5), Duration::from_secs(10))
        };
        
        Ok(Self {
            registry,
            stream_manager,
            update_interval,
            system_update_interval,
        })
    }
    
    pub fn with_intervals(
        stream_manager: Arc<StreamManager>,
        update_interval: Duration,
        system_update_interval: Duration,
    ) -> Result<Self, prometheus::Error> {
        let registry = Arc::new(MetricsRegistry::new()?);
        
        Ok(Self {
            registry,
            stream_manager,
            update_interval,
            system_update_interval,
        })
    }
    
    pub fn registry(&self) -> Arc<MetricsRegistry> {
        self.registry.clone()
    }
    
    pub async fn start_collection(&self) {
        let registry = self.registry.clone();
        let stream_manager = self.stream_manager.clone();
        let update_interval = self.update_interval;
        
        // Start stream metrics collection task
        tokio::spawn(async move {
            let mut interval = interval(update_interval);
            
            loop {
                interval.tick().await;
                
                // Update app uptime
                registry.update_app_uptime();
                
                // Get stream statistics from the stream manager
                match stream_manager.get_all_stream_statistics().await {
                    Ok(stats) => {
                        update_stream_metrics(&registry, stats).await;
                    }
                    Err(e) => {
                        error!("Failed to get stream statistics: {}", e);
                        registry.app_errors_total
                            .with_label_values(&["metrics_collector", "error"])
                            .inc();
                    }
                }
                
                // Update stream counts
                let streams = stream_manager.list_streams().await;
                let total = streams.len() as f64;
                let active = streams.iter()
                    .filter(|s| matches!(s.state, crate::manager::StreamState::Running))
                    .count() as f64;
                let failed = streams.iter()
                    .filter(|s| matches!(s.state, crate::manager::StreamState::Error(_)))
                    .count() as f64;
                
                registry.streams_total.set(total);
                registry.streams_active.set(active);
                registry.streams_failed.set(failed);
            }
        });
        
        
        info!("Metrics collection started");
    }
    
    pub async fn export_prometheus(&self) -> Result<String, prometheus::Error> {
        self.registry.export_prometheus()
    }
    
    pub fn record_api_request(
        &self,
        method: &str,
        endpoint: &str,
        status: u16,
        duration: Duration,
    ) {
        self.registry.app_requests_total
            .with_label_values(&[method, endpoint, &status.to_string()])
            .inc();
        
        self.registry.app_request_duration_seconds
            .with_label_values(&[method, endpoint])
            .observe(duration.as_secs_f64());
    }
    
    pub fn record_stream_event(&self, event: &StreamEvent) {
        match event {
            StreamEvent::StreamError(stream_id, error) => {
                let label = format!("stream_{}", stream_id);
                self.registry.app_errors_total
                    .with_label_values(&[label.as_str(), "error"])
                    .inc();
            }
            StreamEvent::StreamReconnecting(stream_id) => {
                self.registry.stream_reconnect_count
                    .with_label_values(&[stream_id])
                    .inc();
            }
            StreamEvent::StatisticsUpdate(stream_id, stats) => {
                // Update individual stream metrics
                self.update_stream_statistics(stream_id, stats);
            }
            _ => {}
        }
    }
    
    fn update_stream_statistics(&self, stream_id: &str, stats: &StreamStatistics) {
        // Note: This is using the manager's StreamStatistics struct
        // We'll map the available fields to our metrics
        
        if stats.packets_received > 0 {
            self.registry.stream_frames_processed
                .with_label_values(&[stream_id, "rtsp"])
                .inc_by(stats.packets_received as f64);
        }
        
        if stats.bytes_received > 0 {
            self.registry.stream_bytes_processed
                .with_label_values(&[stream_id, "rtsp"])
                .inc_by(stats.bytes_received as f64);
        }
        
        if stats.dropped_frames > 0 {
            self.registry.stream_dropped_frames
                .with_label_values(&[stream_id, "buffer_overflow"])
                .inc_by(stats.dropped_frames as f64);
        }
        
        self.registry.stream_reconnect_count
            .with_label_values(&[stream_id])
            .set(stats.reconnect_count as f64);
    }
    
    pub fn record_pipeline_state_change(
        &self,
        stream_id: &str,
        from_state: &str,
        to_state: &str,
    ) {
        self.registry.pipeline_state_changes
            .with_label_values(&[stream_id, from_state, to_state])
            .inc();
    }
    
    pub fn record_pipeline_message(&self, stream_id: &str, message_type: &str) {
        self.registry.pipeline_bus_messages
            .with_label_values(&[stream_id, message_type])
            .inc();
    }
    
    pub fn update_pipeline_element_count(&self, stream_id: &str, count: usize) {
        self.registry.pipeline_element_count
            .with_label_values(&[stream_id])
            .set(count as f64);
    }
    
    pub fn record_recording_segment(&self, stream_id: &str, format: &str) {
        self.registry.recording_segments_total
            .with_label_values(&[stream_id, format])
            .inc();
    }
    
    pub fn record_recording_bytes(&self, stream_id: &str, storage_path: &str, bytes: u64) {
        self.registry.recording_bytes_written
            .with_label_values(&[stream_id, storage_path])
            .inc_by(bytes as f64);
    }
    
    pub fn update_recording_duration(&self, stream_id: &str, duration_secs: f64) {
        self.registry.recording_duration_seconds
            .with_label_values(&[stream_id])
            .set(duration_secs);
    }
    
    pub fn record_recording_error(&self, stream_id: &str, error_type: &str) {
        self.registry.recording_errors
            .with_label_values(&[stream_id, error_type])
            .inc();
    }
}

async fn update_stream_metrics(
    registry: &Arc<MetricsRegistry>,
    stats: Vec<(String, crate::api::dto::StreamStatistics)>,
) {
    for (stream_id, stream_stats) in stats {
        // Update bitrate
        registry.stream_bitrate
            .with_label_values(&[stream_id.as_str(), "combined"])
            .set(stream_stats.bitrate);
        
        // Update FPS
        registry.stream_fps
            .with_label_values(&[stream_id.as_str()])
            .set(stream_stats.fps);
        
        // Update latency
        registry.stream_latency_ms
            .with_label_values(&[stream_id.as_str()])
            .observe(stream_stats.latency_ms);
        
        // Update frame counts
        if stream_stats.frames_processed > 0 {
            registry.stream_frames_processed
                .with_label_values(&[stream_id.as_str(), "rtsp"])
                .inc_by(stream_stats.frames_processed as f64);
        }
        
        if stream_stats.bytes_processed > 0 {
            registry.stream_bytes_processed
                .with_label_values(&[stream_id.as_str(), "rtsp"])
                .inc_by(stream_stats.bytes_processed as f64);
        }
        
        if stream_stats.dropped_frames > 0 {
            registry.stream_dropped_frames
                .with_label_values(&[stream_id.as_str(), "unknown"])
                .inc_by(stream_stats.dropped_frames as f64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    
    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let collector = MetricsCollector::new(stream_manager, Some(&config.monitoring.metrics)).unwrap();
        
        assert!(collector.export_prometheus().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_record_api_request() {
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let collector = MetricsCollector::new(stream_manager, Some(&config.monitoring.metrics)).unwrap();
        
        collector.record_api_request("GET", "/api/v1/health", 200, Duration::from_millis(50));
        
        let metrics = collector.export_prometheus().await.unwrap();
        assert!(metrics.contains("stream_manager_api_requests_total"));
        assert!(metrics.contains("stream_manager_api_request_duration_seconds"));
    }
    
    #[test]
    fn test_record_pipeline_state_change() {
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let collector = MetricsCollector::new(stream_manager, Some(&config.monitoring.metrics)).unwrap();
        
        collector.record_pipeline_state_change("stream1", "NULL", "READY");
        collector.record_pipeline_state_change("stream1", "READY", "PLAYING");
        
        let metrics = collector.registry.export_prometheus().unwrap();
        assert!(metrics.contains("stream_manager_pipeline_state_changes_total"));
    }
}
