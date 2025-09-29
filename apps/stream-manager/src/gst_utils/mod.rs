use gst::prelude::*;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::{Result, StreamManagerError};

/// Required GStreamer elements that must be present
const REQUIRED_ELEMENTS: &[&str] = &[
    // Core elements
    "queue",
    "tee",
    "filesink",
    "fakesink",
    
    // From gst-plugins-rs
    "fallbacksrc",
    "togglerecord",
    "interaudiosink",
    "interaudiosrc",
    "intersubsink",
    "intersubsrc",
    "intervideosink",
    "intervideosrc",
    
    // Video elements
    "videoconvert",
    "videoscale",
    "x264enc",
    
    // Muxers
    "splitmuxsink",
    "mp4mux",
    
    // RTP/RTSP elements
    "rtspsrc",
    "rtph264pay",
    "rtph264depay",
];

/// Optional elements that enhance functionality
const OPTIONAL_ELEMENTS: &[&str] = &[
    // NVIDIA elements
    "nvh264enc",
    "nvh265enc",
    "nvvideoconvert",
    "nvinfer",
    "nvstreammux",
    "nvstreamdemux",
    "nvdsosd",
    
    // Additional codecs
    "x265enc",
    "vp8enc",
    "vp9enc",
    
    // WebRTC elements
    "webrtcbin",
    "webrtcsink",
    "webrtcsrc",
];

/// Represents the capabilities detected in the GStreamer installation
#[derive(Debug, Clone)]
pub struct GstCapabilities {
    pub has_nvidia: bool,
    pub has_webrtc: bool,
    pub has_h265: bool,
    pub has_vp8: bool,
    pub has_vp9: bool,
    pub available_elements: HashMap<String, bool>,
}

impl GstCapabilities {
    /// Check if a specific element is available
    pub fn has_element(&self, element: &str) -> bool {
        self.available_elements.get(element).copied().unwrap_or(false)
    }
}

/// Initialize GStreamer with error handling and logging
pub fn initialize() -> Result<GstCapabilities> {
    info!("Initializing GStreamer");
    
    // Initialize GStreamer
    gst::init().map_err(|e| {
        error!("Failed to initialize GStreamer: {}", e);
        StreamManagerError::ConfigError(format!("GStreamer initialization failed: {}", e))
    })?;
    
    // Log GStreamer version
    let (major, minor, micro, nano) = gst::version();
    info!(
        "GStreamer version: {}.{}.{}.{}",
        major, minor, micro, nano
    );
    
    // Set debug level from environment if present
    if let Ok(debug_str) = std::env::var("GST_DEBUG") {
        debug!("GST_DEBUG set to: {}", debug_str);
    }
    
    // Check for custom plugin path
    if let Ok(plugin_path) = std::env::var("GST_PLUGIN_PATH") {
        info!("Using custom GST_PLUGIN_PATH: {}", plugin_path);
    }
    
    // Discover available plugins
    let capabilities = discover_plugins()?;
    
    // Log capabilities summary
    info!("GStreamer capabilities detected:");
    info!("  NVIDIA support: {}", capabilities.has_nvidia);
    info!("  WebRTC support: {}", capabilities.has_webrtc);
    info!("  H.265 support: {}", capabilities.has_h265);
    info!("  VP8 support: {}", capabilities.has_vp8);
    info!("  VP9 support: {}", capabilities.has_vp9);
    
    Ok(capabilities)
}

/// Discover available GStreamer plugins and build capability map
pub fn discover_plugins() -> Result<GstCapabilities> {
    let mut available_elements = HashMap::new();
    
    // Check required elements
    info!("Checking required GStreamer elements...");
    let mut missing_required = Vec::new();
    
    for element in REQUIRED_ELEMENTS {
        if let Some(factory) = gst::ElementFactory::find(element) {
            debug!("Found required element: {} (rank: {:?})", element, factory.rank());
            available_elements.insert(element.to_string(), true);
        } else {
            error!("Missing required element: {}", element);
            missing_required.push(*element);
            available_elements.insert(element.to_string(), false);
        }
    }
    
    // Fail if any required elements are missing
    if !missing_required.is_empty() {
        return Err(StreamManagerError::ConfigError(format!(
            "Missing required GStreamer elements: {:?}",
            missing_required
        )));
    }
    
    info!("All required elements found");
    
    // Check optional elements
    info!("Checking optional GStreamer elements...");
    let mut has_nvidia = false;
    let mut has_webrtc = false;
    let mut has_h265 = false;
    let mut has_vp8 = false;
    let mut has_vp9 = false;
    
    for element in OPTIONAL_ELEMENTS {
        if let Some(factory) = gst::ElementFactory::find(element) {
            debug!("Found optional element: {} (rank: {:?})", element, factory.rank());
            available_elements.insert(element.to_string(), true);
            
            // Update capability flags
            match *element {
                "nvh264enc" | "nvvideoconvert" | "nvinfer" => has_nvidia = true,
                "webrtcbin" | "webrtcsink" | "webrtcsrc" => has_webrtc = true,
                "x265enc" | "nvh265enc" => has_h265 = true,
                "vp8enc" => has_vp8 = true,
                "vp9enc" => has_vp9 = true,
                _ => {}
            }
        } else {
            debug!("Optional element not found: {}", element);
            available_elements.insert(element.to_string(), false);
        }
    }
    
    // Additional NVIDIA detection - check if CUDA is available
    if has_nvidia {
        if let Ok(cuda_path) = std::env::var("CUDA_PATH") {
            info!("CUDA detected at: {}", cuda_path);
        } else {
            warn!("NVIDIA elements found but CUDA_PATH not set");
        }
    }
    
    Ok(GstCapabilities {
        has_nvidia,
        has_webrtc,
        has_h265,
        has_vp8,
        has_vp9,
        available_elements,
    })
}

