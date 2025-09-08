use crate::config::Config;
use crate::manager::StreamManager;
use std::sync::Arc;

pub async fn create_test_manager() -> StreamManager {
    // Initialize GStreamer for tests
    gst::init().ok();
    
    let config = Arc::new(Config::default());
    StreamManager::new(config).expect("Failed to create test manager")
}