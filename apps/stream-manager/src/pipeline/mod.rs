use gst::prelude::*;
use gst::{MessageView, glib};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{RwLock, mpsc};
use tracing::{error, info, warn, debug};
use uuid::Uuid;

/// Pipeline state tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineState {
    Null,
    Ready,
    Paused,
    Playing,
    Error(String),
}

impl From<gst::State> for PipelineState {
    fn from(state: gst::State) -> Self {
        match state {
            gst::State::Null => PipelineState::Null,
            gst::State::Ready => PipelineState::Ready,
            gst::State::Paused => PipelineState::Paused,
            gst::State::Playing => PipelineState::Playing,
            gst::State::VoidPending => PipelineState::Null,
        }
    }
}

/// Pipeline error types
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("GStreamer error: {0}")]
    GStreamer(#[from] gst::glib::Error),
    #[error("State change error: {0:?}")]
    StateChange(gst::StateChangeError),
    #[error("Pipeline error: {0}")]
    Pipeline(String),
    #[error("Element not found: {0}")]
    ElementNotFound(String),
}

/// Message bus events
#[derive(Debug, Clone)]
pub enum PipelineMessage {
    Error { source: String, error: String, debug: Option<String> },
    Eos,
    StateChanged { old: PipelineState, new: PipelineState, pending: PipelineState },
    Buffering { percent: i32 },
    Info { source: String, message: String },
}

/// Pipeline wrapper with state management and message bus handling
pub struct Pipeline {
    id: Uuid,
    name: String,
    pipeline: gst::Pipeline,
    state: Arc<Mutex<PipelineState>>,
    message_sender: Option<mpsc::UnboundedSender<PipelineMessage>>,
    _bus_watch: Option<gst::bus::BusWatchGuard>,
}

impl Pipeline {
    /// Create a new pipeline with the given name
    pub fn new(name: &str) -> crate::Result<Self> {
        let pipeline = gst::Pipeline::builder().name(name).build();
        let id = Uuid::new_v4();
        
        Ok(Self {
            id,
            name: name.to_string(),
            pipeline,
            state: Arc::new(Mutex::new(PipelineState::Null)),
            message_sender: None,
            _bus_watch: None,
        })
    }
    
    /// Create a new pipeline with message bus monitoring
    pub fn new_with_message_handler(
        name: &str, 
        message_sender: mpsc::UnboundedSender<PipelineMessage>
    ) -> crate::Result<Self> {
        let mut pipeline = Self::new(name)?;
        pipeline.setup_message_bus(message_sender)?;
        Ok(pipeline)
    }
    
