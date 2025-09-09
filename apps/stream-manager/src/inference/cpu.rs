// CPU-based inference using ONNX Runtime

use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

#[cfg(feature = "cpu-inference")]
use {
    ort::{Session, SessionBuilder, ExecutionProvider, Environment, GraphOptimizationLevel},
    ort::execution_providers::CPUExecutionProviderOptions,
    image::{DynamicImage, ImageBuffer, Rgb},
    ndarray::{Array3, Array4},
};

use super::nvidia::InferenceResult;

#[derive(Debug, Clone)]
pub struct CpuInferenceConfig {
    pub model_path: String,
    pub input_width: u32,
    pub input_height: u32,
    pub batch_size: usize,
    pub num_threads: usize,
    pub confidence_threshold: f32,
    pub skip_frames: usize,  // Process every N frames to reduce CPU load
    pub max_queue_size: usize,
    pub class_names: Vec<String>,
}

impl Default for CpuInferenceConfig {
    fn default() -> Self {
        Self {
            model_path: String::new(),
            input_width: 640,
            input_height: 480,
            batch_size: 1,
            num_threads: 4,
            confidence_threshold: 0.5,
            skip_frames: 5,  // Process every 5th frame by default
            max_queue_size: 100,
            class_names: vec![
                "person".to_string(),
                "car".to_string(),
                "truck".to_string(),
                "bus".to_string(),
                "motorcycle".to_string(),
                "bicycle".to_string(),
            ],
        }
    }
}

#[derive(Debug)]
pub struct DetectedObject {
    pub class_id: u32,
    pub confidence: f32,
    pub bbox: BoundingBox,
}

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct CpuInference {
    stream_id: String,
    config: CpuInferenceConfig,
    #[cfg(feature = "cpu-inference")]
    session: Option<Arc<Session>>,
    frame_queue: Arc<Mutex<VecDeque<FrameData>>>,
    result_sender: tokio::sync::mpsc::Sender<InferenceResult>,
    frame_counter: Arc<RwLock<usize>>,
    is_running: Arc<RwLock<bool>>,
    processing_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl std::fmt::Debug for CpuInference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpuInference")
            .field("stream_id", &self.stream_id)
            .field("config", &self.config)
            .field("frame_queue", &"<frame_queue>")
            .field("result_sender", &"<result_sender>")
            .field("frame_counter", &self.frame_counter)
            .field("is_running", &self.is_running)
            .field("processing_handle", &"<processing_handle>")
            .finish()
    }
}

