use gst::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Debug, Error)]
pub enum BranchingError {
    #[error("Failed to create element: {0}")]
    ElementCreation(String),
    #[error("Failed to link elements")]
    LinkError,
    #[error("Failed to get request pad")]
    RequestPadError,
    #[error("Branch not found: {0}")]
    BranchNotFound(String),
    #[error("Pipeline error")]
    PipelineError(#[from] gst::StateChangeError),
    #[error("Boolean error")]
    BoolError(#[from] gst::glib::BoolError),
    #[error("State change failed")]
    StateChangeError,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamBranch {
    Recording,
    Inference,
    Preview,
    Custom(String),
}

impl StreamBranch {
    pub fn as_str(&self) -> &str {
        match self {
            StreamBranch::Recording => "recording",
            StreamBranch::Inference => "inference",
            StreamBranch::Preview => "preview",
            StreamBranch::Custom(name) => name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub max_size_bytes: u32,
    pub max_size_buffers: u32,
    pub max_size_time: gst::ClockTime,
    pub leaky: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 0, // 0 means unlimited
            max_size_buffers: 200,
            max_size_time: gst::ClockTime::from_seconds(1),
            leaky: false,
        }
    }
}

impl QueueConfig {
    pub fn for_recording() -> Self {
        Self {
            max_size_bytes: 0,
            max_size_buffers: 500,
            max_size_time: gst::ClockTime::from_seconds(5),
            leaky: false,
        }
    }

    pub fn for_inference() -> Self {
        Self {
            max_size_bytes: 0,
            max_size_buffers: 5,
            max_size_time: gst::ClockTime::from_mseconds(100),
            leaky: true, // Drop old buffers if inference is slow
        }
    }

    pub fn for_preview() -> Self {
        Self {
            max_size_bytes: 0,
            max_size_buffers: 2,
            max_size_time: gst::ClockTime::from_mseconds(50),
            leaky: true, // Drop old buffers for smooth preview
        }
    }
}

pub struct BranchInfo {
    pub branch_type: StreamBranch,
    pub queue: gst::Element,
    pub tee_pad: gst::Pad,
    pub queue_sink_pad: gst::Pad,
}

#[derive(Clone)]
pub struct BranchManager {
    tee: gst::Element,
    pipeline: gst::Pipeline,
    branches: Arc<RwLock<HashMap<String, BranchInfo>>>,
}

impl BranchManager {
    pub fn new(pipeline: &gst::Pipeline) -> Result<Self, BranchingError> {
        let tee = gst::ElementFactory::make("tee")
            .name("stream-tee")
            .property("allow-not-linked", true)
            .build()
            .map_err(|_| BranchingError::ElementCreation("tee".to_string()))?;

        pipeline.add(&tee).map_err(|_| {
            BranchingError::ElementCreation("Failed to add tee to pipeline".to_string())
        })?;

        Ok(Self {
            tee,
            pipeline: pipeline.clone(),
            branches: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn create_branch(
        &self,
        branch_type: StreamBranch,
    ) -> Result<gst::Element, BranchingError> {
        let branch_name = format!("branch-{}", branch_type.as_str());
        
        // Check if branch already exists
        {
            let branches = self.branches.read().unwrap();
            if branches.contains_key(&branch_name) {
                warn!("Branch {} already exists", branch_name);
                return Ok(branches.get(&branch_name).unwrap().queue.clone());
            }
        }

        info!("Creating branch: {}", branch_name);

        // Select queue configuration based on branch type
        let queue_config = match &branch_type {
            StreamBranch::Recording => QueueConfig::for_recording(),
            StreamBranch::Inference => QueueConfig::for_inference(),
            StreamBranch::Preview => QueueConfig::for_preview(),
            StreamBranch::Custom(_) => QueueConfig::default(),
        };

        // Create queue element
        let queue = gst::ElementFactory::make("queue")
            .name(&format!("queue-{}", branch_type.as_str()))
            .property("max-size-bytes", queue_config.max_size_bytes)
            .property("max-size-buffers", queue_config.max_size_buffers)
            .property("max-size-time", queue_config.max_size_time.nseconds())
            .build()
            .map_err(|_| BranchingError::ElementCreation("queue".to_string()))?;

        if queue_config.leaky {
            queue.set_property_from_str("leaky", "downstream");
        }

        // Add queue to pipeline
        self.pipeline.add(&queue).map_err(|_| {
            BranchingError::ElementCreation("Failed to add queue to pipeline".to_string())
        })?;

        // Get request pad from tee
        let tee_pad = self
            .tee
            .request_pad_simple("src_%u")
            .ok_or(BranchingError::RequestPadError)?;

        // Get queue sink pad
        let queue_sink_pad = queue
            .static_pad("sink")
            .ok_or(BranchingError::RequestPadError)?;

        // Link tee src pad to queue sink pad
        tee_pad.link(&queue_sink_pad).map_err(|_| {
            error!("Failed to link tee to queue for branch {}", branch_name);
            BranchingError::LinkError
        })?;

        // Sync queue state with parent
        queue
            .sync_state_with_parent()
            .map_err(|_| BranchingError::StateChangeError)?;

        // Store branch info
        {
            let mut branches = self.branches.write().unwrap();
            branches.insert(
                branch_name.clone(),
                BranchInfo {
                    branch_type,
                    queue: queue.clone(),
                    tee_pad,
                    queue_sink_pad,
                },
            );
        }

        info!("Successfully created branch: {}", branch_name);
        Ok(queue)
    }

    pub fn remove_branch(&self, branch_type: &StreamBranch) -> Result<(), BranchingError> {
        let branch_name = format!("branch-{}", branch_type.as_str());
        
        info!("Removing branch: {}", branch_name);

        let branch_info = {
            let mut branches = self.branches.write().unwrap();
            branches
                .remove(&branch_name)
                .ok_or_else(|| BranchingError::BranchNotFound(branch_name.clone()))?
        };

        // Set queue to NULL state
        branch_info
            .queue
            .set_state(gst::State::Null)
            .map_err(|_| BranchingError::StateChangeError)?;

        // Unlink tee from queue
        let _ = branch_info.tee_pad.unlink(&branch_info.queue_sink_pad);

        // Release request pad
        self.tee.release_request_pad(&branch_info.tee_pad);

        // Remove queue from pipeline
        self.pipeline.remove(&branch_info.queue).map_err(|_| {
            BranchingError::ElementCreation("Failed to remove queue from pipeline".to_string())
        })?;

        info!("Successfully removed branch: {}", branch_name);
        Ok(())
    }

    pub fn get_branch_queue(&self, branch_type: &StreamBranch) -> Option<gst::Element> {
        let branch_name = format!("branch-{}", branch_type.as_str());
        let branches = self.branches.read().unwrap();
        branches.get(&branch_name).map(|info| info.queue.clone())
    }

    pub fn list_branches(&self) -> Vec<StreamBranch> {
        let branches = self.branches.read().unwrap();
        branches
            .values()
            .map(|info| info.branch_type.clone())
            .collect()
    }

    pub fn get_tee(&self) -> &gst::Element {
        &self.tee
    }

    pub fn connect_to_source(&self, source_element: &gst::Element) -> Result<(), BranchingError> {
        // Link source element to tee
        source_element.link(&self.tee).map_err(|_| {
            error!("Failed to link source to tee");
            BranchingError::LinkError
        })?;

        debug!("Connected source to tee");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        let _ = gst::init();
        let _ = tracing_subscriber::fmt()
            .with_env_filter("stream_manager=debug")
            .try_init();
    }

    #[test]
    fn test_branch_creation() {
        init();

        let pipeline = gst::Pipeline::new();
        let manager = BranchManager::new(&pipeline).expect("Failed to create BranchManager");

        // Create recording branch
        let queue = manager
            .create_branch(StreamBranch::Recording)
            .expect("Failed to create recording branch");

        assert_eq!(queue.name(), "queue-recording");

        // Verify branch is in list
        let branches = manager.list_branches();
        assert_eq!(branches.len(), 1);
        assert!(branches.contains(&StreamBranch::Recording));
    }

    #[test]
    fn test_multiple_branches() {
        init();

        let pipeline = gst::Pipeline::new();
        let manager = BranchManager::new(&pipeline).expect("Failed to create BranchManager");

        // Create multiple branches
        manager
            .create_branch(StreamBranch::Recording)
            .expect("Failed to create recording branch");
        manager
            .create_branch(StreamBranch::Inference)
            .expect("Failed to create inference branch");
        manager
            .create_branch(StreamBranch::Preview)
            .expect("Failed to create preview branch");

        // Verify all branches exist
        let branches = manager.list_branches();
        assert_eq!(branches.len(), 3);
        assert!(branches.contains(&StreamBranch::Recording));
        assert!(branches.contains(&StreamBranch::Inference));
        assert!(branches.contains(&StreamBranch::Preview));
    }

    #[test]
    fn test_branch_removal() {
        init();

        let pipeline = gst::Pipeline::new();
        let manager = BranchManager::new(&pipeline).expect("Failed to create BranchManager");

        // Create and remove branch
        manager
            .create_branch(StreamBranch::Recording)
            .expect("Failed to create recording branch");

        assert_eq!(manager.list_branches().len(), 1);

        manager
            .remove_branch(&StreamBranch::Recording)
            .expect("Failed to remove recording branch");

        assert_eq!(manager.list_branches().len(), 0);
    }

    #[test]
    fn test_branch_cleanup() {
        init();

        let pipeline = gst::Pipeline::new();
        let manager = BranchManager::new(&pipeline).expect("Failed to create BranchManager");

        // Create multiple branches
        manager
            .create_branch(StreamBranch::Recording)
            .expect("Failed to create recording branch");
        manager
            .create_branch(StreamBranch::Inference)
            .expect("Failed to create inference branch");

        // Remove one branch
        manager
            .remove_branch(&StreamBranch::Recording)
            .expect("Failed to remove recording branch");

        // Verify only one branch remains
        let branches = manager.list_branches();
        assert_eq!(branches.len(), 1);
        assert!(branches.contains(&StreamBranch::Inference));
        assert!(!branches.contains(&StreamBranch::Recording));
    }

    #[test]
    fn test_duplicate_branch_creation() {
        init();

        let pipeline = gst::Pipeline::new();
        let manager = BranchManager::new(&pipeline).expect("Failed to create BranchManager");

        // Create branch
        let queue1 = manager
            .create_branch(StreamBranch::Recording)
            .expect("Failed to create recording branch");

        // Try to create same branch again - should return existing queue
        let queue2 = manager
            .create_branch(StreamBranch::Recording)
            .expect("Failed to get existing recording branch");

        assert_eq!(queue1.name(), queue2.name());
        assert_eq!(manager.list_branches().len(), 1);
    }

    #[test]
    fn test_custom_branch() {
        init();

        let pipeline = gst::Pipeline::new();
        let manager = BranchManager::new(&pipeline).expect("Failed to create BranchManager");

        // Create custom branch
        let custom_branch = StreamBranch::Custom("analytics".to_string());
        let queue = manager
            .create_branch(custom_branch.clone())
            .expect("Failed to create custom branch");

        assert_eq!(queue.name(), "queue-analytics");

        // Verify branch is in list
        let branches = manager.list_branches();
        assert_eq!(branches.len(), 1);
        assert!(branches.contains(&custom_branch));
    }

    #[test]
    fn test_queue_configurations() {
        init();

        let pipeline = gst::Pipeline::new();
        let manager = BranchManager::new(&pipeline).expect("Failed to create BranchManager");

        // Create branches with different configurations
        let recording_queue = manager
            .create_branch(StreamBranch::Recording)
            .expect("Failed to create recording branch");

        let inference_queue = manager
            .create_branch(StreamBranch::Inference)
            .expect("Failed to create inference branch");

        let preview_queue = manager
            .create_branch(StreamBranch::Preview)
            .expect("Failed to create preview branch");

        // Just verify that queues were created with the right names
        assert_eq!(recording_queue.name(), "queue-recording");
        assert_eq!(inference_queue.name(), "queue-inference");
        assert_eq!(preview_queue.name(), "queue-preview");
    }

    #[test]
    fn test_source_connection() {
        init();

        let pipeline = gst::Pipeline::new();
        let manager = BranchManager::new(&pipeline).expect("Failed to create BranchManager");

        // Create a test source
        let source = gst::ElementFactory::make("videotestsrc")
            .name("test-source")
            .build()
            .expect("Failed to create videotestsrc");

        pipeline
            .add(&source)
            .expect("Failed to add source to pipeline");

        // Connect source to tee
        manager
            .connect_to_source(&source)
            .expect("Failed to connect source to tee");

        // Verify connection by checking if elements are linked
        let source_src_pad = source.static_pad("src").unwrap();
        assert!(source_src_pad.is_linked());
    }
}