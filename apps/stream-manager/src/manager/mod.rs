use crate::config::{Config, StreamConfig};
use crate::stream::{BranchManager, StreamSource};
use crate::recording::RecordingBranch;
use crate::inference::{InferenceManager, InferenceBackend};
use crate::Result;
use gst::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

fn calculate_bitrate(bytes: u64, duration: std::time::Duration) -> f64 {
    if duration.as_secs() == 0 {
        return 0.0;
    }
    (bytes as f64 * 8.0) / duration.as_secs_f64()
}

fn calculate_fps(frames: u64, duration: std::time::Duration) -> f64 {
    if duration.as_secs() == 0 {
        return 0.0;
    }
    frames as f64 / duration.as_secs_f64()
}

mod stream_info;
pub use stream_info::{StreamInfo, StreamState, StreamHealth, RecordingState};

#[cfg(test)]
pub mod test_utils;

#[derive(Debug)]
pub struct ManagedStream {
    pub id: String,
    pub source: Arc<StreamSource>,
    pub branch_manager: Arc<BranchManager>,
    pub recording_branch: Option<Arc<RecordingBranch>>,
    pub inference_enabled: bool,
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
    inference_manager: Option<Arc<InferenceManager>>,
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

        // Create inference manager if NVIDIA support is configured
        let inference_manager = if std::env::var("ENABLE_NVIDIA_INFERENCE").unwrap_or_default() == "true" {
            Some(Arc::new(InferenceManager::with_nvidia_support(4, 8192)))
        } else {
            None
        };