struct FrameData {
    frame_num: u64,
    timestamp: i64,
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl CpuInference {
    pub fn new(
        stream_id: String,
        config: CpuInferenceConfig,
        result_sender: tokio::sync::mpsc::Sender<InferenceResult>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(feature = "cpu-inference")]
        {
            // Initialize ONNX Runtime session
            let session = if !config.model_path.is_empty() && Path::new(&config.model_path).exists() {
                info!("Loading ONNX model from: {}", config.model_path);
                
                let env = Arc::new(Environment::builder()
                    .with_name("cpu_inference")
                    .with_log_level(ort::LoggingLevel::Warning)
                    .build()?);
                
                let session = SessionBuilder::new(&env)?
                    .with_execution_providers([ExecutionProvider::CPU(CPUExecutionProviderOptions::default())])?
                    .with_intra_threads(config.num_threads as i16)?
                    .with_optimization_level(GraphOptimizationLevel::Level3)?
                    .with_model_from_file(&config.model_path)?;
                
                Some(Arc::new(session))
            } else {
                warn!("Model path not found or empty, CPU inference will be disabled");
                None
            };
            
            Ok(Self {
                stream_id,
                config,
                session,
                frame_queue: Arc::new(Mutex::new(VecDeque::new())),
                result_sender,
                frame_counter: Arc::new(RwLock::new(0)),
                is_running: Arc::new(RwLock::new(false)),
                processing_handle: Arc::new(Mutex::new(None)),
            })
        }
        
        #[cfg(not(feature = "cpu-inference"))]
        {
            Ok(Self {
                stream_id,
                config,
                frame_queue: Arc::new(Mutex::new(VecDeque::new())),
                result_sender,
                frame_counter: Arc::new(RwLock::new(0)),
                is_running: Arc::new(RwLock::new(false)),
                processing_handle: Arc::new(Mutex::new(None)),
            })
        }
    }
    
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }
        
        *is_running = true;
        info!("Starting CPU inference for stream {}", self.stream_id);
        
        // Start processing task
        let handle = self.spawn_processing_task().await?;
        let mut processing_handle = self.processing_handle.lock().await;
        *processing_handle = Some(handle);
        
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Stopping CPU inference for stream {}", self.stream_id);
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        // Wait for processing to finish
        let mut processing_handle = self.processing_handle.lock().await;
        if let Some(handle) = processing_handle.take() {
            handle.abort();
        }
        
        Ok(())
    }
    
    pub async fn queue_frame(
        &self,
        frame_data: Vec<u8>,
        width: u32,
        height: u32,
        frame_num: u64,
        timestamp: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we should skip this frame
        let mut frame_counter = self.frame_counter.write().await;
        *frame_counter += 1;
        
        if *frame_counter % (self.config.skip_frames + 1) != 0 {
            debug!("Skipping frame {} for CPU inference", frame_num);
            return Ok(());
        }
        
        let mut queue = self.frame_queue.lock().await;
        
        // Check queue size
        if queue.len() >= self.config.max_queue_size {
            warn!("Frame queue full, dropping oldest frame");
            queue.pop_front();
        }
        
        queue.push_back(FrameData {
            frame_num,
            timestamp,
            data: frame_data,
            width,
            height,
        });
        
        Ok(())
    }
    
    async fn spawn_processing_task(&self) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
        let stream_id = self.stream_id.clone();
        let frame_queue = self.frame_queue.clone();
        let result_sender = self.result_sender.clone();
        let is_running = self.is_running.clone();
        let config = self.config.clone();
        
        #[cfg(feature = "cpu-inference")]
        let session = self.session.clone();
        
        let handle = tokio::spawn(async move {
            info!("CPU inference processing task started for stream {}", stream_id);
            
            while *is_running.read().await {
                // Get frames from queue
                let frames = {
                    let mut queue = frame_queue.lock().await;
                    let batch_size = config.batch_size.min(queue.len());
                    
                    if batch_size == 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        continue;
                    }
                    
                    let mut frames = Vec::new();
                    for _ in 0..batch_size {
                        if let Some(frame) = queue.pop_front() {
                            frames.push(frame);
                        }
                    }
                    frames
                };
                
                if frames.is_empty() {
                    continue;
                }
                
                // Process frames
                #[cfg(feature = "cpu-inference")]
                {
                    if let Some(ref session) = session {
                        match Self::process_batch(&frames, session.as_ref(), &config).await {
                            Ok(results) => {
                                for (frame, objects) in frames.iter().zip(results.iter()) {
                                    let inference_result = InferenceResult {
                                        stream_id: stream_id.clone(),
                                        frame_num: frame.frame_num,
                                        timestamp: frame.timestamp,
                                        objects: objects.iter().map(|obj| {
                                            super::nvidia::DetectedObject {
                                                class_id: obj.class_id,
                                                confidence: obj.confidence,
                                                bbox: super::nvidia::BoundingBox {
                                                    x: obj.bbox.x,
                                                    y: obj.bbox.y,
                                                    width: obj.bbox.width,
                                                    height: obj.bbox.height,
                                                },
                                                class_label: format!("class_{}", obj.class_id),
                                                tracker_id: None,
                                            }
                                        }).collect(),
                                    };
                                    
                                    if let Err(e) = result_sender.send(inference_result).await {
                                        error!("Failed to send inference result: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to process batch: {}", e);
                            }
                        }
                    }
                }
                
                #[cfg(not(feature = "cpu-inference"))]
                {
                    // Send empty results when CPU inference is not enabled
                    for frame in frames.iter() {
                        let inference_result = InferenceResult {
                            stream_id: stream_id.clone(),
                            frame_num: frame.frame_num,
                            timestamp: frame.timestamp,
                            objects: Vec::new(),
                        };
                        
                        if let Err(e) = result_sender.send(inference_result).await {
                            error!("Failed to send inference result: {}", e);
                        }
                    }
                }
            }
            
            info!("CPU inference processing task stopped for stream {}", stream_id);
        });
        
        Ok(handle)
    }
    
    #[cfg(feature = "cpu-inference")]
    async fn process_batch(
        frames: &[FrameData],
        session: &Session,
        config: &CpuInferenceConfig,
    ) -> Result<Vec<Vec<DetectedObject>>, Box<dyn std::error::Error + Send + Sync>> {
        // Preprocess frames sequentially for now (can be parallelized later)
        let mut preprocessed = Vec::new();
        for frame in frames {
            preprocessed.push(Self::preprocess_frame(&frame.data, frame.width, frame.height, config)?);
        }
        
        // Stack into batch tensor
        let batch_tensor = Self::create_batch_tensor(preprocessed)?;
        
        // Run inference - convert to ORT tensor and run
        let input_tensor = ort::Value::from_array(session.allocator(), &batch_tensor.view())?;
        let outputs = session.run(vec![input_tensor])?;
        
        // Parse outputs
        let results = Self::parse_outputs(outputs, config)?;
        
        Ok(results)
    }
    
    #[cfg(feature = "cpu-inference")]
    fn preprocess_frame(
        data: &[u8],
        width: u32,
        height: u32,
        config: &CpuInferenceConfig,
    ) -> Result<Array3<f32>, Box<dyn std::error::Error + Send + Sync>> {
        // Convert raw frame data to image
        let img = if data.len() == (width * height * 3) as usize {
            // RGB data
            ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, data.to_vec())
                .ok_or("Failed to create image from raw data")?
                .into()
        } else {
            return Err("Unsupported frame format".into());
        };
        
        let img: DynamicImage = img;
        
        // Resize to model input size
        let resized = img.resize_exact(
            config.input_width,
            config.input_height,
            image::imageops::FilterType::Triangle,
        );
        
        // Convert to normalized tensor
        let rgb = resized.to_rgb8();
        let mut tensor = ndarray::Array3::<f32>::zeros((3, config.input_height as usize, config.input_width as usize));
        
        for (x, y, pixel) in rgb.enumerate_pixels() {
            let channels = pixel.0;
            tensor[[0, y as usize, x as usize]] = channels[0] as f32 / 255.0;
            tensor[[1, y as usize, x as usize]] = channels[1] as f32 / 255.0;
            tensor[[2, y as usize, x as usize]] = channels[2] as f32 / 255.0;
        }
        
        Ok(tensor)
    }
    
    #[cfg(feature = "cpu-inference")]
    fn create_batch_tensor(
        preprocessed: Vec<Array3<f32>>,
    ) -> Result<Array4<f32>, Box<dyn std::error::Error + Send + Sync>> {
        if preprocessed.is_empty() {
            return Err("No frames to process".into());
        }
        
        let batch_size = preprocessed.len();
        let (channels, height, width) = preprocessed[0].dim();
        
        let mut batch = ndarray::Array4::<f32>::zeros((batch_size, channels, height, width));
        
        for (i, frame) in preprocessed.into_iter().enumerate() {
            batch.slice_mut(ndarray::s![i, .., .., ..]).assign(&frame);
        }
        
        Ok(batch)
    }
    
    #[cfg(feature = "cpu-inference")]
    fn parse_outputs(
        outputs: Vec<ort::Value>,
        config: &CpuInferenceConfig,
    ) -> Result<Vec<Vec<DetectedObject>>, Box<dyn std::error::Error + Send + Sync>> {
        // This is a simplified parser - actual implementation depends on model output format
        // Assuming YOLO-style output: [batch, num_detections, (x, y, w, h, confidence, ...classes)]
        
        if outputs.is_empty() {
            return Err("No output tensors found".into());
        }
        
        // Extract tensor data from first output
        let output_array = outputs[0].try_extract_tensor::<f32>()?;
        let output_shape = output_array.shape();
        
        if output_shape.len() < 2 {
            return Err("Unexpected output shape".into());
        }
        
        let batch_size = output_shape[0];
        let mut batch_results = Vec::new();
        
        for batch_idx in 0..batch_size {
            let mut objects = Vec::new();
            
            // Parse detections for this batch item
            // This is model-specific and should be adapted based on actual model output
            
            // Placeholder implementation - replace with actual parsing logic
            objects.push(DetectedObject {
                class_id: 0,
                confidence: 0.95,
                bbox: BoundingBox {
                    x: 100.0,
                    y: 100.0,
                    width: 50.0,
                    height: 50.0,
                },
            });
            
            batch_results.push(objects);
        }
        
        Ok(batch_results)
    }
}

