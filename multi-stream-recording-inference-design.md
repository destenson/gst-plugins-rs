# Unified Multi-Stream Recording and Inference System - Technical Design

**Date:** 2025-09-08  
**Subject:** Consolidating MediaMTX, Python DeepStream, and control app into single Rust application  
**Repository:** gst-plugins-rs

## Executive Summary

This design document outlines the architecture for consolidating three separate applications (MediaMTX proxy/recorder, Python DeepStream inference, and stream control/monitoring) into a single robust Rust application using GStreamer and gst-plugins-rs components.

## Current Architecture Problems

### Three Separate Applications:
1. **MediaMTX** - RTSP proxy and recording server
2. **Python DeepStream** - Inference application with delays
3. **Control App** - Stream management and health monitoring

### Issues:
- Complex inter-process communication
- Multiple points of failure
- Resource duplication (multiple decoders for same stream)
- Latency from proxying
- Difficult deployment and maintenance
- Inconsistent error handling

## Proposed Unified Architecture

### Single Rust Application with:
- Dynamic pipeline management
- Integrated recording with smart buffering
- Native inference support (NVIDIA and CPU)
- Stream health monitoring
- REST/WebSocket API for control
- Robust error handling and recovery

## Core Components from gst-plugins-rs

### 1. Stream Input Management
```rust
// For each stream source
struct StreamSource {
    id: String,
    uri: String,
    fallback_src: gst::Element,  // From utils/fallbackswitch
    tee: gst::Element,           // Split for recording + inference
    health_monitor: StreamHealth,
}
```

**Using `fallbacksrc`** for robust stream handling:
- Automatic reconnection for intermittent streams
- Configurable timeouts for radio links
- Built-in retry statistics

### 2. Dynamic Pipeline Architecture

**Using `intersink/intersrc`** from `generic/inter`:
```rust
// Producer pipeline for each source
let producer_pipeline = format!(
    "fallbacksrc name=source \
        uri={uri} \
        timeout=5000000000 \
        retry-timeout=60000000000 \
     ! tee name=t \
     ! queue ! intersink name=sink_{id}"
);

// Consumer pipelines can connect dynamically
let recording_pipeline = format!(
    "intersrc name=src_{id} \
     ! queue \
     ! splitmuxsink \
        location=recordings/{id}_%05d.mp4 \
        max-size-time=600000000000"
);

let inference_pipeline = format!(
    "intersrc name=src_{id} \
     ! queue \
     ! nvvideoconvert \
     ! nvinfer config-file-path=config.txt \
     ! nvvideoconvert \
     ! appsink name=inference_sink"
);
```

### 3. Recording Management

**Using `togglerecord`** from `utils/togglerecord`:
```rust
struct RecordingManager {
    streams: HashMap<String, RecordingStream>,
}

struct RecordingStream {
    id: String,
    toggle_record: gst::Element,
    splitmuxsink: gst::Element,
    is_recording: bool,
    segment_duration: Duration,
}

impl RecordingManager {
    fn start_recording(&mut self, stream_id: &str) {
        if let Some(stream) = self.streams.get_mut(stream_id) {
            stream.toggle_record.set_property("record", true);
            stream.is_recording = true;
        }
    }
    
    fn stop_recording(&mut self, stream_id: &str) {
        // Ensures clean cut points
        if let Some(stream) = self.streams.get_mut(stream_id) {
            stream.toggle_record.set_property("record", false);
            stream.is_recording = false;
        }
    }
}
```

### 4. Stream Health Monitoring

```rust
struct StreamHealth {
    last_frame_time: Instant,
    retry_count: u64,
    buffering_percent: i32,
    is_healthy: bool,
    removal_threshold: Duration,
}

impl StreamHealth {
    fn update_from_stats(&mut self, stats: &gst::Structure) {
        self.retry_count = stats.get("num-retry").unwrap_or(0);
        self.buffering_percent = stats.get("buffering-percent").unwrap_or(100);
        
        // Auto-remove logic
        if self.last_frame_time.elapsed() > self.removal_threshold {
            self.is_healthy = false;
        }
    }
}
```

## Complete Application Architecture

