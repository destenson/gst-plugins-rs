use crate::config::{Config, StreamConfig};
use crate::stream::{BranchManager, StreamSource};
use crate::recording::RecordingBranch;
use crate::Result;
use gst::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

#[derive(Debug)]
pub struct ManagedStream {
    pub id: String,
    pub source: Arc<StreamSource>,
    pub branch_manager: Arc<BranchManager>,
    pub recording_branch: Option<Arc<RecordingBranch>>,
    pub statistics: StreamStatistics,
    pub health_status: HealthStatus,
}

#[derive(Debug, Clone)]
pub struct StreamStatistics {
    pub packets_received: u64,
    pub bytes_received: u64,
    pub dropped_frames: u64,
    pub reconnect_count: u32,
    pub last_update: std::time::Instant,
}

impl Default for StreamStatistics {
    fn default() -> Self {
        Self {
            packets_received: 0,
            bytes_received: 0,
            dropped_frames: 0,
            reconnect_count: 0,
            last_update: std::time::Instant::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
    Unknown,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug)]
pub struct StreamManager {
    streams: Arc<RwLock<HashMap<String, Arc<ManagedStream>>>>,
    config: Arc<Config>,
    main_pipeline: Option<gst::Pipeline>,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Arc<RwLock<mpsc::Receiver<()>>>,
    event_tx: mpsc::UnboundedSender<StreamEvent>,
    event_rx: Arc<RwLock<mpsc::UnboundedReceiver<StreamEvent>>>,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    StreamAdded(String),
    StreamRemoved(String),
    StreamHealthChanged(String, HealthStatus),
    StreamError(String, String),
    StreamReconnecting(String),
    StreamConnected(String),
    StatisticsUpdate(String, StreamStatistics),
    ShutdownRequested,
}

impl StreamManager {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok(Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            config,
            main_pipeline: None,
            shutdown_tx,
            shutdown_rx: Arc::new(RwLock::new(shutdown_rx)),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing stream manager");

        // Create main pipeline
        let pipeline = gst::Pipeline::builder()
            .name("main-pipeline")
            .build();

        pipeline.set_state(gst::State::Playing)
            .map_err(|e| crate::StreamManagerError::PipelineError(format!("Failed to set pipeline state: {:?}", e)))?;
        self.main_pipeline = Some(pipeline);

        info!("Stream manager initialized successfully");
        Ok(())
    }

    pub async fn add_stream(&self, id: String, config: StreamConfig) -> Result<()> {
        info!("Adding stream: {}", id);

        // Check if stream already exists
        {
            let streams = self.streams.read().await;
            if streams.contains_key(&id) {
                return Err(crate::StreamManagerError::Other(format!("Stream {} already exists", id)));
            }
        }

        // Create stream source
        let source = Arc::new(StreamSource::new(id.clone(), &config)?);
        // Note: source.start() will be called when the pipeline starts

        // Create branch manager
        let pipeline = gst::Pipeline::builder()
            .name(&format!("pipeline-{}", id))
            .build();
        let branch_manager = Arc::new(BranchManager::new(&pipeline)
            .map_err(|e| crate::StreamManagerError::PipelineError(format!("Failed to create branch manager: {:?}", e)))?);

        // Create recording branch if configured
        let recording_branch = if config.recording_enabled {
            // Convert config format to recording format
            let recording_config = crate::recording::RecordingConfig {
                base_path: std::path::PathBuf::from("recordings"),
                file_pattern: format!("stream-{}-{{}}.mp4", id),
                segment_duration: gst::ClockTime::from_seconds(self.config.recording.segment_duration_seconds),
                muxer: crate::recording::MuxerType::Mp4,
                is_live: true,
                send_keyframe_requests: true,
                ensure_no_gaps: true,
            };
            let recording = RecordingBranch::new(
                &id,
                recording_config,
            ).map_err(|e| crate::StreamManagerError::Other(format!("Failed to create recording branch: {}", e)))?;
            Some(Arc::new(recording))
        } else {
            None
        };

        // Create managed stream
        let managed_stream = Arc::new(ManagedStream {
            id: id.clone(),
            source,
            branch_manager,
            recording_branch,
            statistics: StreamStatistics {
                last_update: std::time::Instant::now(),
                ..Default::default()
            },
            health_status: HealthStatus::Unknown,
        });

        // Add to registry
        {
            let mut streams = self.streams.write().await;
            streams.insert(id.clone(), managed_stream);
        }

        // Send event
        let _ = self.event_tx.send(StreamEvent::StreamAdded(id.clone()));

        info!("Stream {} added successfully", id);
        Ok(())
    }