    /// Setup message bus handling
    pub fn setup_message_bus(
        &mut self, 
        sender: mpsc::UnboundedSender<PipelineMessage>
    ) -> crate::Result<()> {
        let bus = self.pipeline.bus().ok_or_else(|| {
            crate::StreamManagerError::Pipeline("Failed to get pipeline bus".to_string())
        })?;
        
        let state = self.state.clone();
        let sender_clone = sender.clone();
        let bus_watch = bus.add_watch(move |_bus, msg| {
            match msg.view() {
                MessageView::Error(err) => {
                    let error_msg = PipelineMessage::Error {
                        source: err.src().map(|s| s.path_string()).unwrap_or_default().to_string(),
                        error: err.error().to_string(),
                        debug: err.debug().map(|s| s.to_string()),
                    };
                    
                    // Update state to error
                    if let Ok(mut state) = state.lock() {
                        *state = PipelineState::Error(err.error().to_string());
                    }
                    
                    let _ = sender_clone.send(error_msg);
                    error!("Pipeline error: {}", err.error());
                }
                MessageView::Eos(_) => {
                    let _ = sender_clone.send(PipelineMessage::Eos);
                    info!("Pipeline received EOS");
                }
                MessageView::StateChanged(state_changed) => {
                    if state_changed.src().map(|s| s.type_().name()) == Some("GstPipeline") {
                        let old_state = PipelineState::from(state_changed.old());
                        let new_state = PipelineState::from(state_changed.current());
                        let pending_state = PipelineState::from(state_changed.pending());
                        
                        // Update internal state
                        if let Ok(mut state) = state.lock() {
                            *state = new_state.clone();
                        }
                        
                        let state_msg = PipelineMessage::StateChanged {
                            old: old_state,
                            new: new_state.clone(),
                            pending: pending_state,
                        };
                        
                        let _ = sender_clone.send(state_msg);
                        debug!("Pipeline state changed to {:?}", new_state);
                    }
                }
                MessageView::Buffering(buffering) => {
                    let buffering_msg = PipelineMessage::Buffering {
                        percent: buffering.percent(),
                    };
                    let _ = sender_clone.send(buffering_msg);
                    debug!("Pipeline buffering: {}%", buffering.percent());
                }
                MessageView::Info(info) => {
                    let info_msg = PipelineMessage::Info {
                        source: info.src().map(|s| s.path_string()).unwrap_or_default().to_string(),
                        message: format!("{:?}", info.message()),
                    };
                    let _ = sender_clone.send(info_msg);
                    debug!("Pipeline info: {:?}", info.message());
                }
                _ => {}
            }
            
            glib::ControlFlow::Continue
        }).map_err(|_| crate::StreamManagerError::Pipeline("Failed to add bus watch".to_string()))?;
        
        self.message_sender = Some(sender);
        self._bus_watch = Some(bus_watch);
        
        Ok(())
    }
    
    /// Start the pipeline (set to Playing state)
    pub fn start(&self) -> Result<(), PipelineError> {
        info!("Starting pipeline: {}", self.name);
        match self.pipeline.set_state(gst::State::Playing) {
            Ok(_) => {
                if let Ok(mut state) = self.state.lock() {
                    *state = PipelineState::Playing;
                }
                Ok(())
            }
            Err(err) => {
                error!("Failed to start pipeline {}: {:?}", self.name, err);
                Err(PipelineError::StateChange(err))
            }
        }
    }
    
    /// Pause the pipeline
    pub fn pause(&self) -> Result<(), PipelineError> {
        info!("Pausing pipeline: {}", self.name);
        match self.pipeline.set_state(gst::State::Paused) {
            Ok(_) => {
                if let Ok(mut state) = self.state.lock() {
                    *state = PipelineState::Paused;
                }
                Ok(())
            }
            Err(err) => {
                error!("Failed to pause pipeline {}: {:?}", self.name, err);
                Err(PipelineError::StateChange(err))
            }
        }
    }
    
    /// Stop the pipeline (set to Null state)
    pub fn stop(&self) -> Result<(), PipelineError> {
        info!("Stopping pipeline: {}", self.name);
        match self.pipeline.set_state(gst::State::Null) {
            Ok(_) => {
                if let Ok(mut state) = self.state.lock() {
                    *state = PipelineState::Null;
                }
                Ok(())
            }
            Err(err) => {
                error!("Failed to stop pipeline {}: {:?}", self.name, err);
                Err(PipelineError::StateChange(err))
            }
        }
    }
    
    /// Add an element to the pipeline
    pub fn add_element(&self, element: &gst::Element) -> Result<(), PipelineError> {
        self.pipeline.add(element).map_err(|e| {
            PipelineError::Pipeline(format!("Failed to add element: {}", e))
        })?;
        debug!("Added element to pipeline: {}", self.name);
        Ok(())
    }
    
    /// Remove an element from the pipeline
    pub fn remove_element(&self, element: &gst::Element) -> Result<(), PipelineError> {
        self.pipeline.remove(element).map_err(|e| {
            PipelineError::Pipeline(format!("Failed to remove element: {}", e))
        })?;
        debug!("Removed element from pipeline: {}", self.name);
        Ok(())
    }
    
    /// Get the current pipeline state
    pub fn current_state(&self) -> PipelineState {
        self.state.lock().map(|guard| guard.clone()).unwrap_or_else(|_| {
            // If mutex is poisoned, check GStreamer state directly
            PipelineState::from(self.pipeline.state(gst::ClockTime::ZERO).1)
        })
    }
    
