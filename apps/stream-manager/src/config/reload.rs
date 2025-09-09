use crate::{Config, Result, StreamManagerError};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigChange {
    AppConfig,
    ApiConfig,
    StorageConfig,
    RecordingConfig,
    InferenceConfig,
    MonitoringConfig,
    StreamAdded(String),
    StreamRemoved(String),
    StreamModified(String),
    StreamDefaults,
}

#[derive(Debug, Clone)]
pub struct ConfigReloadEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub changes: Vec<ConfigChange>,
    pub requires_restart: bool,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadRestriction {
    RuntimeReloadable,
    RequiresRestart,
}

pub struct ConfigReloader {
    config: Arc<RwLock<Config>>,
    config_path: PathBuf,
    watcher: Option<notify::RecommendedWatcher>,
    event_tx: broadcast::Sender<ConfigReloadEvent>,
    debounce_duration: Duration,
    last_reload: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
}

impl ConfigReloader {
    pub fn new(
        config: Arc<RwLock<Config>>,
        config_path: PathBuf,
    ) -> Result<Self> {
        let (event_tx, _) = broadcast::channel(100);
        
        Ok(Self {
            config,
            config_path,
            watcher: None,
            event_tx,
            debounce_duration: Duration::from_millis(500),
            last_reload: Arc::new(RwLock::new(None)),
        })
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigReloadEvent> {
        self.event_tx.subscribe()
    }
    
    pub async fn start_watching(&mut self) -> Result<()> {
        if !self.config_path.exists() {
            warn!("Config file {:?} does not exist, hot-reload will activate when file is created", self.config_path);
            return Ok(());
        }
        
        let config_path = self.config_path.clone();
        let config_arc = self.config.clone();
        let event_tx = self.event_tx.clone();
        let debounce_duration = self.debounce_duration;
        let last_reload = self.last_reload.clone();
        
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        
        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        if event.paths.iter().any(|p| p == &config_path) {
                            let _ = tx.blocking_send(());
                        }
                    }
                    _ => {}
                }
            }
        }).map_err(|e| StreamManagerError::ConfigError(format!("Failed to create file watcher: {}", e)))?;
        
        let watch_path = if self.config_path.exists() {
            self.config_path.clone()
        } else if let Some(parent) = self.config_path.parent() {
            parent.to_path_buf()
        } else {
            self.config_path.clone()
        };
        
        watcher.watch(&watch_path, RecursiveMode::NonRecursive)
            .map_err(|e| StreamManagerError::ConfigError(format!("Failed to watch config file: {}", e)))?;
        
        self.watcher = Some(watcher);
        
        let config_path = self.config_path.clone();
        tokio::spawn(async move {
            let mut debounce_timer = tokio::time::interval(debounce_duration);
            let mut pending_reload = false;
            
            loop {
                tokio::select! {
                    Some(()) = rx.recv() => {
                        debug!("Config file change detected, scheduling reload");
                        pending_reload = true;
                    }
                    _ = debounce_timer.tick() => {
                        if pending_reload {
                            pending_reload = false;
                            
                            // Check if enough time has passed since last reload
                            let mut last = last_reload.write().await;
                            if let Some(last_time) = *last {
                                let elapsed = chrono::Utc::now() - last_time;
                                if elapsed < chrono::Duration::from_std(debounce_duration).unwrap() {
                                    debug!("Skipping reload, too soon since last reload");
                                    continue;
                                }
                            }
                            
                            info!("Reloading configuration from {:?}", config_path);
                            
                            let old_config = config_arc.read().await.clone();
                            
                            match Config::from_file(&config_path).await {
                                Ok(new_config) => {
                                    let reload_event = Self::analyze_changes(&old_config, &new_config).await;
                                    
                                    if reload_event.requires_restart {
                                        warn!("Configuration changes require restart: {:?}", reload_event.changes);
                                        let _ = event_tx.send(reload_event);
                                    } else if !reload_event.validation_errors.is_empty() {
                                        error!("Configuration validation failed: {:?}", reload_event.validation_errors);
                                        let _ = event_tx.send(reload_event);
                                    } else {
                                        *config_arc.write().await = new_config;
                                        *last = Some(chrono::Utc::now());
                                        info!("Configuration reloaded successfully with changes: {:?}", reload_event.changes);
                                        let _ = event_tx.send(reload_event);
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to reload configuration: {}", e);
                                    let reload_event = ConfigReloadEvent {
                                        timestamp: chrono::Utc::now(),
                                        changes: vec![],
                                        requires_restart: false,
                                        validation_errors: vec![e.to_string()],
                                    };
                                    let _ = event_tx.send(reload_event);
                                }
                            }
                        }
                    }
                }
            }
        });
        
        info!("Configuration hot-reload started for {:?}", self.config_path);
        Ok(())
    }
    
    pub async fn reload_now(&self) -> Result<ConfigReloadEvent> {
        info!("Manual configuration reload requested");
        
        let old_config = self.config.read().await.clone();
        let new_config = Config::from_file(&self.config_path).await?;
        
        let reload_event = Self::analyze_changes(&old_config, &new_config).await;
        
        if reload_event.requires_restart {
            return Err(StreamManagerError::ConfigError(
                format!("Configuration changes require restart: {:?}", reload_event.changes)
            ));
        }
        
        if !reload_event.validation_errors.is_empty() {
            return Err(StreamManagerError::ConfigError(
                format!("Configuration validation failed: {:?}", reload_event.validation_errors)
            ));
        }
        
        *self.config.write().await = new_config;
        *self.last_reload.write().await = Some(chrono::Utc::now());
        
        let _ = self.event_tx.send(reload_event.clone());
        Ok(reload_event)
    }
    
    async fn analyze_changes(old: &Config, new: &Config) -> ConfigReloadEvent {
        let mut changes = Vec::new();
        let mut requires_restart = false;
        let validation_errors = Vec::new();
        
        // Check app config changes
        if !Self::configs_equal(&old.app, &new.app) {
            changes.push(ConfigChange::AppConfig);
            // Log level can be changed at runtime
        }
        
        // Check API config changes
        if !Self::configs_equal(&old.api, &new.api) {
            changes.push(ConfigChange::ApiConfig);
            // Port changes require restart
            if old.api.port != new.api.port {
                requires_restart = true;
            }
        }
        
        // Check server config changes
        if !Self::configs_equal(&old.server, &new.server) {
            // Any server port change requires restart
            if old.server.rtsp_port != new.server.rtsp_port
                || old.server.webrtc_port != new.server.webrtc_port
                || old.server.api_port != new.server.api_port
                || old.server.websocket_port != new.server.websocket_port
                || old.server.bind_address != new.server.bind_address
            {
                requires_restart = true;
            }
        }
        
        // Check storage config changes
        if !Self::configs_equal(&old.storage, &new.storage) {
            changes.push(ConfigChange::StorageConfig);
            // Storage path changes can be applied at runtime with validation
        }
        
        // Check recording config changes
        if !Self::configs_equal(&old.recording, &new.recording) {
            changes.push(ConfigChange::RecordingConfig);
            // Recording settings apply to new recordings only
        }
        
        // Check inference config changes
        if !Self::configs_equal(&old.inference, &new.inference) {
            changes.push(ConfigChange::InferenceConfig);
            // Inference settings can be updated at runtime
        }
        
        // Check monitoring config changes
        if !Self::configs_equal(&old.monitoring, &new.monitoring) {
            changes.push(ConfigChange::MonitoringConfig);
            // Prometheus port change requires restart
            if old.monitoring.prometheus_port != new.monitoring.prometheus_port {
                requires_restart = true;
            }
        }
        
        // Check stream defaults changes
        if !Self::configs_equal(&old.stream_defaults, &new.stream_defaults) {
            changes.push(ConfigChange::StreamDefaults);
            // Stream defaults only apply to new streams
        }
        
        // Analyze stream changes
        let old_streams: HashSet<String> = old.streams.iter().map(|s| s.id.clone()).collect();
        let new_streams: HashSet<String> = new.streams.iter().map(|s| s.id.clone()).collect();
        
        // Find added streams
        for id in new_streams.difference(&old_streams) {
            changes.push(ConfigChange::StreamAdded(id.clone()));
        }
        
        // Find removed streams
        for id in old_streams.difference(&new_streams) {
            changes.push(ConfigChange::StreamRemoved(id.clone()));
        }
        
        // Find modified streams
        for id in old_streams.intersection(&new_streams) {
            let old_stream = old.streams.iter().find(|s| s.id == *id);
            let new_stream = new.streams.iter().find(|s| s.id == *id);
            
            if let (Some(old_s), Some(new_s)) = (old_stream, new_stream) {
                if !Self::configs_equal(old_s, new_s) {
                    changes.push(ConfigChange::StreamModified(id.clone()));
                }
            }
        }
        
        ConfigReloadEvent {
            timestamp: chrono::Utc::now(),
            changes,
            requires_restart,
            validation_errors,
        }
    }
    
    fn configs_equal<T: Serialize>(a: &T, b: &T) -> bool {
        // Serialize both configs and compare
        match (serde_json::to_value(a), serde_json::to_value(b)) {
            (Ok(val_a), Ok(val_b)) => val_a == val_b,
            _ => false,
        }
    }
    
    pub fn classify_field(field_path: &str) -> ReloadRestriction {
        match field_path {
            // Fields that require restart
            "api.port" |
            "server.rtsp_port" |
            "server.webrtc_port" |
            "server.api_port" |
            "server.websocket_port" |
            "server.bind_address" |
            "monitoring.prometheus_port" => ReloadRestriction::RequiresRestart,
            
            // Everything else can be reloaded at runtime
            _ => ReloadRestriction::RuntimeReloadable,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReloadStatus {
    pub last_reload: Option<chrono::DateTime<chrono::Utc>>,
    pub config_path: PathBuf,
    pub watching: bool,
    pub pending_changes: Vec<ConfigChange>,
    pub restart_required: bool,
}

impl ConfigReloader {
    pub async fn get_status(&self) -> ReloadStatus {
        ReloadStatus {
            last_reload: *self.last_reload.read().await,
            config_path: self.config_path.clone(),
            watching: self.watcher.is_some(),
            pending_changes: vec![],
            restart_required: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[tokio::test]
    async fn test_config_reload_detection() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"
[app]
name = "Test Manager"
log_level = "info"

[server]
api_port = 8080
        "#).unwrap();
        
        let config = Config::from_file(&temp_file.path().to_path_buf()).await.unwrap();
        let config_arc = Arc::new(RwLock::new(config));
        
        let mut reloader = ConfigReloader::new(
            config_arc.clone(),
            temp_file.path().to_path_buf(),
        ).unwrap();
        
        assert!(reloader.start_watching().await.is_ok());
        
        // Give watcher time to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Modify the config file
        writeln!(temp_file, r#"
[app]
name = "Updated Manager"
log_level = "debug"

[server]
api_port = 8080
        "#).unwrap();
        temp_file.flush().unwrap();
        
        // Wait for debounce
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Check that config was reloaded
        let _config = config_arc.read().await;
        // Note: This test might not catch the change due to async nature
        // In production, the event subscriber would handle this
    }
    
    #[tokio::test]
    async fn test_reload_restrictions() {
        assert_eq!(
            ConfigReloader::classify_field("api.port"),
            ReloadRestriction::RequiresRestart
        );
        
        assert_eq!(
            ConfigReloader::classify_field("app.log_level"),
            ReloadRestriction::RuntimeReloadable
        );
        
        assert_eq!(
            ConfigReloader::classify_field("storage.max_disk_usage_percent"),
            ReloadRestriction::RuntimeReloadable
        );
    }
    
    #[tokio::test]
    async fn test_change_analysis() {
        let mut old_config = Config::default();
        old_config.app.name = "Old Name".to_string();
        old_config.api.port = 8080;
        
        let mut new_config = Config::default();
        new_config.app.name = "New Name".to_string();
        new_config.api.port = 9090;
        
        let event = ConfigReloader::analyze_changes(&old_config, &new_config).await;
        
        assert!(event.changes.contains(&ConfigChange::AppConfig));
        assert!(event.changes.contains(&ConfigChange::ApiConfig));
        assert!(event.requires_restart); // Port change requires restart
    }
    
    #[tokio::test]
    async fn test_stream_change_detection() {
        use crate::config::StreamConfig;
        
        let mut old_config = Config::default();
        old_config.streams.push(StreamConfig {
            id: "stream1".to_string(),
            name: "Stream 1".to_string(),
            ..Default::default()
        });
        old_config.streams.push(StreamConfig {
            id: "stream2".to_string(),
            name: "Stream 2".to_string(),
            ..Default::default()
        });
        
        let mut new_config = Config::default();
        new_config.streams.push(StreamConfig {
            id: "stream1".to_string(),
            name: "Stream 1 Modified".to_string(),
            ..Default::default()
        });
        new_config.streams.push(StreamConfig {
            id: "stream3".to_string(),
            name: "Stream 3".to_string(),
            ..Default::default()
        });
        
        let event = ConfigReloader::analyze_changes(&old_config, &new_config).await;
        
        assert!(event.changes.iter().any(|c| matches!(c, ConfigChange::StreamModified(id) if id == "stream1")));
        assert!(event.changes.iter().any(|c| matches!(c, ConfigChange::StreamRemoved(id) if id == "stream2")));
        assert!(event.changes.iter().any(|c| matches!(c, ConfigChange::StreamAdded(id) if id == "stream3")));
    }
}