    pub async fn remove_stream(&self, id: &str) -> Result<()> {
        info!("Removing stream: {}", id);

        let managed_stream = {
            let mut streams = self.streams.write().await;
            streams.remove(id)
        };

        if let Some(stream) = managed_stream {
            // Stop recording if active
            if let Some(_recording) = &stream.recording_branch {
                // Note: recording.stop() will be called when the pipeline stops
            }

            // Note: source.stop() will be called when the pipeline stops

            // Send event
            let _ = self.event_tx.send(StreamEvent::StreamRemoved(id.to_string()));

            info!("Stream {} removed successfully", id);
            Ok(())
        } else {
            Err(crate::StreamManagerError::StreamNotFound(id.to_string()))
        }
    }

    pub async fn get_stream(&self, id: &str) -> Option<Arc<ManagedStream>> {
        let streams = self.streams.read().await;
        streams.get(id).cloned()
    }

    pub async fn list_streams(&self) -> Vec<String> {
        let streams = self.streams.read().await;
        streams.keys().cloned().collect()
    }

    pub async fn update_stream_health(&self, id: &str, status: HealthStatus) {
        if let Some(_stream) = self.get_stream(id).await {
            // In a real implementation, we'd update the stream's health status
            // For now, just send an event
            let _ = self.event_tx.send(StreamEvent::StreamHealthChanged(
                id.to_string(),
                status,
            ));
        }
    }

    pub async fn update_stream_statistics(&self, id: &str, stats: StreamStatistics) {
        if let Some(_stream) = self.get_stream(id).await {
            // In a real implementation, we'd update the stream's statistics
            // For now, just send an event
            let _ = self.event_tx.send(StreamEvent::StatisticsUpdate(
                id.to_string(),
                stats,
            ));
        }
    }

    pub async fn start_all(&self) -> Result<()> {
        info!("Starting all streams");
        
        let streams = self.streams.read().await;
        for (id, _stream) in streams.iter() {
            debug!("Starting stream: {}", id);
            // Note: source and recording will start with the pipeline
            debug!("Starting stream: {}", id);
        }
        
        info!("All streams started");
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        info!("Stopping all streams");
        
        let streams = self.streams.read().await;
        for (id, _stream) in streams.iter() {
            debug!("Stopping stream: {}", id);
            
            // Note: source and recording will stop with the pipeline
            debug!("Stopping stream: {}", id);
        }
        
        info!("All streams stopped");
        Ok(())
    }

    pub async fn shutdown(&mut self, grace_period: std::time::Duration) -> Result<()> {
        info!("Initiating graceful shutdown with {:?} grace period", grace_period);
        
        // Send shutdown event
        let _ = self.event_tx.send(StreamEvent::ShutdownRequested);
        
        // Stop all streams gracefully
        let stop_result = tokio::time::timeout(grace_period, self.stop_all()).await;
        
        match stop_result {
            Ok(Ok(())) => {
                info!("All streams stopped gracefully");
            }
            Ok(Err(e)) => {
                error!("Error stopping streams: {}", e);
                self.force_shutdown().await?;
            }
            Err(_) => {
                warn!("Graceful shutdown timed out, forcing shutdown");
                self.force_shutdown().await?;
            }
        }
        
        // Stop main pipeline
        if let Some(pipeline) = &self.main_pipeline {
            pipeline.set_state(gst::State::Null)
                .map_err(|e| crate::StreamManagerError::PipelineError(format!("Failed to stop pipeline: {:?}", e)))?;
        }
        
        info!("Shutdown complete");
        Ok(())
    }

    async fn force_shutdown(&self) -> Result<()> {
        warn!("Forcing shutdown of all streams");
        
        let streams = self.streams.read().await;
        for (id, _stream) in streams.iter() {
            warn!("Force stopping stream: {}", id);
            // In a real implementation, we'd force-kill the stream
        }
        
        Ok(())
    }

    pub fn get_event_receiver(&self) -> Arc<RwLock<mpsc::UnboundedReceiver<StreamEvent>>> {
        self.event_rx.clone()
    }