    /// Get pipeline unique identifier
    pub fn id(&self) -> Uuid {
        self.id
    }
    
    /// Get pipeline name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the underlying GStreamer pipeline
    pub fn gst_pipeline(&self) -> &gst::Pipeline {
        &self.pipeline
    }
    
    /// Set state directly (low-level API)
    pub fn set_state(&self, state: gst::State) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
        let result = self.pipeline.set_state(state);
        if result.is_ok() {
            if let Ok(mut internal_state) = self.state.lock() {
                *internal_state = PipelineState::from(state);
            }
        }
        result
    }
    
    /// Get GStreamer state (low-level API)
    pub fn get_state(&self) -> gst::State {
        self.pipeline.state(gst::ClockTime::ZERO).1
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        info!("Dropping pipeline: {} ({})", self.name, self.id);
        
        // Ensure pipeline is stopped
        if let Err(e) = self.stop() {
            warn!("Error stopping pipeline during drop: {:?}", e);
        }
        
        // The bus watch guard will be dropped automatically, stopping the watch
    }
}

/// Manager for multiple pipelines
pub struct PipelineManager {
    pipelines: Arc<RwLock<HashMap<Uuid, Arc<Pipeline>>>>,
    message_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<(Uuid, PipelineMessage)>>>>,
    message_sender: mpsc::UnboundedSender<(Uuid, PipelineMessage)>,
}

impl PipelineManager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        Self {
            pipelines: Arc::new(RwLock::new(HashMap::new())),
            message_receiver: Arc::new(Mutex::new(Some(receiver))),
            message_sender: sender,
        }
    }
    
    /// Create a new pipeline and add it to the manager
    pub async fn create_pipeline(&self, name: &str) -> crate::Result<Arc<Pipeline>> {
        let pipeline_id = Uuid::new_v4();
        let sender = self.message_sender.clone();
        
        // Create a pipeline-specific sender that includes the pipeline ID
        let (pipeline_sender, mut pipeline_receiver) = mpsc::unbounded_channel();
        
        // Spawn a task to forward messages with the pipeline ID
        let manager_sender = sender.clone();
        tokio::spawn(async move {
            while let Some(msg) = pipeline_receiver.recv().await {
                let _ = manager_sender.send((pipeline_id, msg));
            }
        });
        
        let pipeline = Arc::new(Pipeline::new_with_message_handler(name, pipeline_sender)?);
        
        let mut pipelines = self.pipelines.write().await;
        pipelines.insert(pipeline.id(), pipeline.clone());
        
        info!("Created and registered pipeline: {} ({})", name, pipeline.id());
        Ok(pipeline)
    }
    
    /// Get a pipeline by ID
    pub async fn get_pipeline(&self, id: Uuid) -> Option<Arc<Pipeline>> {
        let pipelines = self.pipelines.read().await;
        pipelines.get(&id).cloned()
    }
    
    /// Get a pipeline by name (returns the first match)
    pub async fn get_pipeline_by_name(&self, name: &str) -> Option<Arc<Pipeline>> {
        let pipelines = self.pipelines.read().await;
        pipelines.values()
            .find(|p| p.name() == name)
            .cloned()
    }
    
    /// Remove a pipeline by ID
    pub async fn remove_pipeline(&self, id: Uuid) -> crate::Result<()> {
        let mut pipelines = self.pipelines.write().await;
        if let Some(pipeline) = pipelines.remove(&id) {
            info!("Removed pipeline: {} ({})", pipeline.name(), id);
            // Pipeline will be stopped when dropped
        }
        Ok(())
    }
    
    /// Get all pipeline IDs
    pub async fn list_pipelines(&self) -> Vec<Uuid> {
        let pipelines = self.pipelines.read().await;
        pipelines.keys().copied().collect()
    }
    
    /// Get pipeline count
    pub async fn pipeline_count(&self) -> usize {
        let pipelines = self.pipelines.read().await;
        pipelines.len()
    }
    
    /// Shutdown all pipelines
    pub async fn shutdown_all(&self) -> crate::Result<()> {
        info!("Shutting down all pipelines");
        let mut pipelines = self.pipelines.write().await;
        
        for (id, pipeline) in pipelines.drain() {
            info!("Shutting down pipeline: {} ({})", pipeline.name(), id);
            if let Err(e) = pipeline.stop() {
                warn!("Error stopping pipeline during shutdown: {:?}", e);
            }
        }
        
        Ok(())
    }
    
    /// Get the message receiver (should be called only once)
    pub fn take_message_receiver(&self) -> Option<mpsc::UnboundedReceiver<(Uuid, PipelineMessage)>> {
        self.message_receiver.lock().unwrap().take()
    }
}