```rust
use gst::prelude::*;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

struct UnifiedStreamManager {
    // Core components
    streams: Arc<RwLock<HashMap<String, ManagedStream>>>,
    pipeline: gst::Pipeline,
    
    // Managers
    recording_manager: Arc<RwLock<RecordingManager>>,
    inference_manager: Arc<RwLock<InferenceManager>>,
    
    // Configuration
    config: AppConfig,
}

struct ManagedStream {
    id: String,
    source_bin: gst::Bin,        // Contains fallbacksrc
    
    // Branches
    recording_branch: Option<gst::Bin>,
    inference_branch: Option<gst::Bin>,
    preview_branch: Option<gst::Bin>,
    
    // Monitoring
    health: StreamHealth,
    statistics: StreamStatistics,
}

struct AppConfig {
    max_retry_duration: Duration,
    segment_duration: Duration,
    inference_config: InferenceConfig,
    storage_path: PathBuf,
    auto_remove_unhealthy: bool,
}

impl UnifiedStreamManager {
    async fn add_stream(&self, id: String, uri: String, enable_inference: bool) -> Result<()> {
        // Create source with fallback handling
        let source_bin = self.create_source_bin(&id, &uri)?;
        
        // Add tee for multiple outputs
        let tee = gst::ElementFactory::make("tee").build()?;
        source_bin.add(&tee)?;
        
        // Create recording branch (always present but controlled by togglerecord)
        let recording_branch = self.create_recording_branch(&id)?;
        
        // Optionally create inference branch
        let inference_branch = if enable_inference {
            Some(self.create_inference_branch(&id)?)
        } else {
            None
        };
        
        // Add to pipeline
        self.pipeline.add(&source_bin)?;
        
        // Store in managed streams
        let stream = ManagedStream {
            id: id.clone(),
            source_bin,
            recording_branch: Some(recording_branch),
            inference_branch,
            preview_branch: None,
            health: StreamHealth::default(),
            statistics: StreamStatistics::default(),
        };
        
        self.streams.write().await.insert(id, stream);
        Ok(())
    }
    
    async fn remove_stream(&self, id: &str) -> Result<()> {
        if let Some(stream) = self.streams.write().await.remove(id) {
            // Gracefully stop recording
            self.recording_manager.write().await.stop_recording(id);
            
            // Remove from pipeline
            stream.source_bin.set_state(gst::State::Null)?;
            self.pipeline.remove(&stream.source_bin)?;
        }
        Ok(())
    }
    
    fn create_source_bin(&self, id: &str, uri: &str) -> Result<gst::Bin> {
        let bin = gst::Bin::new();
        
        // Create robust source with fallback
        let source = gst::ElementFactory::make("fallbacksrc")
            .property("uri", uri)
            .property("timeout", 5u64 * gst::ClockTime::SECOND)
            .property("retry-timeout", 60u64 * gst::ClockTime::SECOND)
            .property("restart-timeout", 3u64 * gst::ClockTime::SECOND)
            .name(&format!("source_{}", id))
            .build()?;
        
        // Add decoding
        let decodebin = gst::ElementFactory::make("decodebin3")
            .name(&format!("decode_{}", id))
            .build()?;
        
        bin.add_many([&source, &decodebin])?;
        source.link(&decodebin)?;
        
        // Create ghost pads for output
        decodebin.connect_pad_added(move |_dbin, pad| {
            // Ghost pad creation logic
        });
        
        Ok(bin)
    }
    
    fn create_recording_branch(&self, id: &str) -> Result<gst::Bin> {
        let bin = gst::Bin::new();
        
        // Queue for buffering
        let queue = gst::ElementFactory::make("queue")
            .property("max-size-time", 10u64 * gst::ClockTime::SECOND)
            .build()?;
        
        // Toggle record for clean start/stop
        let toggle = gst::ElementFactory::make("togglerecord")
            .property("record", false)
            .name(&format!("toggle_{}", id))
            .build()?;
        
        // Splitmuxsink for segmented recording
        let sink = gst::ElementFactory::make("splitmuxsink")
            .property("location", format!("recordings/{}/segment_%05d.mp4", id))
            .property("max-size-time", 10u64 * gst::ClockTime::MINUTE)
            .property("send-keyframe-requests", true)
            .build()?;
        
        bin.add_many([&queue, &toggle, &sink])?;
        gst::Element::link_many([&queue, &toggle, &sink])?;
        
        Ok(bin)
    }
    
    fn create_inference_branch(&self, id: &str) -> Result<gst::Bin> {
        let bin = gst::Bin::new();
        
        // Branch for inference
        let queue = gst::ElementFactory::make("queue").build()?;
        
        // Use inter elements for decoupling
        let intersink = gst::ElementFactory::make("intersink")
            .property("name", format!("inference_sink_{}", id))
            .build()?;
        
        bin.add_many([&queue, &intersink])?;
        queue.link(&intersink)?;
        
        // Spawn separate inference pipeline
        self.spawn_inference_pipeline(id)?;
        
        Ok(bin)
    }
    
    fn spawn_inference_pipeline(&self, id: &str) -> Result<()> {
        // Create separate pipeline for inference to isolate potential issues
        let pipeline = gst::Pipeline::new();
        
        let src = gst::ElementFactory::make("intersrc")
            .property("name", format!("inference_src_{}", id))
            .build()?;
        
        // Add inference elements based on config
        let inference = if self.config.inference_config.use_nvidia {
            self.create_nvidia_inference()?
        } else {
            self.create_cpu_inference()?
        };
        
        pipeline.add_many([&src, &inference])?;
        src.link(&inference)?;
        
        pipeline.set_state(gst::State::Playing)?;
        Ok(())
    }
}

// REST API
mod api {
    use actix_web::{web, HttpResponse, Result};
    use super::*;
    
    pub async fn add_stream(
        manager: web::Data<Arc<UnifiedStreamManager>>,
        info: web::Json<AddStreamRequest>,
    ) -> Result<HttpResponse> {
        manager.add_stream(
            info.id.clone(),
            info.uri.clone(),
            info.enable_inference,
        ).await?;
        Ok(HttpResponse::Ok().json(&StreamAddedResponse { id: info.id }))
    }
    
    pub async fn remove_stream(
        manager: web::Data<Arc<UnifiedStreamManager>>,
        path: web::Path<String>,
    ) -> Result<HttpResponse> {
        manager.remove_stream(&path.into_inner()).await?;
        Ok(HttpResponse::Ok().finish())
    }
    
    pub async fn get_stream_health(
        manager: web::Data<Arc<UnifiedStreamManager>>,
    ) -> Result<HttpResponse> {
        let streams = manager.streams.read().await;
        let health: Vec<_> = streams.values()
            .map(|s| StreamHealthInfo {
                id: s.id.clone(),
                is_healthy: s.health.is_healthy,
                retry_count: s.health.retry_count,
                buffering: s.health.buffering_percent,
            })
            .collect();
        Ok(HttpResponse::Ok().json(&health))
    }
    
    pub async fn start_recording(
        manager: web::Data<Arc<UnifiedStreamManager>>,
        path: web::Path<String>,
    ) -> Result<HttpResponse> {
        manager.recording_manager.write().await
            .start_recording(&path.into_inner());
        Ok(HttpResponse::Ok().finish())
    }
}

// Main application
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize GStreamer
    gst::init()?;
    
    // Create manager
    let manager = Arc::new(UnifiedStreamManager::new()?);
    
    // Start monitoring task
    let monitor_manager = manager.clone();
    tokio::spawn(async move {
        monitor_streams(monitor_manager).await;
    });
    
    // Start REST API
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(manager.clone()))
            .route("/streams", web::post().to(api::add_stream))
            .route("/streams/{id}", web::delete().to(api::remove_stream))
            .route("/streams/{id}/record", web::post().to(api::start_recording))
            .route("/health", web::get().to(api::get_stream_health))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;
    
    Ok(())
}

async fn monitor_streams(manager: Arc<UnifiedStreamManager>) {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        let mut streams = manager.streams.write().await;
        let mut to_remove = Vec::new();
        
        for (id, stream) in streams.iter_mut() {
            // Update health from fallbacksrc statistics
            if let Ok(stats) = stream.source_bin
                .by_name(&format!("source_{}", id))
                .unwrap()
                .property::<gst::Structure>("statistics") 
            {
                stream.health.update_from_stats(&stats);
                
                // Check if should auto-remove
                if !stream.health.is_healthy && 
                   manager.config.auto_remove_unhealthy {
                    to_remove.push(id.clone());
                }
            }
        }
        
        // Remove unhealthy streams
        for id in to_remove {
            streams.remove(&id);
            println!("Auto-removed unhealthy stream: {}", id);
        }
    }
}
```