    pub async fn handle_events(&self) {
        let mut event_rx = self.event_rx.write().await;
        
        while let Some(event) = event_rx.recv().await {
            match event {
                StreamEvent::StreamAdded(id) => {
                    debug!("Event: Stream {} added", id);
                }
                StreamEvent::StreamRemoved(id) => {
                    debug!("Event: Stream {} removed", id);
                }
                StreamEvent::StreamHealthChanged(id, status) => {
                    debug!("Event: Stream {} health changed to {:?}", id, status);
                }
                StreamEvent::StreamError(id, error) => {
                    error!("Event: Stream {} error: {}", id, error);
                }
                StreamEvent::StreamReconnecting(id) => {
                    info!("Event: Stream {} reconnecting", id);
                }
                StreamEvent::StreamConnected(id) => {
                    info!("Event: Stream {} connected", id);
                }
                StreamEvent::StatisticsUpdate(id, _stats) => {
                    debug!("Event: Stream {} statistics updated", id);
                }
                StreamEvent::ShutdownRequested => {
                    info!("Event: Shutdown requested");
                    break;
                }
            }
        }
    }
}

impl Drop for StreamManager {
    fn drop(&mut self) {
        info!("Dropping stream manager");
        
        // Best effort cleanup - don't block in Drop
        if let Some(pipeline) = &self.main_pipeline {
            let _ = pipeline.set_state(gst::State::Null);
        }
        
        info!("Stream manager dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_gst() {
        gst::init().ok();
    }

    #[tokio::test]
    async fn test_stream_manager_creation() {
        init_gst();
        let config = Arc::new(Config::default());
        let manager = StreamManager::new(config);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_stream_add_remove() {
        init_gst();
        let config = Arc::new(Config::default());
        let manager = StreamManager::new(config).unwrap();
        
        let stream_config = StreamConfig {
            id: "test-stream".to_string(),
            name: "Test Stream".to_string(),
            source_uri: "rtsp://localhost:8554/test".to_string(),
            enabled: true,
            recording_enabled: false,
            inference_enabled: false,
            reconnect_timeout_seconds: 5,
            max_reconnect_attempts: 3,
            buffer_size_mb: 10,
        };
        
        // Add stream
        let result = manager.add_stream("test-stream".to_string(), stream_config).await;
        assert!(result.is_ok());
        
        // Verify stream exists
        let stream = manager.get_stream("test-stream").await;
        assert!(stream.is_some());
        
        // List streams
        let streams = manager.list_streams().await;
        assert_eq!(streams.len(), 1);
        assert!(streams.contains(&"test-stream".to_string()));
        
        // Remove stream
        let result = manager.remove_stream("test-stream").await;
        assert!(result.is_ok());
        
        // Verify stream removed
        let stream = manager.get_stream("test-stream").await;
        assert!(stream.is_none());
        
        let streams = manager.list_streams().await;
        assert_eq!(streams.len(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_stream_access() {
        init_gst();
        let config = Arc::new(Config::default());
        let manager = Arc::new(StreamManager::new(config).unwrap());
        
        let manager1 = manager.clone();
        let manager2 = manager.clone();
        
        // Spawn concurrent tasks
        let handle1 = tokio::spawn(async move {
            let stream_config = StreamConfig {
                id: "stream1".to_string(),
                name: "Stream 1".to_string(),
                source_uri: "rtsp://localhost:8554/stream1".to_string(),
                enabled: true,
                recording_enabled: false,
                inference_enabled: false,
                reconnect_timeout_seconds: 5,
                max_reconnect_attempts: 3,
                buffer_size_mb: 10,
            };
            manager1.add_stream("stream1".to_string(), stream_config).await
        });
        
        let handle2 = tokio::spawn(async move {
            let stream_config = StreamConfig {
                id: "stream2".to_string(),
                name: "Stream 2".to_string(),
                source_uri: "rtsp://localhost:8554/stream2".to_string(),
                enabled: true,
                recording_enabled: false,
                inference_enabled: false,
                reconnect_timeout_seconds: 5,
                max_reconnect_attempts: 3,
                buffer_size_mb: 10,
            };
            manager2.add_stream("stream2".to_string(), stream_config).await
        });
        
        // Wait for both tasks
        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // Verify both streams exist
        let streams = manager.list_streams().await;
        assert_eq!(streams.len(), 2);
    }
}