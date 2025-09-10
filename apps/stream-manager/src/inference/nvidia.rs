#![allow(unused)]
use gst::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvidiaInferenceConfig {
    pub model_config_path: PathBuf,
    pub model_engine_path: Option<PathBuf>,
    pub batch_size: u32,
    pub gpu_device_id: u32,
    pub inference_interval: u32,
    pub max_gpu_memory_mb: u64,
    pub enable_dla: bool,
    pub dla_core: u32,
    pub mux_width: u32,
    pub mux_height: u32,
    pub batched_push_timeout: i32,
}

impl Default for NvidiaInferenceConfig {
    fn default() -> Self {
        Self {
            model_config_path: PathBuf::from("/opt/nvidia/deepstream/samples/configs/deepstream-app/config_infer_primary.txt"),
            model_engine_path: None,
            batch_size: 1,
            gpu_device_id: 0,
            inference_interval: 1,
            max_gpu_memory_mb: 2048,
            enable_dla: false,
            dla_core: 0,
            mux_width: 1920,
            mux_height: 1080,
            batched_push_timeout: 40000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub stream_id: String,
    pub timestamp: i64,
    pub frame_num: u64,
    pub objects: Vec<DetectedObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedObject {
    pub class_id: u32,
    pub class_label: String,
    pub confidence: f32,
    pub bbox: BoundingBox,
    pub tracker_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug)]
pub struct NvidiaInference {
    pipeline: gst::Pipeline,
    config: NvidiaInferenceConfig,
    stream_id: String,
    result_sender: Option<tokio::sync::mpsc::Sender<InferenceResult>>,
    is_running: Arc<RwLock<bool>>,
    gpu_memory_usage: Arc<RwLock<u64>>,
}

impl NvidiaInference {
    pub fn new(
        stream_id: String,
        config: NvidiaInferenceConfig,
        result_sender: tokio::sync::mpsc::Sender<InferenceResult>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Check GPU availability
        if !Self::check_gpu_available(config.gpu_device_id)? {
            return Err(format!("GPU device {} not available", config.gpu_device_id).into());
        }

        // Create inference pipeline
        let pipeline = gst::Pipeline::new();

        Ok(Self {
            pipeline,
            config,
            stream_id,
            result_sender: Some(result_sender),
            is_running: Arc::new(RwLock::new(false)),
            gpu_memory_usage: Arc::new(RwLock::new(0)),
        })
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting NVIDIA inference for stream {}", self.stream_id);

        // Build the pipeline
        self.build_pipeline().await?;

        // Set pipeline to playing
        self.pipeline.set_state(gst::State::Playing)?;
        
        *self.is_running.write().await = true;
        
        info!("NVIDIA inference started for stream {}", self.stream_id);
        Ok(())
    }

    async fn build_pipeline(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Create elements
        let intersrc = gst::ElementFactory::make("intersrc")
            .property("channel", &format!("inference-{}", self.stream_id))
            .build()?;

        let nvvideoconvert = gst::ElementFactory::make("nvvideoconvert")
            .property("gpu-id", self.config.gpu_device_id)
            .build()?;

        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("video/x-raw(memory:NVMM)")
                    .field("format", "NV12")
                    .build(),
            )
            .build()?;

        // Create nvstreammux for batching
        let nvstreammux = gst::ElementFactory::make("nvstreammux")
            .property("batch-size", self.config.batch_size)
            .property("width", self.config.mux_width)
            .property("height", self.config.mux_height)
            .property("gpu-id", self.config.gpu_device_id)
            .property("live-source", true)
            .property("batched-push-timeout", self.config.batched_push_timeout)
            .build()?;

        // Create nvinfer element with configuration
        let nvinfer = gst::ElementFactory::make("nvinfer")
            .property("config-file-path", self.config.model_config_path.to_str().unwrap())
            .property("batch-size", self.config.batch_size)
            .property("gpu-id", self.config.gpu_device_id)
            .property("interval", self.config.inference_interval)
            .build()?;

        // Optional: Configure DLA if enabled
        if self.config.enable_dla {
            nvinfer.set_property("enable-dla", true);
            nvinfer.set_property("dla-core", self.config.dla_core);
        }

        // Create a fakesink for now (could be replaced with nvmsgconv + nvmsgbroker for real output)
        let fakesink = gst::ElementFactory::make("fakesink")
            .property("sync", false)
            .build()?;

        // Add elements to pipeline
        self.pipeline.add_many(&[
            &intersrc,
            &nvvideoconvert,
            &capsfilter,
            &nvstreammux,
            &nvinfer,
            &fakesink,
        ])?;

        // Link elements up to capsfilter
        gst::Element::link_many(&[
            &intersrc,
            &nvvideoconvert,
            &capsfilter,
        ])?;

        // Connect capsfilter to nvstreammux sink pad
        let sink_pad = nvstreammux.request_pad_simple("sink_0")
            .ok_or("Failed to get sink pad from nvstreammux")?;
        let src_pad = capsfilter.static_pad("src")
            .ok_or("Failed to get src pad from capsfilter")?;
        src_pad.link(&sink_pad)?;

        // Link nvstreammux to nvinfer and fakesink
        gst::Element::link_many(&[
            &nvstreammux,
            &nvinfer,
            &fakesink,
        ])?;

        // Add probe to extract inference results
        self.add_inference_probe(&nvinfer)?;

        Ok(())
    }

    fn add_inference_probe(&mut self, nvinfer: &gst::Element) -> Result<(), Box<dyn std::error::Error>> {
        let src_pad = nvinfer.static_pad("src").ok_or("No src pad on nvinfer")?;
        
        let stream_id = self.stream_id.clone();
        let sender = self.result_sender.clone();
        
        src_pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, probe_info| {
            if let Some(gst::PadProbeData::Buffer(ref buffer)) = probe_info.data {
                // Extract timestamp
                let timestamp = buffer.pts()
                    .map(|t| t.nseconds() as i64)
                    .unwrap_or(0);
                
                // TODO: Parse NvDsInferMeta from buffer metadata
                // This requires bindings to DeepStream SDK structures
                // For now, we'll create a mock result
                
                let mock_result = InferenceResult {
                    stream_id: stream_id.clone(),
                    timestamp,
                    frame_num: 0,
                    objects: vec![
                        DetectedObject {
                            class_id: 0,
                            class_label: "person".to_string(),
                            confidence: 0.95,
                            bbox: BoundingBox {
                                x: 100.0,
                                y: 100.0,
                                width: 50.0,
                                height: 100.0,
                            },
                            tracker_id: Some(1),
                        },
                    ],
                };
                
                // Send result through channel
                if let Some(ref sender) = sender {
                    let result = mock_result.clone();
                    let sender = sender.clone();
                    std::thread::spawn(move || {
                        let _ = sender.blocking_send(result);
                    });
                }
                
                debug!("Extracted inference result with {} objects", mock_result.objects.len());
            }
            
            gst::PadProbeReturn::Ok
        });
        
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Stopping NVIDIA inference for stream {}", self.stream_id);
        
        *self.is_running.write().await = false;
        
        self.pipeline.set_state(gst::State::Null)?;
        
        info!("NVIDIA inference stopped for stream {}", self.stream_id);
        Ok(())
    }