## Key Advantages of Unified Architecture

### 1. Resource Efficiency
- Single decode per stream (shared via tee)
- Unified buffer management
- Reduced memory footprint

### 2. Robustness
- Built-in retry/fallback mechanisms
- Isolated inference pipelines (won't crash recording)
- Graceful degradation

### 3. Simplified Operations
- Single deployment unit
- Unified logging and monitoring
- Consistent configuration

### 4. Performance
- Zero-copy between components (using GStreamer)
- Native Rust performance
- Optimized buffer handling

### 5. Flexibility
- Dynamic stream addition/removal
- Per-stream inference control
- Pluggable inference backends

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1-2)
- [ ] Basic pipeline management
- [ ] Stream addition/removal
- [ ] Health monitoring

### Phase 2: Recording (Week 3)
- [ ] Togglerecord integration
- [ ] Splitmuxsink configuration
- [ ] Storage management

### Phase 3: Inference (Week 4)
- [ ] NVIDIA inference branch
- [ ] CPU inference fallback
- [ ] Result processing

### Phase 4: API & Control (Week 5)
- [ ] REST API
- [ ] WebSocket for live stats
- [ ] Configuration management

### Phase 5: Production Hardening (Week 6)
- [ ] Error recovery
- [ ] Performance tuning
- [ ] Deployment packaging

