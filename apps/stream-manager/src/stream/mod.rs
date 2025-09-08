use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use tracing::{debug, error, info};

use crate::config::StreamConfig;

pub mod source;
pub use source::{StreamSource, SourceType, SourceHealth, SourceStatistics, SourceMessage};

#[derive(Debug, Clone)]
pub struct Stream {
    pub id: String,
    pub name: String,
    pub source_uri: String,
    pub status: StreamStatus,
    pub source_type: SourceType,
    pub source_health: SourceHealth,
    pub statistics: SourceStatistics,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamStatus {
    Idle,
    Connecting,
    Active,
    Recording,
    Error(String),
}

pub struct StreamManager {
    streams: Arc<RwLock<Vec<Stream>>>,
    sources: Arc<RwLock<HashMap<String, StreamSource>>>,
    message_receiver: Option<mpsc::UnboundedReceiver<(String, SourceMessage)>>,
    message_sender: mpsc::UnboundedSender<(String, SourceMessage)>,
}

impl StreamManager {
    pub fn new() -> Self {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        
        Self {
            streams: Arc::new(RwLock::new(Vec::new())),
            sources: Arc::new(RwLock::new(HashMap::new())),
            message_receiver: Some(message_receiver),
            message_sender,
        }
    }

    pub async fn add_stream(&self, name: String, source_uri: String) -> crate::Result<String> {
        self.add_stream_from_config(StreamConfig {
            id: Uuid::new_v4().to_string(),
            name,
            source_uri,
            enabled: true,
            recording_enabled: false,
            inference_enabled: false,
            reconnect_timeout_seconds: 5,
            max_reconnect_attempts: 10,
            buffer_size_mb: 50,
        }).await
    }

    pub async fn add_stream_from_config(&self, config: StreamConfig) -> crate::Result<String> {
        let id = config.id.clone();
        
        debug!("Adding stream from config: {}", id);

        // Create stream source
        let mut stream_source = StreamSource::new(id.clone(), &config)?;
        
        // Create message sender for this source
        let source_sender = {
            let global_sender = self.message_sender.clone();
            let stream_id = id.clone();
            let (tx, mut rx) = mpsc::unbounded_channel();
            
            // Spawn task to forward source messages with stream ID
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    let _ = global_sender.send((stream_id.clone(), msg));
                }
            });
            
