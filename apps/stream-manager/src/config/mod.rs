use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use notify::{Watcher, RecursiveMode, Event};
use tracing::{info, warn, error};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub app: AppConfig,
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub recording: RecordingConfig,
    pub inference: InferenceConfig,
    pub monitoring: MonitoringConfig,
    pub stream_defaults: StreamDefaultConfig,
    pub streams: Vec<StreamConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AppConfig {
    pub name: String,
    pub log_level: String,
    pub max_concurrent_streams: usize,
    pub shutdown_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ServerConfig {
    pub bind_address: String,
    pub rtsp_port: u16,
    pub webrtc_port: u16,
    pub api_port: u16,
    pub websocket_port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct StorageConfig {
    pub base_path: PathBuf,
    pub max_disk_usage_percent: f32,
    pub rotation_enabled: bool,
    pub retention_days: u32,
    pub min_free_space_gb: f32,
    pub check_interval_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct StreamConfig {
    pub id: String,
    pub name: String,
    pub source_uri: String,
    pub enabled: bool,
    pub recording_enabled: bool,
    pub inference_enabled: bool,
    pub reconnect_timeout_seconds: u64,
    pub max_reconnect_attempts: u32,
    pub buffer_size_mb: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct RecordingConfig {
    pub segment_duration_seconds: u64,
    pub format: String,
    pub video_codec: String,
    pub audio_codec: String,
    pub video_bitrate_kbps: u32,
    pub audio_bitrate_kbps: u32,
    pub keyframe_interval: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct InferenceConfig {
    pub enabled: bool,
    pub gpu_enabled: bool,
    pub batch_size: usize,
    pub model_path: Option<PathBuf>,
    pub confidence_threshold: f32,
    pub inference_interval_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct MonitoringConfig {
    pub health_check_interval_seconds: u64,
    pub metrics_enabled: bool,
    pub telemetry_enabled: bool,
    pub prometheus_port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct StreamDefaultConfig {
    pub reconnect_timeout_seconds: u64,
    pub max_reconnect_attempts: u32,
    pub buffer_size_mb: u32,
}

// Default implementations
impl Default for Config {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            recording: RecordingConfig::default(),
            inference: InferenceConfig::default(),
            monitoring: MonitoringConfig::default(),
            stream_defaults: StreamDefaultConfig::default(),
            streams: Vec::new(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "Stream Manager".to_string(),
            log_level: "info".to_string(),
            max_concurrent_streams: 10,
            shutdown_timeout_seconds: 30,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            rtsp_port: 8554,
            webrtc_port: 8555,
            api_port: 8080,
            websocket_port: 8081,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from("/var/lib/stream-manager/recordings"),
            max_disk_usage_percent: 80.0,
            rotation_enabled: true,
            retention_days: 7,
            min_free_space_gb: 10.0,
            check_interval_seconds: 60,
        }
    }
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            segment_duration_seconds: 300, // 5 minutes
            format: "mp4".to_string(),
            video_codec: "h264".to_string(),
            audio_codec: "aac".to_string(),
            video_bitrate_kbps: 2000,
            audio_bitrate_kbps: 128,
            keyframe_interval: 30,
        }
    }
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            gpu_enabled: false,
            batch_size: 1,
            model_path: None,
            confidence_threshold: 0.5,
            inference_interval_ms: 100,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            health_check_interval_seconds: 10,
            metrics_enabled: true,
            telemetry_enabled: false,
            prometheus_port: 9090,
        }
    }
}

impl Default for StreamDefaultConfig {
    fn default() -> Self {
        Self {
            reconnect_timeout_seconds: 5,
            max_reconnect_attempts: 10,
            buffer_size_mb: 50,
        }
    }
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            source_uri: String::new(),
            enabled: true,
            recording_enabled: true,
            inference_enabled: false,
            reconnect_timeout_seconds: 5,
            max_reconnect_attempts: 10,
            buffer_size_mb: 50,
        }
    }
}