## Configuration Example

```toml
[app]
auto_remove_unhealthy = true
health_check_interval = 5

[recording]
base_path = "/recordings"
segment_duration = "10m"
format = "mp4"

[inference]
backend = "nvidia"  # or "cpu"
config_path = "/etc/inference/config.txt"
batch_size = 4

[stream_defaults]
timeout = "5s"
retry_timeout = "60s"
restart_timeout = "3s"
buffer_duration = "10s"

[[streams]]
id = "camera_1"
uri = "rtsp://192.168.1.100/stream"
enable_inference = true
enable_recording = true

[[streams]]
id = "camera_2"
uri = "rtsp://192.168.1.101/stream"
enable_inference = false
enable_recording = true
```

## Testing Strategy

### Unit Tests
- Stream management logic
- Health monitoring algorithms
- Recording state transitions

### Integration Tests
- Multi-stream scenarios
- Failure recovery
- API endpoints

### Load Tests
- Maximum concurrent streams
- CPU/GPU utilization
- Network bandwidth

## Deployment

### Docker Container
```dockerfile
FROM nvidia/cuda:12.0-base-ubuntu22.04

# Install GStreamer and plugins
RUN apt-get update && apt-get install -y \
    gstreamer1.0-tools \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-rtsp \
    deepstream-6.3

# Copy Rust binary
COPY target/release/unified-stream-manager /usr/local/bin/

# Copy configuration
COPY config.toml /etc/unified-stream/

EXPOSE 8080

CMD ["/usr/local/bin/unified-stream-manager"]
```

## Conclusion

By consolidating MediaMTX, Python DeepStream, and the control application into a single Rust application using gst-plugins-rs components, we achieve:

1. **Simplified architecture** - One application instead of three
2. **Better resource utilization** - Shared decoding and buffering
3. **Improved reliability** - Unified error handling and recovery
4. **Easier maintenance** - Single codebase and deployment
5. **Better performance** - Native Rust with zero-copy GStreamer pipelines

The existing gst-plugins-rs components (fallbacksrc, togglerecord, intersink/intersrc) provide production-ready building blocks that handle the complex aspects of stream management, making this consolidation both feasible and advantageous.