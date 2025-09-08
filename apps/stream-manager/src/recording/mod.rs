pub mod branch;

pub use branch::{RecordingBranch, RecordingConfig, RecordingError, MuxerType};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tracing::info;

pub struct RecordingManager {
    base_path: PathBuf,
    recording_branches: Arc<RwLock<HashMap<String, RecordingBranch>>>,
}

impl RecordingManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self { 
            base_path,
            recording_branches: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_recording_branch(
        &self,
        stream_id: &str,
        config: Option<RecordingConfig>,
    ) -> Result<gst::Bin, RecordingError> {
        let config = config.unwrap_or_else(|| {
            let mut cfg = RecordingConfig::default();
            cfg.base_path = self.base_path.clone();
            cfg
        });

        let branch = RecordingBranch::new(stream_id, config)?;
        let bin = branch.get_bin().clone();

        let mut branches = self.recording_branches.write().unwrap();
        branches.insert(stream_id.to_string(), branch);

        info!("Created recording branch for stream: {}", stream_id);
        Ok(bin)
    }

    pub fn start_recording(&self, stream_id: &str) -> Result<(), RecordingError> {
        let branches = self.recording_branches.read().unwrap();
        let branch = branches
            .get(stream_id)
            .ok_or_else(|| RecordingError::InvalidConfig(format!("Stream {} not found", stream_id)))?;
        
        branch.start_recording()
    }

    pub fn stop_recording(&self, stream_id: &str) -> Result<(), RecordingError> {
        let branches = self.recording_branches.read().unwrap();
        let branch = branches
            .get(stream_id)
            .ok_or_else(|| RecordingError::InvalidConfig(format!("Stream {} not found", stream_id)))?;
        
        branch.stop_recording()
    }

    pub fn is_recording(&self, stream_id: &str) -> bool {
        let branches = self.recording_branches.read().unwrap();
        branches
            .get(stream_id)
            .map(|b| b.is_recording())
            .unwrap_or(false)
    }

    pub fn get_current_segment(&self, stream_id: &str) -> Option<u32> {
        let branches = self.recording_branches.read().unwrap();
        branches
            .get(stream_id)
            .map(|b| b.get_current_segment())
    }

    pub fn remove_recording_branch(&self, stream_id: &str) -> Result<(), RecordingError> {
        let mut branches = self.recording_branches.write().unwrap();
        
        if let Some(branch) = branches.remove(stream_id) {
            if branch.is_recording() {
                branch.stop_recording()?;
            }
            info!("Removed recording branch for stream: {}", stream_id);
            Ok(())
        } else {
            Err(RecordingError::InvalidConfig(format!("Stream {} not found", stream_id)))
        }
    }
}