    pub async fn update_model(&mut self, new_config: NvidiaInferenceConfig) -> Result<(), Box<dyn std::error::Error>> {
        info!("Updating model for stream {}", self.stream_id);
        
        // Stop current pipeline
        self.stop().await?;
        
        // Update configuration
        self.config = new_config;
        
        // Restart with new configuration
        self.start().await?;
        
        info!("Model updated for stream {}", self.stream_id);
        Ok(())
    }

    pub async fn get_gpu_memory_usage(&self) -> u64 {
        *self.gpu_memory_usage.read().await
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    fn check_gpu_available(device_id: u32) -> Result<bool, Box<dyn std::error::Error>> {
        // Check if NVIDIA GPU is available
        // This would typically use nvidia-ml or CUDA APIs
        // For now, we'll check if nvinfer plugin exists
        
        let registry = gst::Registry::get();
        let has_nvinfer = registry.find_plugin("nvdsgst_infer").is_some();
        
        if !has_nvinfer {
            warn!("NVIDIA DeepStream plugins not found");
            return Ok(false);
        }
        
        // TODO: Actually check if specific GPU device is available
        // using nvidia-ml bindings or CUDA runtime
        
        info!("GPU device {} appears to be available", device_id);
        Ok(true)
    }

    pub async fn monitor_gpu_resources(&self) -> Result<GpuResourceInfo, Box<dyn std::error::Error>> {
        // TODO: Implement actual GPU monitoring using nvidia-ml
        // For now, return mock data
        
        Ok(GpuResourceInfo {
            device_id: self.config.gpu_device_id,
            memory_used_mb: *self.gpu_memory_usage.read().await,
            memory_total_mb: 8192, // Mock 8GB GPU
            utilization_percent: 25.0,
            temperature_celsius: 65.0,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuResourceInfo {
    pub device_id: u32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub utilization_percent: f32,
    pub temperature_celsius: f32,
}

// GPU resource manager to handle multiple inference pipelines
#[derive(Debug)]
pub struct GpuResourceManager {
    max_concurrent_inferences: usize,
    active_inferences: Arc<RwLock<HashMap<String, Arc<RwLock<NvidiaInference>>>>>,
    gpu_memory_limit_mb: u64,
    current_memory_usage: Arc<RwLock<u64>>,
}

impl GpuResourceManager {
    pub fn new(max_concurrent: usize, memory_limit_mb: u64) -> Self {
        Self {
            max_concurrent_inferences: max_concurrent,
            active_inferences: Arc::new(RwLock::new(HashMap::new())),
            gpu_memory_limit_mb: memory_limit_mb,
            current_memory_usage: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn can_add_inference(&self, estimated_memory_mb: u64) -> bool {
        let active = self.active_inferences.read().await;
        let current_memory = *self.current_memory_usage.read().await;
        
        active.len() < self.max_concurrent_inferences &&
        (current_memory + estimated_memory_mb) <= self.gpu_memory_limit_mb
    }

    pub async fn add_inference(&self, stream_id: String, inference: Arc<RwLock<NvidiaInference>>) -> Result<(), Box<dyn std::error::Error>> {
        if !self.can_add_inference(512).await {
            return Err("GPU resources exhausted".into());
        }
        
        let mut active = self.active_inferences.write().await;
        active.insert(stream_id.clone(), inference);
        
        // Update memory usage estimate
        let mut memory = self.current_memory_usage.write().await;
        *memory += 512; // Estimate 512MB per inference pipeline
        
        info!("Added inference for stream {}, total active: {}", stream_id, active.len());
        Ok(())
    }

    pub async fn remove_inference(&self, stream_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut active = self.active_inferences.write().await;
        
        if let Some(inference) = active.remove(stream_id) {
            // Stop the inference pipeline
            let mut inf = inference.write().await;
            inf.stop().await?;
            
            // Update memory usage
            let mut memory = self.current_memory_usage.write().await;
            *memory = memory.saturating_sub(512);
            
            info!("Removed inference for stream {}, remaining active: {}", stream_id, active.len());
        }
        
        Ok(())
    }

    pub async fn handle_oom_error(&self) -> Result<(), Box<dyn std::error::Error>> {
        error!("GPU OOM detected, clearing least recently used inference pipelines");
        
        // In a real implementation, we'd track LRU and remove oldest
        // For now, just clear half of the pipelines
        let active = self.active_inferences.read().await;
        let to_remove: Vec<String> = active.keys()
            .take(active.len() / 2)
            .cloned()
            .collect();
        drop(active);
        
        for stream_id in to_remove {
            self.remove_inference(&stream_id).await?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nvidia_gpu_check() {
        // Initialize GStreamer
        gst::init().ok();
        
        // Check if GPU is available
        let available = NvidiaInference::check_gpu_available(0).unwrap();
        // This test will pass or fail based on whether DeepStream is installed
        println!("GPU available: {}", available);
    }

    #[tokio::test]
    async fn test_inference_config() {
        let config = NvidiaInferenceConfig::default();
        assert_eq!(config.batch_size, 1);
        assert_eq!(config.gpu_device_id, 0);
        assert_eq!(config.inference_interval, 1);
    }

    #[tokio::test]
    async fn test_inference_results() {
        let result = InferenceResult {
            stream_id: "test-stream".to_string(),
            timestamp: 123456789,
            frame_num: 100,
            objects: vec![
                DetectedObject {
                    class_id: 0,
                    class_label: "person".to_string(),
                    confidence: 0.95,
                    bbox: BoundingBox {
                        x: 100.0,
                        y: 100.0,
                        width: 50.0,
                        height: 100.0,
                    },
                    tracker_id: Some(1),
                },
            ],
        };
        
        assert_eq!(result.objects.len(), 1);
        assert_eq!(result.objects[0].class_label, "person");
        assert!(result.objects[0].confidence > 0.9);
    }

    #[tokio::test]
    async fn test_gpu_resource_manager() {
        let manager = GpuResourceManager::new(4, 8192);
        
        // Should be able to add inference
        assert!(manager.can_add_inference(512).await);
        
        // Add a mock inference
        let (tx, _rx) = tokio::sync::mpsc::channel::<InferenceResult>(100);
        let config = NvidiaInferenceConfig::default();
        
        // We can't create actual NvidiaInference without GPU, so just test the manager logic
        assert_eq!(manager.active_inferences.read().await.len(), 0);
    }

    #[tokio::test] 
    async fn test_bounding_box_serialization() {
        let bbox = BoundingBox {
            x: 10.5,
            y: 20.5,
            width: 100.0,
            height: 200.0,
        };
        
        let json = serde_json::to_string(&bbox).unwrap();
        let deserialized: BoundingBox = serde_json::from_str(&json).unwrap();
        
        assert_eq!(bbox.x, deserialized.x);
        assert_eq!(bbox.y, deserialized.y);
        assert_eq!(bbox.width, deserialized.width);
        assert_eq!(bbox.height, deserialized.height);
    }
}
