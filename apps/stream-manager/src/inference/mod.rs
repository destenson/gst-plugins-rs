// Inference pipeline management

pub mod nvidia;
pub mod deepstream_config;
pub mod cpu;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use self::nvidia::{NvidiaInference, NvidiaInferenceConfig, InferenceResult, GpuResourceManager};
use self::cpu::{CpuInference, CpuInferenceConfig, CpuResourceManager};

#[derive(Debug)]
pub enum InferenceBackend {
    Nvidia(NvidiaInferenceConfig),
    Cpu(CpuInferenceConfig),
}

#[derive(Debug)]
pub struct InferenceManager {
    backends: Arc<RwLock<HashMap<String, InferenceBackend>>>,
    nvidia_manager: Option<Arc<GpuResourceManager>>,
    cpu_manager: Option<Arc<CpuResourceManager>>,
    cpu_inferences: Arc<RwLock<HashMap<String, Arc<RwLock<CpuInference>>>>>,
    result_receiver: Arc<RwLock<Option<tokio::sync::mpsc::Receiver<InferenceResult>>>>,
    result_sender: tokio::sync::mpsc::Sender<InferenceResult>,
}

impl InferenceManager {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
            nvidia_manager: None,
            cpu_manager: None,
            cpu_inferences: Arc::new(RwLock::new(HashMap::new())),
            result_receiver: Arc::new(RwLock::new(Some(rx))),
            result_sender: tx,
        }
    }
    
    pub fn with_nvidia_support(max_concurrent: usize, gpu_memory_limit_mb: u64) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
            nvidia_manager: Some(Arc::new(GpuResourceManager::new(max_concurrent, gpu_memory_limit_mb))),
            cpu_manager: None,
            cpu_inferences: Arc::new(RwLock::new(HashMap::new())),
            result_receiver: Arc::new(RwLock::new(Some(rx))),
            result_sender: tx,
        }
    }
    
    pub fn with_cpu_support(max_concurrent: usize, num_threads: usize) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
            nvidia_manager: None,
            cpu_manager: Some(Arc::new(CpuResourceManager::new(max_concurrent, num_threads))),
            cpu_inferences: Arc::new(RwLock::new(HashMap::new())),
            result_receiver: Arc::new(RwLock::new(Some(rx))),
            result_sender: tx,
        }
    }
    
    pub fn with_both_backends(
        nvidia_max_concurrent: usize,
        gpu_memory_limit_mb: u64,
        cpu_max_concurrent: usize,
        cpu_threads: usize,
    ) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
            nvidia_manager: Some(Arc::new(GpuResourceManager::new(nvidia_max_concurrent, gpu_memory_limit_mb))),
            cpu_manager: Some(Arc::new(CpuResourceManager::new(cpu_max_concurrent, cpu_threads))),
            cpu_inferences: Arc::new(RwLock::new(HashMap::new())),
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
                InferenceBackend::Cpu(_) => "CPU",
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
            InferenceBackend::Cpu(config) => {
                if let Some(ref cpu_manager) = self.cpu_manager {
                    // Check if we can add another CPU inference
                    if !cpu_manager.can_add_inference().await {
                        return Err("Maximum concurrent CPU inferences reached".into());
                    }
                    
                    // Create CPU inference pipeline
                    let mut inference = CpuInference::new(
                        stream_id.clone(),
                        config.clone(),
                        self.result_sender.clone(),
                    )?;
                    
                    // Start the inference
                    inference.start().await?;
                    
                    // Add to CPU manager
                    cpu_manager.add_inference(stream_id.clone()).await?;
                    
                    // Store inference instance
                    let mut cpu_inferences = self.cpu_inferences.write().await;
                    cpu_inferences.insert(stream_id.clone(), Arc::new(RwLock::new(inference)));
                    
                    // Store backend info
                    let mut backends = self.backends.write().await;
                    backends.insert(stream_id.clone(), InferenceBackend::Cpu(config));
                    
                    info!("CPU inference added for stream {}", stream_id);
                } else {
                    return Err("CPU support not initialized".into());
                }
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
                InferenceBackend::Cpu(_) => {
                    if let Some(ref cpu_manager) = self.cpu_manager {
                        cpu_manager.remove_inference(stream_id).await?;
                    }
                    
                    // Stop and remove CPU inference instance
                    let mut cpu_inferences = self.cpu_inferences.write().await;
                    if let Some(inference) = cpu_inferences.remove(stream_id) {
                        let mut inference = inference.write().await;
                        inference.stop().await?;
                    }
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
    
    pub async fn add_inference_with_fallback(
        &self,
        stream_id: String,
        nvidia_config: Option<NvidiaInferenceConfig>,
        cpu_config: Option<CpuInferenceConfig>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Try NVIDIA first if available
        if let Some(nvidia_cfg) = nvidia_config {
            if self.nvidia_manager.is_some() {
                match self.add_inference_stream(
                    stream_id.clone(),
                    InferenceBackend::Nvidia(nvidia_cfg),
                ).await {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        warn!("Failed to add NVIDIA inference for {}: {}, trying CPU fallback", stream_id, e);
                    }
                }
            }
        }
        
        // Fallback to CPU
        if let Some(cpu_cfg) = cpu_config {
            if self.cpu_manager.is_some() {
                self.add_inference_stream(
                    stream_id,
                    InferenceBackend::Cpu(cpu_cfg),
                ).await?;
                return Ok(());
            }
        }
        
        Err("No inference backend available".into())
    }
    
    pub async fn queue_frame_for_inference(
        &self,
        stream_id: &str,
        frame_data: Vec<u8>,
        width: u32,
        height: u32,
        frame_num: u64,
        timestamp: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let backends = self.backends.read().await;
        
        if let Some(backend) = backends.get(stream_id) {
            match backend {
                InferenceBackend::Nvidia(_) => {
                    // NVIDIA inference handles frames through the GStreamer pipeline
                    // Frame queuing is managed by the nvidia module
                    Ok(())
                }
                InferenceBackend::Cpu(_) => {
                    // Queue frame for CPU inference
                    let cpu_inferences = self.cpu_inferences.read().await;
                    if let Some(inference) = cpu_inferences.get(stream_id) {
                        let inference = inference.read().await;
                        inference.queue_frame(frame_data, width, height, frame_num, timestamp).await?;
                    }
                    Ok(())
                }
            }
        } else {
            Err(format!("No inference backend found for stream {}", stream_id).into())
        }
    }
}

impl Default for InferenceManager {
    fn default() -> Self {
        Self::new()
    }
}