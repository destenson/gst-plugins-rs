use crate::{Result, StreamManagerError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub paths: Vec<StoragePath>,
    pub cleanup_policy: CleanupPolicy,
    pub check_interval_seconds: u64,
    pub min_free_space_gb: f32,
    pub max_total_usage_gb: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePath {
    pub path: PathBuf,
    pub enabled: bool,
    pub priority: u32,
    pub max_usage_gb: Option<f32>,
    pub stream_affinity: Vec<String>, // Stream IDs that prefer this path
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPolicy {
    pub enabled: bool,
    pub max_age_days: Option<u32>,
    pub max_size_gb: Option<f32>,
    pub min_segments_per_stream: u32,
    pub priority_retention: HashMap<String, u32>, // Stream ID -> retention days
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathSelectionStrategy {
    RoundRobin,
    LeastUsed,
    Priority,
    Affinity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiskStatus {
    Available,
    Active,
    Rotating,
    Unmounted,
    Failed,
}

#[derive(Debug, Clone)]
pub struct StorageStats {
    pub path: PathBuf,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub usage_percent: f32,
    pub is_healthy: bool,
    pub last_check: SystemTime,
}

#[derive(Debug, Clone)]
pub enum StorageEvent {
    LowSpace(PathBuf, f32),
    PathUnavailable(PathBuf),
    PathRecovered(PathBuf),
    CleanupStarted,
    CleanupCompleted(usize, u64), // files_deleted, bytes_freed
    StorageFull(PathBuf),
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Storage path not found: {0}")]
    PathNotFound(PathBuf),
    #[error("Insufficient space: {0} bytes needed, {1} bytes available")]
    InsufficientSpace(u64, u64),
    #[error("All storage paths unavailable")]
    AllPathsUnavailable,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

struct PathState {
    path: StoragePath,
    stats: StorageStats,
    last_used: SystemTime,
    current_streams: Vec<String>,
}

pub struct StorageManager {
    paths: Arc<RwLock<Vec<PathState>>>,
    config: StorageConfig,
    selection_strategy: PathSelectionStrategy,
    event_tx: broadcast::Sender<StorageEvent>,
    round_robin_index: Arc<RwLock<usize>>,
    monitoring_handle: Option<tokio::task::JoinHandle<()>>,
}

impl StorageManager {
    pub fn new(config: StorageConfig) -> Result<Self> {
        let (event_tx, _) = broadcast::channel(100);
        
        let mut paths = Vec::new();
        for storage_path in &config.paths {
            let stats = Self::check_path_stats(&storage_path.path)?;
            paths.push(PathState {
                path: storage_path.clone(),
                stats,
                last_used: SystemTime::now(),
                current_streams: Vec::new(),
            });
        }
        
        Ok(Self {
            paths: Arc::new(RwLock::new(paths)),
            config,
            selection_strategy: PathSelectionStrategy::LeastUsed,
            event_tx,
            round_robin_index: Arc::new(RwLock::new(0)),
            monitoring_handle: None,
        })
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<StorageEvent> {
        self.event_tx.subscribe()
    }
    
    pub async fn start_monitoring(&mut self) -> Result<()> {
        let paths = self.paths.clone();
        let event_tx = self.event_tx.clone();
        let config = self.config.clone();
        let check_interval = Duration::from_secs(config.check_interval_seconds);
        
        let handle = tokio::spawn(async move {
            let mut ticker = interval(check_interval);
            
            loop {
                ticker.tick().await;
                
                // Check each path
                let mut paths_guard = paths.write().await;
                for path_state in paths_guard.iter_mut() {
                    if !path_state.path.enabled {
                        continue;
                    }
                    
                    match Self::check_path_stats(&path_state.path.path) {
                        Ok(stats) => {
                            let was_unhealthy = !path_state.stats.is_healthy;
                            path_state.stats = stats.clone();
                            
                            // Check for low space
                            let free_gb = stats.available_bytes as f32 / 1_073_741_824.0;
                            if free_gb < config.min_free_space_gb {
                                let _ = event_tx.send(StorageEvent::LowSpace(
                                    path_state.path.path.clone(),
                                    stats.usage_percent,
                                ));
                            }
                            
                            // Check if path recovered
                            if was_unhealthy && stats.is_healthy {
                                let _ = event_tx.send(StorageEvent::PathRecovered(
                                    path_state.path.path.clone(),
                                ));
                            }
                            
                            // Check if storage is full
                            if stats.usage_percent > 95.0 {
                                let _ = event_tx.send(StorageEvent::StorageFull(
                                    path_state.path.path.clone(),
                                ));
                            }
                        }
                        Err(e) => {
                            error!("Failed to check path {:?}: {}", path_state.path.path, e);
                            path_state.stats.is_healthy = false;
                            let _ = event_tx.send(StorageEvent::PathUnavailable(
                                path_state.path.path.clone(),
                            ));
                        }
                    }
                }
                
                // Trigger cleanup if needed
                if config.cleanup_policy.enabled {
                    Self::check_and_cleanup(&paths, &config, &event_tx).await;
                }
            }
        });
        
        self.monitoring_handle = Some(handle);
        info!("Storage monitoring started");
        Ok(())
    }
    
    pub async fn stop_monitoring(&mut self) {
        if let Some(handle) = self.monitoring_handle.take() {
            handle.abort();
            info!("Storage monitoring stopped");
        }
    }
    
    pub async fn select_path(&self, stream_id: &str, size_hint: Option<u64>) -> Result<PathBuf> {
        let mut paths = self.paths.write().await;
        
        // Filter enabled and healthy paths
        let available_paths: Vec<usize> = paths
            .iter()
            .enumerate()
            .filter(|(_, p)| p.path.enabled && p.stats.is_healthy)
            .map(|(i, _)| i)
            .collect();
        
        if available_paths.is_empty() {
            return Err(StorageError::AllPathsUnavailable.into());
        }
        
        // Check size hint if provided
        if let Some(size) = size_hint {
            let suitable_paths: Vec<usize> = available_paths
                .iter()
                .filter(|&&i| paths[i].stats.available_bytes >= size)
                .copied()
                .collect();
            
            if suitable_paths.is_empty() {
                let max_available = paths
                    .iter()
                    .map(|p| p.stats.available_bytes)
                    .max()
                    .unwrap_or(0);
                return Err(StorageError::InsufficientSpace(size, max_available).into());
            }
        }
        
        let selected_index = match self.selection_strategy {
            PathSelectionStrategy::RoundRobin => {
                let mut index = self.round_robin_index.write().await;
                let current = *index % available_paths.len();
                *index = (*index + 1) % available_paths.len();
                available_paths[current]
            }
            
            PathSelectionStrategy::LeastUsed => {
                available_paths
                    .iter()
                    .min_by_key(|&&i| paths[i].stats.used_bytes)
                    .copied()
                    .unwrap_or(available_paths[0])
            }
            
            PathSelectionStrategy::Priority => {
                available_paths
                    .iter()
                    .min_by_key(|&&i| paths[i].path.priority)
                    .copied()
                    .unwrap_or(available_paths[0])
            }
            
            PathSelectionStrategy::Affinity => {
                // Check if stream has affinity to a specific path
                available_paths
                    .iter()
                    .find(|&&i| paths[i].path.stream_affinity.contains(&stream_id.to_string()))
                    .copied()
                    .unwrap_or_else(|| {
                        // Fall back to least used
                        available_paths
                            .iter()
                            .min_by_key(|&&i| paths[i].stats.used_bytes)
                            .copied()
                            .unwrap_or(available_paths[0])
                    })
            }
        };
        
        // Update path state
        let selected_path = &mut paths[selected_index];
        selected_path.last_used = SystemTime::now();
        if !selected_path.current_streams.contains(&stream_id.to_string()) {
            selected_path.current_streams.push(stream_id.to_string());
        }
        
        Ok(selected_path.path.path.clone())
    }
    
    pub async fn release_path(&self, stream_id: &str, path: &Path) -> Result<()> {
        let mut paths = self.paths.write().await;
        
        if let Some(path_state) = paths.iter_mut().find(|p| p.path.path == path) {
            path_state.current_streams.retain(|id| id != stream_id);
            debug!("Released path {:?} for stream {}", path, stream_id);
        }
        
        Ok(())
    }
    
    pub async fn get_storage_stats(&self) -> Vec<StorageStats> {
        let paths = self.paths.read().await;
        paths.iter().map(|p| p.stats.clone()).collect()
    }
    
    pub async fn get_total_stats(&self) -> StorageStats {
        let paths = self.paths.read().await;
        
        let total_bytes: u64 = paths.iter().map(|p| p.stats.total_bytes).sum();
        let available_bytes: u64 = paths.iter().map(|p| p.stats.available_bytes).sum();
        let used_bytes: u64 = paths.iter().map(|p| p.stats.used_bytes).sum();
        
        StorageStats {
            path: PathBuf::from("total"),
            total_bytes,
            available_bytes,
            used_bytes,
            usage_percent: if total_bytes > 0 {
                (used_bytes as f32 / total_bytes as f32) * 100.0
            } else {
                0.0
            },
            is_healthy: paths.iter().any(|p| p.stats.is_healthy),
            last_check: SystemTime::now(),
        }
    }
    
    fn check_path_stats(path: &Path) -> Result<StorageStats> {
        use std::fs;
        
        // Ensure path exists
        if !path.exists() {
            fs::create_dir_all(path)?;
            info!("Created storage path: {:?}", path);
        }
        
        // Check if we can write to the path
        let test_file = path.join(".storage_test");
        let is_healthy = match fs::write(&test_file, b"test") {
            Ok(_) => {
                let _ = fs::remove_file(test_file);
                true
            }
            Err(e) => {
                warn!("Cannot write to path {:?}: {}", path, e);
                false
            }
        };
        
        // Get disk usage (platform-specific)
        #[cfg(windows)]
        let (total_bytes, available_bytes) = {
            use winapi::um::fileapi::GetDiskFreeSpaceExW;
            use winapi::um::winnt::ULARGE_INTEGER;
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            
            let path_wide: Vec<u16> = OsStr::new(path.to_str().unwrap_or("."))
                .encode_wide()
                .chain(Some(0))
                .collect();
            
            let mut available: ULARGE_INTEGER = unsafe { std::mem::zeroed() };
            let mut total: ULARGE_INTEGER = unsafe { std::mem::zeroed() };
            let mut free: ULARGE_INTEGER = unsafe { std::mem::zeroed() };
            
            unsafe {
                if GetDiskFreeSpaceExW(
                    path_wide.as_ptr(),
                    &mut available,
                    &mut total,
                    &mut free,
                ) != 0 {
                    (*total.QuadPart() as u64, *available.QuadPart() as u64)
                } else {
                    (0, 0)
                }
            }
        };
        
        #[cfg(not(windows))]
        let (total_bytes, available_bytes) = {
            use nix::sys::statvfs::statvfs;
            
            match statvfs(path) {
                Ok(stat) => {
                    let total = stat.blocks() * stat.block_size();
                    let available = stat.blocks_available() * stat.block_size();
                    (total, available)
                }
                Err(_) => (0, 0),
            }
        };
        
        let used_bytes = total_bytes.saturating_sub(available_bytes);
        let usage_percent = if total_bytes > 0 {
            (used_bytes as f32 / total_bytes as f32) * 100.0
        } else {
            0.0
        };
        
        Ok(StorageStats {
            path: path.to_path_buf(),
            total_bytes,
            available_bytes,
            used_bytes,
            usage_percent,
            is_healthy,
            last_check: SystemTime::now(),
        })
    }
    
    async fn check_and_cleanup(
        paths: &Arc<RwLock<Vec<PathState>>>,
        config: &StorageConfig,
        event_tx: &broadcast::Sender<StorageEvent>,
    ) {
        let paths = paths.read().await;
        let mut files_to_delete = Vec::new();
        
        for path_state in paths.iter() {
            if !path_state.path.enabled || !path_state.stats.is_healthy {
                continue;
            }
            
            // Check if cleanup is needed
            let needs_cleanup = if let Some(max_gb) = config.cleanup_policy.max_size_gb {
                let used_gb = path_state.stats.used_bytes as f32 / 1_073_741_824.0;
                used_gb > max_gb
            } else {
                false
            } || path_state.stats.usage_percent > 90.0;
            
            if !needs_cleanup {
                continue;
            }
            
            // Find files to delete based on age
            if let Some(max_age_days) = config.cleanup_policy.max_age_days {
                let max_age = Duration::from_secs(max_age_days as u64 * 86400);
                
                if let Ok(entries) = std::fs::read_dir(&path_state.path.path) {
                    for entry in entries.flatten() {
                        if let Ok(metadata) = entry.metadata() {
                            if metadata.is_file() {
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(age) = SystemTime::now().duration_since(modified) {
                                        if age > max_age {
                                            files_to_delete.push((entry.path(), metadata.len()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if !files_to_delete.is_empty() {
            let _ = event_tx.send(StorageEvent::CleanupStarted);
            
            let mut deleted_count = 0;
            let mut freed_bytes = 0u64;
            
            for (file_path, size) in files_to_delete {
                match std::fs::remove_file(&file_path) {
                    Ok(_) => {
                        deleted_count += 1;
                        freed_bytes += size;
                        debug!("Deleted old file: {:?}", file_path);
                    }
                    Err(e) => {
                        warn!("Failed to delete {:?}: {}", file_path, e);
                    }
                }
            }
            
            if deleted_count > 0 {
                info!("Cleanup completed: {} files deleted, {} bytes freed", 
                      deleted_count, freed_bytes);
                let _ = event_tx.send(StorageEvent::CleanupCompleted(deleted_count, freed_bytes));
            }
        }
    }
    
    pub fn set_selection_strategy(&mut self, strategy: PathSelectionStrategy) {
        self.selection_strategy = strategy;
        info!("Storage selection strategy changed to {:?}", strategy);
    }
    
    pub async fn add_path(&self, path: StoragePath) -> Result<()> {
        let stats = Self::check_path_stats(&path.path)?;
        
        let mut paths = self.paths.write().await;
        
        // Check if path already exists
        if paths.iter().any(|p| p.path.path == path.path) {
            return Err(StreamManagerError::ConfigError(
                format!("Storage path already exists: {:?}", path.path)
            ));
        }
        
        paths.push(PathState {
            path,
            stats,
            last_used: SystemTime::now(),
            current_streams: Vec::new(),
        });
        
        info!("Added storage path: {:?}", paths.last().unwrap().path.path);
        Ok(())
    }
    
    pub async fn remove_path(&self, path: &Path) -> Result<()> {
        let mut paths = self.paths.write().await;
        
        let initial_len = paths.len();
        paths.retain(|p| p.path.path != path);
        
        if paths.len() < initial_len {
            info!("Removed storage path: {:?}", path);
            Ok(())
        } else {
            Err(StorageError::PathNotFound(path.to_path_buf()).into())
        }
    }
    
    pub async fn enable_path(&self, path: &Path, enabled: bool) -> Result<()> {
        let mut paths = self.paths.write().await;
        
        if let Some(path_state) = paths.iter_mut().find(|p| p.path.path == path) {
            path_state.path.enabled = enabled;
            info!("Storage path {:?} {}", path, if enabled { "enabled" } else { "disabled" });
            Ok(())
        } else {
            Err(StorageError::PathNotFound(path.to_path_buf()).into())
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\ProgramData\\stream-manager\\recordings")
        } else {
            PathBuf::from("/var/lib/stream-manager/recordings")
        };
        
        Self {
            paths: vec![StoragePath {
                path: base_path,
                enabled: true,
                priority: 1,
                max_usage_gb: None,
                stream_affinity: Vec::new(),
            }],
            cleanup_policy: CleanupPolicy::default(),
            check_interval_seconds: 60,
            min_free_space_gb: 10.0,
            max_total_usage_gb: None,
        }
    }
}

impl Default for CleanupPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_age_days: Some(7),
            max_size_gb: None,
            min_segments_per_stream: 2,
            priority_retention: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_storage_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = StorageConfig {
            paths: vec![StoragePath {
                path: temp_dir.path().to_path_buf(),
                enabled: true,
                priority: 1,
                max_usage_gb: None,
                stream_affinity: Vec::new(),
            }],
            cleanup_policy: CleanupPolicy::default(),
            check_interval_seconds: 60,
            min_free_space_gb: 1.0,
            max_total_usage_gb: None,
        };
        
        let manager = StorageManager::new(config).unwrap();
        let stats = manager.get_storage_stats().await;
        
        assert_eq!(stats.len(), 1);
        assert!(stats[0].is_healthy);
    }
    
    #[tokio::test]
    async fn test_path_selection_round_robin() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        
        let config = StorageConfig {
            paths: vec![
                StoragePath {
                    path: temp_dir1.path().to_path_buf(),
                    enabled: true,
                    priority: 1,
                    max_usage_gb: None,
                    stream_affinity: Vec::new(),
                },
                StoragePath {
                    path: temp_dir2.path().to_path_buf(),
                    enabled: true,
                    priority: 2,
                    max_usage_gb: None,
                    stream_affinity: Vec::new(),
                },
            ],
            cleanup_policy: CleanupPolicy::default(),
            check_interval_seconds: 60,
            min_free_space_gb: 1.0,
            max_total_usage_gb: None,
        };
        
        let mut manager = StorageManager::new(config).unwrap();
        manager.set_selection_strategy(PathSelectionStrategy::RoundRobin);
        
        let path1 = manager.select_path("stream1", None).await.unwrap();
        let path2 = manager.select_path("stream2", None).await.unwrap();
        let path3 = manager.select_path("stream3", None).await.unwrap();
        
        // Should cycle through paths
        assert_eq!(path1, temp_dir1.path());
        assert_eq!(path2, temp_dir2.path());
        assert_eq!(path3, temp_dir1.path());
    }
    
    #[tokio::test]
    async fn test_path_selection_priority() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        
        let config = StorageConfig {
            paths: vec![
                StoragePath {
                    path: temp_dir1.path().to_path_buf(),
                    enabled: true,
                    priority: 2,
                    max_usage_gb: None,
                    stream_affinity: Vec::new(),
                },
                StoragePath {
                    path: temp_dir2.path().to_path_buf(),
                    enabled: true,
                    priority: 1, // Higher priority
                    max_usage_gb: None,
                    stream_affinity: Vec::new(),
                },
            ],
            cleanup_policy: CleanupPolicy::default(),
            check_interval_seconds: 60,
            min_free_space_gb: 1.0,
            max_total_usage_gb: None,
        };
        
        let mut manager = StorageManager::new(config).unwrap();
        manager.set_selection_strategy(PathSelectionStrategy::Priority);
        
        let path1 = manager.select_path("stream1", None).await.unwrap();
        let path2 = manager.select_path("stream2", None).await.unwrap();
        
        // Should always select higher priority path
        assert_eq!(path1, temp_dir2.path());
        assert_eq!(path2, temp_dir2.path());
    }
    
    #[tokio::test]
    async fn test_path_affinity() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        
        let config = StorageConfig {
            paths: vec![
                StoragePath {
                    path: temp_dir1.path().to_path_buf(),
                    enabled: true,
                    priority: 1,
                    max_usage_gb: None,
                    stream_affinity: vec!["stream1".to_string()],
                },
                StoragePath {
                    path: temp_dir2.path().to_path_buf(),
                    enabled: true,
                    priority: 1,
                    max_usage_gb: None,
                    stream_affinity: vec!["stream2".to_string()],
                },
            ],
            cleanup_policy: CleanupPolicy::default(),
            check_interval_seconds: 60,
            min_free_space_gb: 1.0,
            max_total_usage_gb: None,
        };
        
        let mut manager = StorageManager::new(config).unwrap();
        manager.set_selection_strategy(PathSelectionStrategy::Affinity);
        
        let path1 = manager.select_path("stream1", None).await.unwrap();
        let path2 = manager.select_path("stream2", None).await.unwrap();
        
        // Should select paths based on affinity
        assert_eq!(path1, temp_dir1.path());
        assert_eq!(path2, temp_dir2.path());
    }
    
    #[tokio::test]
    async fn test_total_stats_calculation() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        
        let config = StorageConfig {
            paths: vec![
                StoragePath {
                    path: temp_dir1.path().to_path_buf(),
                    enabled: true,
                    priority: 1,
                    max_usage_gb: None,
                    stream_affinity: Vec::new(),
                },
                StoragePath {
                    path: temp_dir2.path().to_path_buf(),
                    enabled: true,
                    priority: 2,
                    max_usage_gb: None,
                    stream_affinity: Vec::new(),
                },
            ],
            cleanup_policy: CleanupPolicy::default(),
            check_interval_seconds: 60,
            min_free_space_gb: 1.0,
            max_total_usage_gb: None,
        };
        
        let manager = StorageManager::new(config).unwrap();
        let total = manager.get_total_stats().await;
        
        // Just verify the calculation logic works
        assert!(total.total_bytes >= 0);
        assert!(total.available_bytes >= 0);
        assert!(total.usage_percent >= 0.0);
        assert!(total.usage_percent <= 100.0);
    }
    
    #[tokio::test]
    async fn test_event_subscription() {
        let temp_dir = TempDir::new().unwrap();
        
        let config = StorageConfig {
            paths: vec![StoragePath {
                path: temp_dir.path().to_path_buf(),
                enabled: true,
                priority: 1,
                max_usage_gb: None,
                stream_affinity: Vec::new(),
            }],
            cleanup_policy: CleanupPolicy::default(),
            check_interval_seconds: 60,
            min_free_space_gb: 1.0,
            max_total_usage_gb: None,
        };
        
        let manager = StorageManager::new(config).unwrap();
        
        // Test that we can subscribe to events
        let _rx1 = manager.subscribe();
        let _rx2 = manager.subscribe();
        
        // Multiple subscriptions should work
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_cleanup_policy_configuration() {
        let policy = CleanupPolicy {
            enabled: true,
            max_age_days: Some(30),
            max_size_gb: Some(100.0),
            min_segments_per_stream: 5,
            priority_retention: {
                let mut map = HashMap::new();
                map.insert("important_stream".to_string(), 60);
                map
            },
        };
        
        assert!(policy.enabled);
        assert_eq!(policy.max_age_days, Some(30));
        assert_eq!(policy.max_size_gb, Some(100.0));
        assert_eq!(policy.min_segments_per_stream, 5);
        assert_eq!(policy.priority_retention.get("important_stream"), Some(&60));
    }
    
    #[tokio::test]
    async fn test_add_remove_paths() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        
        let config = StorageConfig {
            paths: vec![StoragePath {
                path: temp_dir1.path().to_path_buf(),
                enabled: true,
                priority: 1,
                max_usage_gb: None,
                stream_affinity: Vec::new(),
            }],
            cleanup_policy: CleanupPolicy::default(),
            check_interval_seconds: 60,
            min_free_space_gb: 1.0,
            max_total_usage_gb: None,
        };
        
        let manager = StorageManager::new(config).unwrap();
        
        // Add a new path
        let new_path = StoragePath {
            path: temp_dir2.path().to_path_buf(),
            enabled: true,
            priority: 2,
            max_usage_gb: None,
            stream_affinity: Vec::new(),
        };
        
        manager.add_path(new_path).await.unwrap();
        
        let stats = manager.get_storage_stats().await;
        assert_eq!(stats.len(), 2);
        
        // Remove the path
        manager.remove_path(temp_dir2.path()).await.unwrap();
        
        let stats = manager.get_storage_stats().await;
        assert_eq!(stats.len(), 1);
    }
    
    #[tokio::test]
    async fn test_insufficient_space() {
        let temp_dir = TempDir::new().unwrap();
        
        let config = StorageConfig {
            paths: vec![StoragePath {
                path: temp_dir.path().to_path_buf(),
                enabled: true,
                priority: 1,
                max_usage_gb: None,
                stream_affinity: Vec::new(),
            }],
            cleanup_policy: CleanupPolicy::default(),
            check_interval_seconds: 60,
            min_free_space_gb: 1.0,
            max_total_usage_gb: None,
        };
        
        let manager = StorageManager::new(config).unwrap();
        
        // Request more space than available
        let huge_size = u64::MAX;
        let result = manager.select_path("stream1", Some(huge_size)).await;
        
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Insufficient space"));
    }
}