        Ok(Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            config,
            main_pipeline: None,
            inference_manager,
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

        // Setup inference if enabled
        let inference_enabled = config.inference_enabled;
        if inference_enabled {
            if let Some(ref inference_mgr) = self.inference_manager {
                // Default to NVIDIA inference if available
                let nvidia_config = crate::inference::nvidia::NvidiaInferenceConfig::default();
                let backend = InferenceBackend::Nvidia(nvidia_config);
                
                if let Err(e) = inference_mgr.add_inference_stream(id.clone(), backend).await {
                    error!("Failed to add inference for stream {}: {}", id, e);
                } else {
                    info!("Inference enabled for stream {}", id);
                }
            }
        }

        // Create managed stream
        let managed_stream = Arc::new(ManagedStream {
            id: id.clone(),
            source,
            branch_manager,
            recording_branch,
            inference_enabled,
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
            // Stop inference if active
            if stream.inference_enabled {
                if let Some(ref inference_mgr) = self.inference_manager {
                    if let Err(e) = inference_mgr.remove_inference_stream(id).await {
                        error!("Failed to remove inference for stream {}: {}", id, e);
                    }
                }
            }

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

    pub async fn list_streams(&self) -> Vec<StreamInfo> {
        let streams = self.streams.read().await;
        let mut result = Vec::new();
        
        for id in streams.keys() {
            if let Ok(info) = self.get_stream_info(id).await {
                result.push(info);
            }
        }
        
        result
    }
    
    pub async fn get_all_stream_statistics(&self) -> Result<Vec<(String, crate::api::dto::StreamStatistics)>> {
        let streams = self.streams.read().await;
        let mut result = Vec::new();
        
        for (id, stream) in streams.iter() {
            // Create StreamStatistics from the internal statistics
            let stats = crate::api::dto::StreamStatistics {
                frames_processed: stream.statistics.packets_received,
                bytes_processed: stream.statistics.bytes_received,
                dropped_frames: stream.statistics.dropped_frames,
                bitrate: calculate_bitrate(stream.statistics.bytes_received, stream.statistics.last_update.elapsed()),
                fps: calculate_fps(stream.statistics.packets_received, stream.statistics.last_update.elapsed()),
                latency_ms: 0.0, // Would need actual latency measurement
                uptime_seconds: stream.statistics.last_update.elapsed().as_secs(),
                last_frame_time: Some(chrono::Utc::now().to_rfc3339()),
            };
            
            result.push((id.clone(), stats));
        }
        
        Ok(result)
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

    pub async fn process_inference_results(&self) -> Result<()> {
        if let Some(ref inference_mgr) = self.inference_manager {
            inference_mgr.process_results().await
                .map_err(|e| crate::StreamManagerError::Other(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn enable_inference_for_stream(&self, id: &str, backend: InferenceBackend) -> Result<()> {
        if let Some(ref inference_mgr) = self.inference_manager {
            inference_mgr.add_inference_stream(id.to_string(), backend).await
                .map_err(|e| crate::StreamManagerError::Other(e.to_string()))?;
            
            // Update stream state
            let streams = self.streams.read().await;
            if let Some(_stream) = streams.get(id) {
                info!("Inference enabled for stream {}", id);
            }
        } else {
            return Err(crate::StreamManagerError::Other("Inference manager not initialized".to_string()));
        }
        Ok(())
    }

    pub async fn disable_inference_for_stream(&self, id: &str) -> Result<()> {
        if let Some(ref inference_mgr) = self.inference_manager {
            inference_mgr.remove_inference_stream(id).await
                .map_err(|e| crate::StreamManagerError::Other(e.to_string()))?;
            info!("Inference disabled for stream {}", id);
        }
        Ok(())
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

    pub async fn get_stream_info(&self, id: &str) -> Result<StreamInfo> {
        let streams = self.streams.read().await;
        let stream = streams.get(id)
            .ok_or_else(|| crate::StreamManagerError::StreamNotFound(id.to_string()))?;
        
        // Convert ManagedStream to StreamInfo
        let health = StreamHealth::from(&stream.statistics);
        let health = StreamHealth {
            is_healthy: stream.health_status == HealthStatus::Healthy,
            ..health
        };
        
        let recording_state = if let Some(_recording) = &stream.recording_branch {
            RecordingState {
                is_recording: true, // TODO: Get actual state from recording branch
                current_file: Some(format!("stream-{}.mp4", id)),
                duration: Some(std::time::Duration::from_secs(0)),
                bytes_written: Some(0),
            }
        } else {
            RecordingState::default()
        };
        
        // Get config from somewhere - for now use a default
        let config = StreamConfig {
            id: id.to_string(),
            name: id.to_string(),
            source_uri: format!("rtsp://localhost:8554/{}", id),
            enabled: true,
            recording_enabled: stream.recording_branch.is_some(),
            inference_enabled: false,
            reconnect_timeout_seconds: 5,
            max_reconnect_attempts: 3,
            buffer_size_mb: 10,
        };
        
        Ok(StreamInfo {
            id: stream.id.clone(),
            config,
            state: StreamState::Running,
            health,
            recording_state,
        })
    }
    
    pub async fn start_recording(&self, id: &str) -> Result<()> {
        let streams = self.streams.read().await;
        let stream = streams.get(id)
            .ok_or_else(|| crate::StreamManagerError::StreamNotFound(id.to_string()))?;
        
        if let Some(_recording) = &stream.recording_branch {
            // TODO: Implement actual start recording
            info!("Starting recording for stream: {}", id);
            Ok(())
        } else {
            Err(crate::StreamManagerError::Other("Recording not configured for stream".to_string()))
        }
    }
    
    pub async fn stop_recording(&self, id: &str) -> Result<()> {
        let streams = self.streams.read().await;
        let stream = streams.get(id)
            .ok_or_else(|| crate::StreamManagerError::StreamNotFound(id.to_string()))?;
        
        if let Some(_recording) = &stream.recording_branch {
            // TODO: Implement actual stop recording
            info!("Stopping recording for stream: {}", id);
            Ok(())
        } else {
            Err(crate::StreamManagerError::Other("Recording not configured for stream".to_string()))
        }
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
        assert!(streams.iter().any(|s| s.id == "test-stream"));
        
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