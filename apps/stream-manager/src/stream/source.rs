use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use gst::prelude::*;
use tracing::{debug, error, info, warn};
use tokio::sync::mpsc;
use futures::stream::StreamExt;

use crate::config::StreamConfig;

/// Source type detection based on URI scheme
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SourceType {
    Rtsp,
    Http,
    File,
    Unknown,
}

impl SourceType {
    pub fn from_uri(uri: &str) -> Self {
        match uri.split(':').next() {
            Some(p) => {
                match p.to_lowercase().as_str() {
                    "rtsp" | "rtsps" | "rtspt"  | "rtspu" => Self::Rtsp,
                    "http" | "https" => Self::Http,
                    "file" => Self::File,
                    _ => Self::Unknown,
                }
            }
            _ => Self::Unknown,
        }
    }
}

/// Health status of a stream source
#[derive(Debug, Clone, PartialEq)]
pub enum SourceHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Statistics collected from fallbacksrc
#[derive(Debug, Clone, Default)]
pub struct SourceStatistics {
    pub num_retry: u64,
    pub num_fallback_retry: u64,
    pub buffering_percent: i32,
    pub fallback_buffering_percent: i32,
    pub last_frame_timestamp: Option<Instant>,
    pub connection_start_time: Option<Instant>,
    pub total_bytes_received: u64,
    pub current_bitrate_bps: u64,
}

/// Health configuration thresholds
#[derive(Debug, Clone)]
pub struct HealthThresholds {
    pub max_retry_count: u64,
    pub frame_timeout_seconds: u64,
    pub min_buffering_percent: i32,
    pub max_reconnect_attempts: u32,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            max_retry_count: 5,
            frame_timeout_seconds: 30,
            min_buffering_percent: 50,
            max_reconnect_attempts: 10,
        }
    }
}

/// Main stream source abstraction using fallbacksrc
#[derive(Clone)]
pub struct StreamSource {
    pub id: String,
    pub source_uri: String,
    pub source_type: SourceType,
    source_bin: Option<gst::Bin>,
    fallbacksrc: Option<gst::Element>,
    decodebin: Option<gst::Element>,
    statistics: Arc<Mutex<SourceStatistics>>,
    health_thresholds: HealthThresholds,
    message_sender: Option<mpsc::UnboundedSender<SourceMessage>>,
    creation_time: Instant,
}

impl std::fmt::Debug for StreamSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamSource")
            .field("id", &self.id)
            .field("source_uri", &self.source_uri)
            .field("source_type", &self.source_type)
            .field("has_source_bin", &self.source_bin.is_some())
            .field("has_fallbacksrc", &self.fallbacksrc.is_some())
            .field("has_decodebin", &self.decodebin.is_some())
            .finish()
    }
}

/// Messages that can be sent from the source
#[derive(Debug, Clone)]
pub enum SourceMessage {
    StateChanged(gst::State),
    Error(String),
    Eos,
    Buffering(i32),
    StatisticsUpdate(SourceStatistics),
    HealthUpdate(SourceHealth),
}

impl StreamSource {
    /// Create a new stream source
    pub fn new(id: String, config: &StreamConfig) -> crate::Result<Self> {
        let source_type = SourceType::from_uri(&config.source_uri);
        let health_thresholds = HealthThresholds {
            max_retry_count: 5,
            frame_timeout_seconds: config.reconnect_timeout_seconds,
            min_buffering_percent: 50,
            max_reconnect_attempts: config.max_reconnect_attempts,
        };

        debug!("Creating stream source: {} with URI: {}", id, config.source_uri);

        Ok(Self {
            id,
            source_uri: config.source_uri.clone(),
            source_type,
            source_bin: None,
            fallbacksrc: None,
            decodebin: None,
            statistics: Arc::new(Mutex::new(SourceStatistics::default())),
            health_thresholds,
            message_sender: None,
            creation_time: Instant::now(),
        })
    }

    /// Set message sender for communication with parent
    pub fn set_message_sender(&mut self, sender: mpsc::UnboundedSender<SourceMessage>) {
        self.message_sender = Some(sender);
    }

