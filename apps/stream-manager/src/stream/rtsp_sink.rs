use gst::prelude::*;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::config::RtspSinkConfig;
use crate::stream::branching::{BranchManager, StreamBranch};

#[derive(Debug, thiserror::Error)]
pub enum RtspSinkError {
    #[error("Failed to create element: {0}")]
    ElementCreation(String),
    #[error("Failed to link elements")]
    LinkError,
    #[error("Pipeline error: {0}")]
    PipelineError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Builder for RTSP sink pipelines
pub struct RtspSinkBuilder {
    config: RtspSinkConfig,
    pipeline: gst::Pipeline,
    branch_manager: Arc<BranchManager>,
}

impl RtspSinkBuilder {
    pub fn new(
        config: RtspSinkConfig,
        pipeline: gst::Pipeline,
        branch_manager: Arc<BranchManager>,
    ) -> Self {
        Self {
            config,
            pipeline,
            branch_manager,
        }
    }
    
    /// Build RTSP sink branch
    pub fn build(&self) -> Result<gst::Element, RtspSinkError> {
        info!("Building RTSP sink for location: {}", self.config.location);
        
        // Get or create RTSP branch queue
        let queue = self.branch_manager
            .create_branch(StreamBranch::Rtsp)
            .map_err(|e| RtspSinkError::PipelineError(format!("Failed to create RTSP branch: {}", e)))?;
        
        // Create video converter for format compatibility
        let videoconvert = gst::ElementFactory::make("videoconvert")
            .name("rtsp-videoconvert")
            .build()
            .map_err(|_| RtspSinkError::ElementCreation("videoconvert".to_string()))?;
        
        // Create video scaler if resolution is specified
        let videoscale = if self.config.width.is_some() || self.config.height.is_some() {
            Some(gst::ElementFactory::make("videoscale")
                .name("rtsp-videoscale")
                .build()
                .map_err(|_| RtspSinkError::ElementCreation("videoscale".to_string()))?)
        } else {
            None
        };
        
        // Create caps filter if resolution is specified
        let capsfilter = if let (Some(width), Some(height)) = (self.config.width, self.config.height) {
            let caps = gst::Caps::builder("video/x-raw")
                .field("width", width as i32)
                .field("height", height as i32)
                .build();
            
            Some(gst::ElementFactory::make("capsfilter")
                .name("rtsp-capsfilter")
                .property("caps", &caps)
                .build()
                .map_err(|_| RtspSinkError::ElementCreation("capsfilter".to_string()))?)
        } else {
            None
        };
        
        // Create encoder based on configuration
        let encoder = self.create_encoder()?;
        
        // Create RTP payloader based on codec
        let payloader = self.create_payloader()?;
        
        // Create RTSP client sink
        let rtspsink = gst::ElementFactory::make("rtspclientsink")
            .name("rtsp-sink")
            .property("location", &self.config.location)
            .property("latency", self.config.latency_ms)
            .property_from_str("protocols", self.config.protocols.as_str())
            .build()
            .map_err(|_| RtspSinkError::ElementCreation("rtspclientsink".to_string()))?;
        
        // Set authentication if configured
        if let (Some(user), Some(pass)) = (&self.config.username, &self.config.password) {
            rtspsink.set_property("user-id", user);
            rtspsink.set_property("user-pw", pass);
        }
        
        // Add all elements to pipeline
        self.pipeline.add(&videoconvert)
            .map_err(|_| RtspSinkError::PipelineError("Failed to add videoconvert".to_string()))?;
        
        if let Some(ref scale) = videoscale {
            self.pipeline.add(scale)
                .map_err(|_| RtspSinkError::PipelineError("Failed to add videoscale".to_string()))?;
        }
        
        if let Some(ref caps) = capsfilter {
            self.pipeline.add(caps)
                .map_err(|_| RtspSinkError::PipelineError("Failed to add capsfilter".to_string()))?;
        }
        
        self.pipeline.add(&encoder)
            .map_err(|_| RtspSinkError::PipelineError("Failed to add encoder".to_string()))?;
        
        self.pipeline.add(&payloader)
            .map_err(|_| RtspSinkError::PipelineError("Failed to add payloader".to_string()))?;
        
        self.pipeline.add(&rtspsink)
            .map_err(|_| RtspSinkError::PipelineError("Failed to add rtspsink".to_string()))?;
        
        // Link elements
        self.link_elements(&queue, &videoconvert, videoscale.as_ref(), capsfilter.as_ref(), 
                          &encoder, &payloader, &rtspsink)?;
        
        // Sync state with parent
        videoconvert.sync_state_with_parent()
            .map_err(|_| RtspSinkError::PipelineError("Failed to sync videoconvert state".to_string()))?;
        
        if let Some(ref scale) = videoscale {
            scale.sync_state_with_parent()
                .map_err(|_| RtspSinkError::PipelineError("Failed to sync videoscale state".to_string()))?;
        }
        
        if let Some(ref caps) = capsfilter {
            caps.sync_state_with_parent()
                .map_err(|_| RtspSinkError::PipelineError("Failed to sync capsfilter state".to_string()))?;
        }
        
        encoder.sync_state_with_parent()
            .map_err(|_| RtspSinkError::PipelineError("Failed to sync encoder state".to_string()))?;
        
        payloader.sync_state_with_parent()
            .map_err(|_| RtspSinkError::PipelineError("Failed to sync payloader state".to_string()))?;
        
        rtspsink.sync_state_with_parent()
            .map_err(|_| RtspSinkError::PipelineError("Failed to sync rtspsink state".to_string()))?;
        
        info!("RTSP sink branch created successfully");
        Ok(rtspsink)
    }
    