            tx
        };

        stream_source.set_message_sender(source_sender);

        // Create the stream record
        let source_type = stream_source.get_source_type();
        let stream = Stream {
            id: id.clone(),
            name: config.name.clone(),
            source_uri: config.source_uri.clone(),
            status: StreamStatus::Idle,
            source_type,
            source_health: SourceHealth::Unknown,
            statistics: SourceStatistics::default(),
        };
        
        // Store both stream and source
        let mut streams = self.streams.write().await;
        let mut sources = self.sources.write().await;
        
        streams.push(stream);
        sources.insert(id.clone(), stream_source);
        
        info!("Stream added successfully: {} ({})", config.name, id);
        Ok(id)
    }

    pub async fn get_stream(&self, id: &str) -> Option<Stream> {
        let streams = self.streams.read().await;
        streams.iter().find(|s| s.id == id).cloned()
    }

    pub async fn list_streams(&self) -> Vec<Stream> {
        self.streams.read().await.clone()
    }

    /// Start source monitoring and message handling
    pub async fn start_monitoring(&mut self) -> crate::Result<()> {
        let message_receiver = self.message_receiver.take().ok_or_else(|| {
            crate::StreamManagerError::Other("Message receiver already taken".to_string())
        })?;

        let streams_ref = Arc::clone(&self.streams);
        let sources_ref = Arc::clone(&self.sources);

        tokio::spawn(async move {
            Self::handle_source_messages(message_receiver, streams_ref, sources_ref).await;
        });

        info!("Stream source monitoring started");
        Ok(())
    }

    /// Handle messages from stream sources
    async fn handle_source_messages(
        mut receiver: mpsc::UnboundedReceiver<(String, SourceMessage)>,
        streams: Arc<RwLock<Vec<Stream>>>,
        _sources: Arc<RwLock<HashMap<String, StreamSource>>>,
    ) {
        while let Some((stream_id, message)) = receiver.recv().await {
            debug!("Received message from stream {}: {:?}", stream_id, message);

            match message {
                SourceMessage::StatisticsUpdate(stats) => {
                    Self::update_stream_statistics(&streams, &stream_id, stats).await;
                }
                SourceMessage::HealthUpdate(health) => {
                    Self::update_stream_health(&streams, &stream_id, health).await;
                }
                SourceMessage::StateChanged(state) => {
                    Self::handle_state_change(&streams, &stream_id, state).await;
                }
                SourceMessage::Error(error) => {
                    Self::handle_source_error(&streams, &stream_id, error).await;
                }
                SourceMessage::Eos => {
                    Self::handle_source_eos(&streams, &stream_id).await;
                }
                SourceMessage::Buffering(percent) => {
                    debug!("Stream {} buffering: {}%", stream_id, percent);
                }
            }
        }
    }

    async fn update_stream_statistics(
        streams: &Arc<RwLock<Vec<Stream>>>,
        stream_id: &str,
        stats: SourceStatistics,
    ) {
        let mut streams = streams.write().await;
        if let Some(stream) = streams.iter_mut().find(|s| s.id == stream_id) {
            stream.statistics = stats;
        }
    }

    async fn update_stream_health(
        streams: &Arc<RwLock<Vec<Stream>>>,
        stream_id: &str,
        health: SourceHealth,
    ) {
        let mut streams = streams.write().await;
        if let Some(stream) = streams.iter_mut().find(|s| s.id == stream_id) {
            stream.source_health = health;
        }
    }

    async fn handle_state_change(
        streams: &Arc<RwLock<Vec<Stream>>>,
        stream_id: &str,
        state: gst::State,
    ) {
        let new_status = match state {
            gst::State::Null | gst::State::Ready => StreamStatus::Idle,
            gst::State::Paused => StreamStatus::Connecting,
            gst::State::Playing => StreamStatus::Active,
            _ => return,
        };

        let mut streams = streams.write().await;
        if let Some(stream) = streams.iter_mut().find(|s| s.id == stream_id) {
            stream.status = new_status;
            debug!("Stream {} state changed to: {:?}", stream_id, stream.status);
        }
    }

    async fn handle_source_error(
        streams: &Arc<RwLock<Vec<Stream>>>,
        stream_id: &str,
        error: String,
    ) {
        error!("Stream {} error: {}", stream_id, error);
        
        let mut streams = streams.write().await;
        if let Some(stream) = streams.iter_mut().find(|s| s.id == stream_id) {
            stream.status = StreamStatus::Error(error);
            stream.source_health = SourceHealth::Unhealthy;
        }
    }

    async fn handle_source_eos(
        streams: &Arc<RwLock<Vec<Stream>>>,
        stream_id: &str,
    ) {
        info!("Stream {} received EOS", stream_id);
        
        let mut streams = streams.write().await;
        if let Some(stream) = streams.iter_mut().find(|s| s.id == stream_id) {
            stream.status = StreamStatus::Idle;
        }
    }

    /// Get stream source for direct access
    pub async fn get_stream_source(&self, stream_id: &str) -> Option<StreamSource> {
        let sources = self.sources.read().await;
        sources.get(stream_id).cloned()
    }

    /// Create source bin for a stream
    pub async fn create_source_bin(&self, stream_id: &str) -> crate::Result<gst::Bin> {
        let mut sources = self.sources.write().await;
        let source = sources.get_mut(stream_id)
            .ok_or_else(|| crate::StreamManagerError::StreamNotFound(stream_id.to_string()))?;

        let bin = source.create_source_bin()?;
        Ok(bin)
    }

    /// Update source statistics for all sources
    pub async fn update_all_statistics(&self) {
        let sources = self.sources.read().await;
        let source_data: Vec<(String, SourceStatistics, SourceHealth)> = sources.iter()
            .map(|(stream_id, source)| {
                let stats = source.get_statistics();
                let health = source.get_health_status();
                (stream_id.clone(), stats, health)
            })
            .collect();
        drop(sources); // Release read lock
        
        // Update all stats without holding the lock
        for (stream_id, stats, health) in source_data {
            Self::update_stream_statistics(&self.streams, &stream_id, stats).await;
            Self::update_stream_health(&self.streams, &stream_id, health).await;
        }
    }

    /// Remove a stream and its source
    pub async fn remove_stream(&self, stream_id: &str) -> crate::Result<()> {
        info!("Removing stream: {}", stream_id);

        let mut streams = self.streams.write().await;
        let mut sources = self.sources.write().await;

        // Remove from collections
        streams.retain(|s| s.id != stream_id);
        sources.remove(stream_id);

        info!("Stream {} removed successfully", stream_id);
        Ok(())
    }
}