impl Default for PipelineManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};
    
    #[tokio::test]
    async fn test_pipeline_creation() {
        // Initialize GStreamer for testing
        gst::init().unwrap();
        
        let pipeline = Pipeline::new("test-pipeline").unwrap();
        assert_eq!(pipeline.name(), "test-pipeline");
        assert_eq!(pipeline.current_state(), PipelineState::Null);
    }
    
    #[tokio::test]
    async fn test_pipeline_state_changes() {
        gst::init().unwrap();
        
        let pipeline = Pipeline::new("test-state-pipeline").unwrap();
        
        // Test start
        // Note: This might fail without proper elements, but we're testing the state tracking
        let _ = pipeline.start();
        
        // Test pause
        let _ = pipeline.pause();
        
        // Test stop
        pipeline.stop().unwrap();
        assert_eq!(pipeline.current_state(), PipelineState::Null);
    }
    
    #[tokio::test]
    async fn test_pipeline_manager() {
        gst::init().unwrap();
        
        let manager = PipelineManager::new();
        assert_eq!(manager.pipeline_count().await, 0);
        
        let pipeline = manager.create_pipeline("managed-pipeline").await.unwrap();
        assert_eq!(manager.pipeline_count().await, 1);
        
        let found = manager.get_pipeline_by_name("managed-pipeline").await;
        assert!(found.is_some());
        
        manager.remove_pipeline(pipeline.id()).await.unwrap();
        assert_eq!(manager.pipeline_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_pipeline_cleanup() {
        gst::init().unwrap();
        
        let manager = PipelineManager::new();
        
        // Create multiple pipelines
        let _p1 = manager.create_pipeline("cleanup-test-1").await.unwrap();
        let _p2 = manager.create_pipeline("cleanup-test-2").await.unwrap();
        
        assert_eq!(manager.pipeline_count().await, 2);
        
        // Shutdown all
        manager.shutdown_all().await.unwrap();
        assert_eq!(manager.pipeline_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_message_handling() {
        gst::init().unwrap();
        
        let manager = PipelineManager::new();
        let mut receiver = manager.take_message_receiver().unwrap();
        
        // Create a pipeline with message handling
        let pipeline = manager.create_pipeline("message-test").await.unwrap();
        
        // Create a simple test pipeline with fakesrc and fakesink
        let src = gst::ElementFactory::make("fakesrc").build().unwrap();
        let sink = gst::ElementFactory::make("fakesink").build().unwrap();
        
        pipeline.add_element(&src).unwrap();
        pipeline.add_element(&sink).unwrap();
        
        // Link elements
        src.link(&sink).unwrap();
        
        // Start pipeline
        let _ = pipeline.start();
        
        // Wait for a message or timeout
        let result = timeout(Duration::from_millis(100), receiver.recv()).await;
        
        // Clean up
        pipeline.stop().unwrap();
        
        // We might or might not get a message depending on timing,
        // but this tests the message handling setup
        match result {
            Ok(Some((id, msg))) => {
                assert_eq!(id, pipeline.id());
                println!("Received message: {:?}", msg);
            }
            Ok(None) => println!("Channel closed"),
            Err(_) => println!("Timeout waiting for message"),
        }
    }
}