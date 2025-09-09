// Inference pipeline management

pub mod nvidia;
pub mod deepstream_config;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use self::nvidia::{NvidiaInference, NvidiaInferenceConfig, InferenceResult, GpuResourceManager};

#[derive(Debug)]
pub enum InferenceBackend {
    Nvidia(NvidiaInferenceConfig),
    Cpu, // To be implemented in PRP-22
}

#[derive(Debug)]
pub struct InferenceManager {
    backends: Arc<RwLock<HashMap<String, InferenceBackend>>>,
    nvidia_manager: Option<Arc<GpuResourceManager>>,
    result_receiver: Arc<RwLock<Option<tokio::sync::mpsc::Receiver<InferenceResult>>>>,
    result_sender: tokio::sync::mpsc::Sender<InferenceResult>,
}

impl InferenceManager {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
            nvidia_manager: None,
            result_receiver: Arc::new(RwLock::new(Some(rx))),
            result_sender: tx,
        }
    }
    
    pub fn with_nvidia_support(max_concurrent: usize, gpu_memory_limit_mb: u64) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
            nvidia_manager: Some(Arc::new(GpuResourceManager::new(max_concurrent, gpu_memory_limit_mb))),
            result_receiver: Arc::new(RwLock::new(Some(rx))),
            result_sender: tx,
        }
    }
    
    pub async fn add_inference_stream(
        &self,
        stream_id: String,
        backend: InferenceBackend,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Adding inference stream {} with backend {:?}", stream_id, 
            match &backend {
                InferenceBackend::Nvidia(_) => "NVIDIA",
                InferenceBackend::Cpu => "CPU",
            }
        );
        
        match backend {
            InferenceBackend::Nvidia(config) => {
                if let Some(ref gpu_manager) = self.nvidia_manager {
                    // Create NVIDIA inference pipeline
                    let mut inference = NvidiaInference::new(
                        stream_id.clone(),
                        config.clone(),
                        self.result_sender.clone(),
                    )?;
                    
                    // Start the inference
                    inference.start().await?;
                    
                    // Add to GPU manager
                    gpu_manager.add_inference(
                        stream_id.clone(),
                        Arc::new(RwLock::new(inference)),
                    ).await?;
                    
                    // Store backend info
                    let mut backends = self.backends.write().await;
                    backends.insert(stream_id.clone(), InferenceBackend::Nvidia(config));
                    
                    info!("NVIDIA inference added for stream {}", stream_id);
                } else {
                    return Err("NVIDIA support not initialized".into());
                }
            }
            InferenceBackend::Cpu => {
                // TODO: Implement CPU inference in PRP-22
                warn!("CPU inference not yet implemented");
                return Err("CPU inference not yet implemented".into());
            }
        }
        
        Ok(())
    }
    
    pub async fn remove_inference_stream(&self, stream_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Removing inference stream {}", stream_id);
        
        let mut backends = self.backends.write().await;
        
        if let Some(backend) = backends.remove(stream_id) {
            match backend {
                InferenceBackend::Nvidia(_) => {
                    if let Some(ref gpu_manager) = self.nvidia_manager {
                        gpu_manager.remove_inference(stream_id).await?;
                    }
                }
                InferenceBackend::Cpu => {
                    // TODO: Handle CPU inference removal
                }
            }
            
            info!("Inference removed for stream {}", stream_id);
        } else {
            warn!("No inference found for stream {}", stream_id);
        }
        
        Ok(())
    }
    
    pub async fn process_results(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut receiver = self.result_receiver.write().await;
        
        if let Some(ref mut rx) = *receiver {
            while let Ok(result) = rx.try_recv() {
                // Process inference results
                info!(
                    "Inference result for stream {}: {} objects detected at frame {}",
                    result.stream_id,
                    result.objects.len(),
                    result.frame_num
                );
                
                // TODO: Send results to database, webhook, or other consumers
            }
        }
        
        Ok(())
    }
    
    pub async fn handle_gpu_oom(&self) -> Result<(), Box<dyn std::error::Error>> {
        error!("Handling GPU OOM condition");
        
        if let Some(ref gpu_manager) = self.nvidia_manager {
            gpu_manager.handle_oom_error().await?;
        }
        
        Ok(())
    }
    
    pub async fn get_active_streams(&self) -> Vec<String> {
        let backends = self.backends.read().await;
        backends.keys().cloned().collect()
    }
}

impl Default for InferenceManager {
    fn default() -> Self {
        Self::new()
    }
}