    /// Create encoder based on codec configuration
    fn create_encoder(&self) -> Result<gst::Element, RtspSinkError> {
        let encoder = match self.config.codec.as_str() {
            "h264" => {
                let enc = gst::ElementFactory::make("x264enc")
                    .name("rtsp-encoder")
                    .property_from_str("tune", "zerolatency")
                    .property_from_str("speed-preset", "ultrafast")
                    .property("key-int-max", 30u32)
                    .build()
                    .map_err(|_| RtspSinkError::ElementCreation("x264enc".to_string()))?;
                
                if let Some(bitrate) = self.config.bitrate_kbps {
                    enc.set_property("bitrate", bitrate);
                }
                
                enc
            }
            "h265" => {
                let enc = gst::ElementFactory::make("x265enc")
                    .name("rtsp-encoder")
                    .property_from_str("tune", "zerolatency")
                    .property_from_str("speed-preset", "ultrafast")
                    .property("key-int-max", 30u32)
                    .build()
                    .map_err(|_| RtspSinkError::ElementCreation("x265enc".to_string()))?;
                
                if let Some(bitrate) = self.config.bitrate_kbps {
                    enc.set_property("bitrate", bitrate);
                }
                
                enc
            }
            "vp8" => {
                gst::ElementFactory::make("vp8enc")
                    .name("rtsp-encoder")
                    .property("deadline", 1i64) // Realtime encoding
                    .property("cpu-used", 5i32) // Fastest preset
                    .property("keyframe-max-dist", 30i32)
                    .property_from_str("target-bitrate", &format!("{}000", self.config.bitrate_kbps.unwrap_or(2000)))
                    .build()
                    .map_err(|_| RtspSinkError::ElementCreation("vp8enc".to_string()))?
            }
            "vp9" => {
                gst::ElementFactory::make("vp9enc")
                    .name("rtsp-encoder")
                    .property("deadline", 1i64) // Realtime encoding
                    .property("cpu-used", 5i32) // Fastest preset
                    .property("keyframe-max-dist", 30i32)
                    .property_from_str("target-bitrate", &format!("{}000", self.config.bitrate_kbps.unwrap_or(2000)))
                    .build()
                    .map_err(|_| RtspSinkError::ElementCreation("vp9enc".to_string()))?
            }
            codec => {
                return Err(RtspSinkError::ConfigError(format!("Unsupported codec: {}", codec)));
            }
        };
        
        Ok(encoder)
    }
    
    /// Create RTP payloader based on codec
    fn create_payloader(&self) -> Result<gst::Element, RtspSinkError> {
        let payloader = match self.config.codec.as_str() {
            "h264" => {
                gst::ElementFactory::make("rtph264pay")
                    .name("rtsp-payloader")
                    .property("config-interval", -1i32)
                    .property("pt", 96u32)
                    .build()
                    .map_err(|_| RtspSinkError::ElementCreation("rtph264pay".to_string()))?
            }
            "h265" => {
                gst::ElementFactory::make("rtph265pay")
                    .name("rtsp-payloader")
                    .property("config-interval", -1i32)
                    .property("pt", 96u32)
                    .build()
                    .map_err(|_| RtspSinkError::ElementCreation("rtph265pay".to_string()))?
            }
            "vp8" => {
                gst::ElementFactory::make("rtpvp8pay")
                    .name("rtsp-payloader")
                    .property("pt", 96u32)
                    .build()
                    .map_err(|_| RtspSinkError::ElementCreation("rtpvp8pay".to_string()))?
            }
            "vp9" => {
                gst::ElementFactory::make("rtpvp9pay")
                    .name("rtsp-payloader")
                    .property("pt", 96u32)
                    .build()
                    .map_err(|_| RtspSinkError::ElementCreation("rtpvp9pay".to_string()))?
            }
            codec => {
                return Err(RtspSinkError::ConfigError(format!("Unsupported codec for RTP: {}", codec)));
            }
        };
        
        Ok(payloader)
    }
    