impl Config {
    pub async fn from_file(path: &PathBuf) -> crate::Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| crate::StreamManagerError::ConfigError(e.to_string()))?;
        
        config.validate()?;
        Ok(config)
    }
    
    pub fn validate(&self) -> crate::Result<()> {
        // Validate storage path
        if !self.storage.base_path.exists() {
            warn!("Storage path does not exist: {:?}", self.storage.base_path);
        }
        
        // Validate disk usage percentage
        if self.storage.max_disk_usage_percent > 95.0 || self.storage.max_disk_usage_percent < 10.0 {
            return Err(crate::StreamManagerError::ConfigError(
                "max_disk_usage_percent must be between 10 and 95".to_string()
            ));
        }
        
        // Validate ports
        let ports = vec![
            self.server.rtsp_port,
            self.server.webrtc_port,
            self.server.api_port,
            self.server.websocket_port,
            self.monitoring.prometheus_port,
        ];
        
        for (i, port) in ports.iter().enumerate() {
            if *port == 0 {
                return Err(crate::StreamManagerError::ConfigError(
                    format!("Invalid port configuration: port {} cannot be 0", i)
                ));
            }
        }
        
        // Check for duplicate stream IDs
        let mut stream_ids = std::collections::HashSet::new();
        for stream in &self.streams {
            if !stream_ids.insert(&stream.id) {
                return Err(crate::StreamManagerError::ConfigError(
                    format!("Duplicate stream ID: {}", stream.id)
                ));
            }
        }
        
        Ok(())
    }
    
    pub fn merge(&mut self, partial: PartialConfig) {
        if let Some(app) = partial.app {
            self.app = app;
        }
        if let Some(server) = partial.server {
            self.server = server;
        }
        if let Some(storage) = partial.storage {
            self.storage = storage;
        }
        if let Some(recording) = partial.recording {
            self.recording = recording;
        }
        if let Some(inference) = partial.inference {
            self.inference = inference;
        }
        if let Some(monitoring) = partial.monitoring {
            self.monitoring = monitoring;
        }
        if let Some(streams) = partial.streams {
            self.streams = streams;
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartialConfig {
    pub app: Option<AppConfig>,
    pub server: Option<ServerConfig>,
    pub storage: Option<StorageConfig>,
    pub recording: Option<RecordingConfig>,
    pub inference: Option<InferenceConfig>,
    pub monitoring: Option<MonitoringConfig>,
    pub streams: Option<Vec<StreamConfig>>,
}

pub struct ConfigManager {
    config: Arc<RwLock<Config>>,
    config_path: PathBuf,
    watcher: Option<notify::RecommendedWatcher>,
}

impl ConfigManager {
    pub async fn new(config_path: PathBuf) -> crate::Result<Self> {
        let config = Config::from_file(&config_path).await?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
            watcher: None,
        })
    }
    
    pub async fn get(&self) -> Config {
        self.config.read().await.clone()
    }
    
    pub async fn reload(&self) -> crate::Result<()> {
        info!("Reloading configuration from {:?}", self.config_path);
        let new_config = Config::from_file(&self.config_path).await?;
        *self.config.write().await = new_config;
        info!("Configuration reloaded successfully");
        Ok(())
    }
    
    pub async fn update_partial(&self, partial: PartialConfig) -> crate::Result<()> {
        let mut config = self.config.write().await;
        config.merge(partial);
        config.validate()?;
        Ok(())
    }
    
    pub async fn start_watching(&mut self) -> crate::Result<()> {
        let config_path = self.config_path.clone();
        let config_arc = self.config.clone();
        
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if event.paths.iter().any(|p| p == &config_path) {
                    let _ = tx.blocking_send(());
                }
            }
        }).map_err(|e| crate::StreamManagerError::ConfigError(e.to_string()))?;
        
        watcher.watch(&self.config_path, RecursiveMode::NonRecursive)
            .map_err(|e| crate::StreamManagerError::ConfigError(e.to_string()))?;
        
        self.watcher = Some(watcher);
        
        let config_path = self.config_path.clone();
        tokio::spawn(async move {
            while rx.recv().await.is_some() {
                info!("Configuration file changed, reloading...");
                match Config::from_file(&config_path).await {
                    Ok(new_config) => {
                        *config_arc.write().await = new_config;
                        info!("Configuration reloaded successfully");
                    }
                    Err(e) => {
                        error!("Failed to reload configuration: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[tokio::test]
    async fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.app.name, "Stream Manager");
        assert_eq!(config.server.api_port, 8080);
    }
    
    #[tokio::test]
    async fn test_config_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"
[app]
name = "Test Manager"
log_level = "debug"

[server]
bind_address = "127.0.0.1"
api_port = 9090
        "#).unwrap();
        
        let config = Config::from_file(&temp_file.path().to_path_buf()).await.unwrap();
        assert_eq!(config.app.name, "Test Manager");
        assert_eq!(config.server.api_port, 9090);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());
        
        // Test invalid disk usage
        config.storage.max_disk_usage_percent = 99.0;
        assert!(config.validate().is_err());
        
        config.storage.max_disk_usage_percent = 80.0;
        
        // Test duplicate stream IDs
        config.streams.push(StreamConfig {
            id: "stream1".to_string(),
            ..Default::default()
        });
        config.streams.push(StreamConfig {
            id: "stream1".to_string(),
            ..Default::default()
        });
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_merge() {
        let mut config = Config::default();
        let partial = PartialConfig {
            app: Some(AppConfig {
                name: "Updated".to_string(),
                ..Default::default()
            }),
            server: None,
            storage: None,
            recording: None,
            inference: None,
            monitoring: None,
            streams: None,
        };
        
        config.merge(partial);
        assert_eq!(config.app.name, "Updated");
    }
}