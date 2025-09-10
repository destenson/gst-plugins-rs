pub mod common;
pub mod scenarios;
pub mod load_tests;
pub mod failure_injection;
pub mod validation;

use stream_manager::Config;
use std::sync::Arc;

#[cfg(test)]
pub fn init_test_environment() {
    // Initialize GStreamer once for all tests
    gst::init().ok();
    
    // Set up test logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .try_init();
}

#[cfg(test)]
pub fn create_test_config() -> Arc<Config> {
    let mut config = Config::default();
    
    // Configure for testing
    config.api.host = "127.0.0.1".to_string();
    config.api.port = 0; // Use random port
    
    // Disable persistence for tests
    if let Some(ref mut backup) = config.backup {
        backup.enabled = false;
    }
    
    Arc::new(config)
}