    /// Link all elements in the RTSP sink branch
    fn link_elements(
        &self,
        queue: &gst::Element,
        videoconvert: &gst::Element,
        videoscale: Option<&gst::Element>,
        capsfilter: Option<&gst::Element>,
        encoder: &gst::Element,
        payloader: &gst::Element,
        rtspsink: &gst::Element,
    ) -> Result<(), RtspSinkError> {
        // Build the linking chain based on optional elements
        let mut prev_element = queue;
        
        // Link queue to videoconvert
        prev_element.link(videoconvert)
            .map_err(|_| RtspSinkError::LinkError)?;
        prev_element = videoconvert;
        
        // Link videoscale if present
        if let Some(scale) = videoscale {
            prev_element.link(scale)
                .map_err(|_| RtspSinkError::LinkError)?;
            prev_element = scale;
        }
        
        // Link capsfilter if present
        if let Some(caps) = capsfilter {
            prev_element.link(caps)
                .map_err(|_| RtspSinkError::LinkError)?;
            prev_element = caps;
        }
        
        // Link to encoder
        prev_element.link(encoder)
            .map_err(|_| RtspSinkError::LinkError)?;
        
        // Link encoder to payloader
        encoder.link(payloader)
            .map_err(|_| RtspSinkError::LinkError)?;
        
        // Link payloader to RTSP sink
        payloader.link(rtspsink)
            .map_err(|_| RtspSinkError::LinkError)?;
        
        debug!("RTSP sink elements linked successfully");
        Ok(())
    }
}

/// Manager for RTSP sinks in a stream
pub struct RtspSinkManager {
    sinks: Vec<gst::Element>,
    branch_manager: Arc<BranchManager>,
}

impl RtspSinkManager {
    pub fn new(branch_manager: Arc<BranchManager>) -> Self {
        Self {
            sinks: Vec::new(),
            branch_manager,
        }
    }
    
    /// Add a new RTSP sink
    pub fn add_sink(&mut self, config: RtspSinkConfig, pipeline: &gst::Pipeline) -> Result<(), RtspSinkError> {
        let builder = RtspSinkBuilder::new(config, pipeline.clone(), self.branch_manager.clone());
        let sink = builder.build()?;
        self.sinks.push(sink);
        Ok(())
    }
    
    /// Remove all RTSP sinks
    pub fn remove_all(&mut self) -> Result<(), RtspSinkError> {
        for sink in self.sinks.drain(..) {
            sink.set_state(gst::State::Null)
                .map_err(|_| RtspSinkError::PipelineError("Failed to stop RTSP sink".to_string()))?;
        }
        
        // Remove the RTSP branch
        self.branch_manager.remove_branch(&StreamBranch::Rtsp)
            .map_err(|e| RtspSinkError::PipelineError(format!("Failed to remove RTSP branch: {}", e)))?;
        
        Ok(())
    }
    
    /// Get the number of active RTSP sinks
    pub fn sink_count(&self) -> usize {
        self.sinks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rtsp_sink_builder_creation() {
        gst::init().unwrap();
        
        let pipeline = gst::Pipeline::new();
        let branch_manager = Arc::new(BranchManager::new(&pipeline).unwrap());
        
        let config = RtspSinkConfig {
            enabled: true,
            location: "rtsp://localhost:8554/test".to_string(),
            codec: "h264".to_string(),
            bitrate_kbps: Some(2000),
            width: Some(1920),
            height: Some(1080),
            latency_ms: 100,
            protocols: "tcp".to_string(),
            username: None,
            password: None,
        };
        
        let builder = RtspSinkBuilder::new(config, pipeline.clone(), branch_manager);
        
        // Just verify the builder can be created
        // Actual build would fail without proper setup
    }
    
    #[test]
    fn test_rtsp_sink_manager() {
        gst::init().unwrap();
        
        let pipeline = gst::Pipeline::new();
        let branch_manager = Arc::new(BranchManager::new(&pipeline).unwrap());
        
        let manager = RtspSinkManager::new(branch_manager);
        assert_eq!(manager.sink_count(), 0);
    }
}