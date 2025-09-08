use gst::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct Pipeline {
    pipeline: gst::Pipeline,
    name: String,
}

impl Pipeline {
    pub fn new(name: &str) -> crate::Result<Self> {
        let pipeline = gst::Pipeline::builder().name(name).build();
        
        Ok(Self {
            pipeline,
            name: name.to_string(),
        })
    }
    
    pub fn add_element(&self, element: &gst::Element) -> crate::Result<()> {
        self.pipeline.add(element)?;
        Ok(())
    }
    
    pub fn set_state(&self, state: gst::State) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
        self.pipeline.set_state(state)
    }
    
    pub fn get_state(&self) -> gst::State {
        self.pipeline.state(gst::ClockTime::ZERO).1
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct PipelineManager {
    pipelines: Arc<RwLock<Vec<Arc<Pipeline>>>>,
}

impl PipelineManager {
    pub fn new() -> Self {
        Self {
            pipelines: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn create_pipeline(&self, name: &str) -> crate::Result<Arc<Pipeline>> {
        let pipeline = Arc::new(Pipeline::new(name)?);
        let mut pipelines = self.pipelines.write().await;
        pipelines.push(pipeline.clone());
        info!("Created pipeline: {}", name);
        Ok(pipeline)
    }
    
    pub async fn get_pipeline(&self, name: &str) -> Option<Arc<Pipeline>> {
        let pipelines = self.pipelines.read().await;
        pipelines.iter()
            .find(|p| p.name() == name)
            .cloned()
    }
    
    pub async fn remove_pipeline(&self, name: &str) -> crate::Result<()> {
        let mut pipelines = self.pipelines.write().await;
        pipelines.retain(|p| p.name() != name);
        info!("Removed pipeline: {}", name);
        Ok(())
    }
}