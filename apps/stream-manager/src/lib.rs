pub mod config;
pub mod gst_utils;
pub mod pipeline;
pub mod stream;
pub mod recording;
pub mod health;
pub mod api;
pub mod storage;
pub mod metrics;
pub mod database;
pub mod recovery;
pub mod inference;
pub mod rtsp;
pub mod webrtc;
pub mod backup;
pub mod service;
pub mod telemetry;

// Re-export commonly used types
pub use config::{Config, ConfigManager};

// Common error types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StreamManagerError {
    #[error("Stream not found: {0}")]
    StreamNotFound(String),
    
    #[error("Pipeline error: {0}")]
    PipelineError(#[from] gst::glib::Error),
    
    #[error("GStreamer boolean error: {0}")]
    GstBoolError(#[from] gst::glib::BoolError),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, StreamManagerError>;