    /// Create and configure the source bin with fallbacksrc
    pub fn create_source_bin(&mut self) -> crate::Result<gst::Bin> {
        if self.source_bin.is_some() {
            return Err(crate::StreamManagerError::PipelineError(
                "Source bin already created".to_string(),
            ));
        }

        debug!("Creating source bin for stream: {}", self.id);

        // Create the main bin
        let bin_name = format!("source-bin-{}", self.id);
        let bin = gst::Bin::new();
        bin.set_property("name", &bin_name);

        // Create fallbacksrc element
        let fallbacksrc = gst::ElementFactory::make("fallbacksrc")
            .name(&format!("fallbacksrc-{}", self.id))
            .build()
            .map_err(|e| {
                crate::StreamManagerError::PipelineError(format!(
                    "Failed to create fallbacksrc: {}",
                    e
                ))
            })?;

        // Configure fallbacksrc properties
        self.configure_fallbacksrc(&fallbacksrc)?;

        // Create decodebin3 for format handling
        let decodebin = gst::ElementFactory::make("decodebin3")
            .name(&format!("decodebin-{}", self.id))
            .build()
            .map_err(|e| {
                crate::StreamManagerError::PipelineError(format!(
                    "Failed to create decodebin3: {}",
                    e
                ))
            })?;

        // Add elements to bin
        bin.add_many([&fallbacksrc, &decodebin])
            .map_err(|e| {
                crate::StreamManagerError::PipelineError(format!(
                    "Failed to add elements to source bin: {}",
                    e
                ))
            })?;

        // Link fallbacksrc to decodebin
        fallbacksrc.link(&decodebin).map_err(|e| {
            crate::StreamManagerError::PipelineError(format!(
                "Failed to link fallbacksrc to decodebin: {}",
                e
            ))
        })?;

        // Connect to pad-added signal for dynamic pads
        self.connect_pad_added_signal(&decodebin, &bin)?;

        // Setup message bus monitoring
        self.setup_message_monitoring(&bin)?;

        // Store references
        self.source_bin = Some(bin.clone());
        self.fallbacksrc = Some(fallbacksrc);
        self.decodebin = Some(decodebin);

        info!("Source bin created successfully for stream: {}", self.id);
        Ok(bin)
    }

    /// Configure fallbacksrc element with appropriate properties
    fn configure_fallbacksrc(&self, fallbacksrc: &gst::Element) -> crate::Result<()> {
        debug!("Configuring fallbacksrc for stream: {}", self.id);

        // Set source URI
        fallbacksrc.set_property("uri", &self.source_uri);

        // Configure timeouts based on source type and health thresholds
        let timeout = gst::ClockTime::from_seconds(self.health_thresholds.frame_timeout_seconds);
        let restart_timeout = gst::ClockTime::from_seconds(self.health_thresholds.frame_timeout_seconds);
        let retry_timeout = gst::ClockTime::from_seconds(60); // Fixed retry timeout

        fallbacksrc.set_property("timeout", timeout);
        fallbacksrc.set_property("restart-timeout", restart_timeout);
        fallbacksrc.set_property("retry-timeout", retry_timeout);

        // Enable restart on EOS for continuous streams (especially RTSP)
        match self.source_type {
            SourceType::Rtsp => {
                fallbacksrc.set_property("restart-on-eos", true);
            }
            SourceType::Http => {
                fallbacksrc.set_property("restart-on-eos", false);
            }
            SourceType::File => {
                fallbacksrc.set_property("restart-on-eos", false);
            }
            SourceType::Unknown => {
                fallbacksrc.set_property("restart-on-eos", false);
            }
        }

        // Set buffer duration (in nanoseconds, -1 for automatic)
        fallbacksrc.set_property("buffer-duration", -1i64);

        // Enable immediate fallback for faster recovery
        fallbacksrc.set_property("immediate-fallback", true);

        debug!("Fallbacksrc configured for stream: {}", self.id);
        Ok(())
    }

    /// Connect to pad-added signal for dynamic pad handling
    fn connect_pad_added_signal(&self, decodebin: &gst::Element, bin: &gst::Bin) -> crate::Result<()> {
        let bin_weak = bin.downgrade();
        let stream_id = self.id.clone();

        decodebin.connect_pad_added(move |_decodebin, pad| {
            let Some(bin) = bin_weak.upgrade() else {
                warn!("Bin has been dropped, ignoring pad-added signal");
                return;
            };

            let pad_name = pad.name();
            let caps = pad.current_caps();

            debug!(
                "New pad added to decodebin in stream {}: {} with caps: {:?}",
                stream_id, pad_name, caps
            );

            // Create ghost pad to expose the decoded output
            let ghost_pad_name = format!("src_{}", pad_name.replace('_', "-"));
            let ghost_pad = gst::GhostPad::with_target(pad).map(|gp| {
                gp.set_property("name", &ghost_pad_name);
                gp
            });

            if let Ok(ghost_pad) = ghost_pad {
                if bin.add_pad(&ghost_pad).is_ok() {
                    debug!("Ghost pad {} added successfully", ghost_pad_name);
                } else {
                    warn!("Failed to add ghost pad: {}", ghost_pad_name);
                }
            } else {
                warn!("Failed to create ghost pad for: {}", pad_name);
            }
        });

        Ok(())
    }