/// Verify plugin availability for a specific use case
pub fn verify_plugins_for_feature(capabilities: &GstCapabilities, feature: &str) -> Result<()> {
    match feature {
        "recording" => {
            // Recording requires basic elements (already checked in required)
            Ok(())
        }
        "nvidia-inference" => {
            if !capabilities.has_nvidia {
                return Err(StreamManagerError::ConfigError(
                    "NVIDIA inference requested but NVIDIA elements not available".to_string()
                ));
            }
            Ok(())
        }
        "webrtc" => {
            if !capabilities.has_webrtc {
                return Err(StreamManagerError::ConfigError(
                    "WebRTC streaming requested but WebRTC elements not available".to_string()
                ));
            }
            Ok(())
        }
        _ => Ok(())
    }
}

/// Print detailed plugin information (for --check-plugins flag)
pub fn print_plugin_info(capabilities: &GstCapabilities) {
    println!("\nGStreamer Plugin Discovery Report");
    println!("==================================");
    
    // GStreamer version
    let (major, minor, micro, nano) = gst::version();
    println!("\nGStreamer Version: {}.{}.{}.{}", major, minor, micro, nano);
    
    // Plugin paths
    if let Ok(plugin_path) = std::env::var("GST_PLUGIN_PATH") {
        println!("Custom Plugin Path: {}", plugin_path);
    }
    
    // Required elements
    println!("\nRequired Elements:");
    for element in REQUIRED_ELEMENTS {
        let status = if capabilities.has_element(element) {
            "✓"
        } else {
            "✗"
        };
        println!("  {} {}", status, element);
    }
    
    // Optional elements
    println!("\nOptional Elements:");
    for element in OPTIONAL_ELEMENTS {
        let status = if capabilities.has_element(element) {
            "✓"
        } else {
            "✗"
        };
        println!("  {} {}", status, element);
    }
    
    // Capabilities summary
    println!("\nCapabilities Summary:");
    println!("  NVIDIA Support: {}", if capabilities.has_nvidia { "Yes" } else { "No" });
    println!("  WebRTC Support: {}", if capabilities.has_webrtc { "Yes" } else { "No" });
    println!("  H.265 Encoding: {}", if capabilities.has_h265 { "Yes" } else { "No" });
    println!("  VP8 Encoding: {}", if capabilities.has_vp8 { "Yes" } else { "No" });
    println!("  VP9 Encoding: {}", if capabilities.has_vp9 { "Yes" } else { "No" });
    
    // Registry stats
    let registry = gst::Registry::get();
    let plugins = registry.plugins();
    println!("\nRegistry Statistics:");
    println!("  Total Plugins: {}", plugins.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gstreamer_initialization() {
        // This test verifies GStreamer can be initialized
        let result = initialize();
        assert!(result.is_ok(), "GStreamer initialization failed: {:?}", result);
        
        let capabilities = result.unwrap();
        assert!(!capabilities.available_elements.is_empty());
    }
    
    #[test]
    fn test_required_elements_present() {
        // Initialize and check that all required elements are available
        let capabilities = initialize().expect("Failed to initialize GStreamer");
        
        for element in REQUIRED_ELEMENTS {
            assert!(
                capabilities.has_element(element),
                "Required element '{}' not found",
                element
            );
        }
    }
    
    #[test]
    fn test_capability_detection() {
        // Test that capability detection works correctly
        // First initialize GStreamer
        gst::init().ok();
        let capabilities = discover_plugins().expect("Failed to discover plugins");
        
        // Check that boolean flags match element availability
        if capabilities.has_element("nvh264enc") {
            assert!(capabilities.has_nvidia);
        }
        
        if capabilities.has_element("webrtcbin") {
            assert!(capabilities.has_webrtc);
        }
    }
    
    #[test]
    fn test_feature_verification() {
        let capabilities = initialize().expect("Failed to initialize GStreamer");
        
        // Recording should always work with required elements
        assert!(verify_plugins_for_feature(&capabilities, "recording").is_ok());
        
        // NVIDIA inference should fail if no NVIDIA elements
        if !capabilities.has_nvidia {
            assert!(verify_plugins_for_feature(&capabilities, "nvidia-inference").is_err());
        }
        
        // WebRTC should fail if no WebRTC elements
        if !capabilities.has_webrtc {
            assert!(verify_plugins_for_feature(&capabilities, "webrtc").is_err());
        }
    }
}