// CPU resource management
#[derive(Debug)]
pub struct CpuResourceManager {
    max_concurrent: usize,
    active_inferences: Arc<RwLock<Vec<String>>>,
    #[cfg(feature = "cpu-inference")]
    thread_pool: Arc<rayon::ThreadPool>,
}

impl CpuResourceManager {
    pub fn new(max_concurrent: usize, _num_threads: usize) -> Self {
        #[cfg(feature = "cpu-inference")]
        let thread_pool = {
            rayon::ThreadPoolBuilder::new()
                .num_threads(_num_threads)
                .build()
                .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap())
        };
        
        Self {
            max_concurrent,
            active_inferences: Arc::new(RwLock::new(Vec::new())),
            #[cfg(feature = "cpu-inference")]
            thread_pool: Arc::new(thread_pool),
        }
    }
    
    pub async fn can_add_inference(&self) -> bool {
        let active = self.active_inferences.read().await;
        active.len() < self.max_concurrent
    }
    
    pub async fn add_inference(&self, stream_id: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut active = self.active_inferences.write().await;
        
        if active.len() >= self.max_concurrent {
            return Err("Maximum concurrent CPU inferences reached".into());
        }
        
        active.push(stream_id);
        Ok(())
    }
    
    pub async fn remove_inference(&self, stream_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut active = self.active_inferences.write().await;
        active.retain(|id| id != stream_id);
        Ok(())
    }
    
    pub async fn get_active_count(&self) -> usize {
        let active = self.active_inferences.read().await;
        active.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cpu_inference_creation() {
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        let config = CpuInferenceConfig::default();
        
        let inference = CpuInference::new(
            "test_stream".to_string(),
            config,
            tx,
        );
        
        assert!(inference.is_ok());
    }
    
    #[tokio::test]
    async fn test_frame_skipping() {
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        let mut config = CpuInferenceConfig::default();
        config.skip_frames = 2; // Process every 3rd frame
        
        let inference = CpuInference::new(
            "test_stream".to_string(),
            config,
            tx,
        ).unwrap();
        
        // Queue multiple frames
        for i in 0..10 {
            let _ = inference.queue_frame(
                vec![0; 640 * 480 * 3],
                640,
                480,
                i,
                i as i64,
            ).await;
        }
        
        // Check that only some frames were queued
        let queue = inference.frame_queue.lock().await;
        assert!(queue.len() < 10);
    }
    
    #[tokio::test]
    async fn test_cpu_resource_manager() {
        let manager = CpuResourceManager::new(2, 4);
        
        assert!(manager.can_add_inference().await);
        
        manager.add_inference("stream1".to_string()).await.unwrap();
        manager.add_inference("stream2".to_string()).await.unwrap();
        
        assert!(!manager.can_add_inference().await);
        assert_eq!(manager.get_active_count().await, 2);
        
        manager.remove_inference("stream1").await.unwrap();
        assert!(manager.can_add_inference().await);
        assert_eq!(manager.get_active_count().await, 1);
    }
    
    #[test]
    fn test_bounding_box_creation() {
        let bbox = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 200.0,
        };
        
        assert_eq!(bbox.x, 10.0);
        assert_eq!(bbox.y, 20.0);
        assert_eq!(bbox.width, 100.0);
        assert_eq!(bbox.height, 200.0);
    }
    
    #[test]
    fn test_config_defaults() {
        let config = CpuInferenceConfig::default();
        
        assert_eq!(config.input_width, 640);
        assert_eq!(config.input_height, 480);
        assert_eq!(config.batch_size, 1);
        assert_eq!(config.num_threads, 4);
        assert_eq!(config.skip_frames, 5);
        assert_eq!(config.confidence_threshold, 0.5);
    }
}