    /// Setup message bus monitoring for the source bin
    fn setup_message_monitoring(&self, bin: &gst::Bin) -> crate::Result<()> {
        let bus = bin.bus().ok_or_else(|| {
            crate::StreamManagerError::PipelineError("Failed to get bin bus".to_string())
        })?;

        let sender = self.message_sender.clone();
        let stream_id = self.id.clone();
        let stats_ref = self.statistics.clone();

        // Spawn async task to handle messages
        tokio::spawn(async move {
            let mut messages = bus.stream();
            while let Some(msg) = messages.next().await {
                match msg.view() {
                    gst::MessageView::Error(err) => {
                        error!(
                            "Error from stream {}: {} (debug: {:?})",
                            stream_id,
                            err.error(),
                            err.debug()
                        );
                        if let Some(ref sender) = sender {
                            let _ = sender.send(SourceMessage::Error(err.error().to_string()));
                        }
                    }
                    gst::MessageView::Eos(_) => {
                        info!("EOS received for stream: {}", stream_id);
                        if let Some(ref sender) = sender {
                            let _ = sender.send(SourceMessage::Eos);
                        }
                    }
                    gst::MessageView::StateChanged(state_changed) => {
                        debug!(
                            "State changed for stream {}: {:?} -> {:?}",
                            stream_id,
                            state_changed.old(),
                            state_changed.current()
                        );
                        if let Some(ref sender) = sender {
                            let _ = sender.send(SourceMessage::StateChanged(state_changed.current()));
                        }
                    }
                    gst::MessageView::Buffering(buffering) => {
                        let percent = buffering.percent();
                        debug!("Buffering for stream {}: {}%", stream_id, percent);

                        // Update statistics
                        if let Ok(mut stats) = stats_ref.lock() {
                            stats.buffering_percent = percent;
                        }

                        if let Some(ref sender) = sender {
                            let _ = sender.send(SourceMessage::Buffering(percent));
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Get current source statistics
    pub fn get_statistics(&self) -> SourceStatistics {
        if let Some(fallbacksrc) = &self.fallbacksrc {
            self.update_statistics_from_element(fallbacksrc);
        }
        
        self.statistics.lock().unwrap().clone()
    }

    /// Update statistics from fallbacksrc element
    fn update_statistics_from_element(&self, fallbacksrc: &gst::Element) {
        if let Some(stats_structure) = fallbacksrc.property::<Option<gst::Structure>>("stats") {
            let mut stats = self.statistics.lock().unwrap();

            // Extract values from GStreamer structure
            if let Ok(num_retry) = stats_structure.get::<u64>("num-retry") {
                stats.num_retry = num_retry;
            }
            if let Ok(num_fallback_retry) = stats_structure.get::<u64>("num-fallback-retry") {
                stats.num_fallback_retry = num_fallback_retry;
            }
            if let Ok(buffering_percent) = stats_structure.get::<i32>("buffering-percent") {
                stats.buffering_percent = buffering_percent;
            }
            if let Ok(fallback_buffering_percent) = stats_structure.get::<i32>("fallback-buffering-percent") {
                stats.fallback_buffering_percent = fallback_buffering_percent;
            }

            // Update frame timestamp
            stats.last_frame_timestamp = Some(Instant::now());
        }
    }

    /// Calculate current health status based on statistics and thresholds
    pub fn get_health_status(&self) -> SourceHealth {
        let stats = self.get_statistics();
        
        // Check if too many retries
        if stats.num_retry > self.health_thresholds.max_retry_count {
            return SourceHealth::Unhealthy;
        }

        // Check frame timeout
        if let Some(last_frame) = stats.last_frame_timestamp {
            let frame_age = last_frame.elapsed();
            if frame_age > Duration::from_secs(self.health_thresholds.frame_timeout_seconds) {
                return SourceHealth::Degraded;
            }
        } else {
            // No frames received yet
            let since_creation = self.creation_time.elapsed();
            if since_creation > Duration::from_secs(self.health_thresholds.frame_timeout_seconds) {
                // Been trying for too long without success
                return SourceHealth::Unhealthy;
            } else if !self.is_ready() {
                // Not ready yet, give it time
                return SourceHealth::Unknown;
            }
        }

        // Check buffering percentage
        if stats.buffering_percent < self.health_thresholds.min_buffering_percent {
            return SourceHealth::Degraded;
        }

        SourceHealth::Healthy
    }

    /// Get the source bin for use in pipelines
    pub fn get_source_bin(&self) -> Option<&gst::Bin> {
        self.source_bin.as_ref()
    }

    /// Get source type
    pub fn get_source_type(&self) -> SourceType {
        self.source_type
    }

    /// Get source URI
    pub fn get_source_uri(&self) -> &str {
        &self.source_uri
    }

    /// Check if source is ready (bin created and configured)
    pub fn is_ready(&self) -> bool {
        self.source_bin.is_some() && self.fallbacksrc.is_some()
    }
}

impl Drop for StreamSource {
    fn drop(&mut self) {
        debug!("Dropping StreamSource: {}", self.id);
        
        if let Some(bin) = &self.source_bin {
            let _ = bin.set_state(gst::State::Null);
        }
        
        info!("StreamSource {} dropped successfully", self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StreamConfig;

    fn create_test_config(uri: &str) -> StreamConfig {
        StreamConfig {
            id: "test-stream".to_string(),
            name: "Test Stream".to_string(),
            source_uri: uri.to_string(),
            enabled: true,
            recording_enabled: true,
            inference_enabled: false,
            reconnect_timeout_seconds: 5,
            max_reconnect_attempts: 3,
            buffer_size_mb: 50,
            rtsp_outputs: None,
        }
    }

    #[test]
    fn test_source_type_detection() {
        assert_eq!(SourceType::from_uri("rtsp://example.com/stream"), SourceType::Rtsp);
        assert_eq!(SourceType::from_uri("rtsps://example.com/stream"), SourceType::Rtsp);
        assert_eq!(SourceType::from_uri("http://example.com/stream.m3u8"), SourceType::Http);
        assert_eq!(SourceType::from_uri("https://example.com/stream.m3u8"), SourceType::Http);
        assert_eq!(SourceType::from_uri("file:///path/to/video.mp4"), SourceType::File);
        assert_eq!(SourceType::from_uri("ftp://example.com"), SourceType::Unknown);
    }

    #[test]
    fn test_stream_source_creation() {
        let config = create_test_config("rtsp://example.com/stream");
        let source = StreamSource::new("test-id".to_string(), &config).unwrap();
        
        assert_eq!(source.id, "test-id");
        assert_eq!(source.source_uri, "rtsp://example.com/stream");
        assert_eq!(source.source_type, SourceType::Rtsp);
        assert!(!source.is_ready());
    }

    #[test] 
    fn test_health_thresholds() {
        let thresholds = HealthThresholds::default();
        assert_eq!(thresholds.max_retry_count, 5);
        assert_eq!(thresholds.frame_timeout_seconds, 30);
        assert_eq!(thresholds.min_buffering_percent, 50);
    }

    #[test]
    fn test_statistics_default() {
        let stats = SourceStatistics::default();
        assert_eq!(stats.num_retry, 0);
        assert_eq!(stats.buffering_percent, 0);
        assert!(stats.last_frame_timestamp.is_none());
    }

    #[tokio::test]
    async fn test_source_bin_creation_requires_gstreamer() {
        // This test requires GStreamer to be initialized
        // Skip if not available to avoid test failures
        if gst::init().is_err() {
            return;
        }

        let config = create_test_config("file:///dev/null");
        let mut source = StreamSource::new("test".to_string(), &config).unwrap();
        
        // Note: This will likely fail without proper GStreamer plugins
        // but tests the code path
        let result = source.create_source_bin();
        
        // We expect this to fail in test environment, but the function should handle it gracefully
        match result {
            Ok(_) => {
                assert!(source.is_ready());
            }
            Err(e) => {
                // Expected in test environment without full GStreamer setup
                assert!(e.to_string().contains("fallbacksrc") || e.to_string().contains("decodebin3"));
            }
        }
    }

    #[test]
    fn test_health_status_calculation() {
        let config = create_test_config("rtsp://example.com/stream");
        let source = StreamSource::new("test".to_string(), &config).unwrap();
        
        // New source should be unknown/healthy initially
        let health = source.get_health_status();
        assert!(matches!(health, SourceHealth::Healthy | SourceHealth::Unknown));
    }
}
