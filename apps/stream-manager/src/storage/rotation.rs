use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex, mpsc};
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode};
use tokio::time::{Duration, interval};

use crate::storage::StorageError;

#[derive(Debug, Error)]
pub enum RotationError {
    #[error("Disk not found: {0}")]
    DiskNotFound(String),
    
    #[error("Rotation already in progress")]
    RotationInProgress,
    
    #[error("No alternative disk available")]
    NoAlternativeDisk,
    
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Watcher error: {0}")]
    Watcher(#[from] notify::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskRotationConfig {
    pub auto_rotate_on_unmount: bool,
    pub buffer_size_mb: usize,
    pub migration_timeout_secs: u64,
    pub poll_interval_secs: u64,
    pub min_free_space_gb: f64,
}

impl Default for DiskRotationConfig {
    fn default() -> Self {
        Self {
            auto_rotate_on_unmount: true,
            buffer_size_mb: 512,
            migration_timeout_secs: 30,
            poll_interval_secs: 5,
            min_free_space_gb: 10.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub path: PathBuf,
    pub uuid: Option<String>,
    pub serial: Option<String>,
    pub mounted: bool,
    pub available_space_gb: f64,
    pub total_space_gb: f64,
    pub is_active: bool,
    pub last_seen: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize)]
pub enum RotationState {
    Idle,
    Preparing { target_disk: PathBuf },
    Migrating { from: PathBuf, to: PathBuf, progress: f32 },
    Completing { new_disk: PathBuf },
    Failed { reason: String },
}

#[derive(Debug)]
struct WriteBuffer {
    data: VecDeque<BufferedWrite>,
    max_size: usize,
    current_size: usize,
}

#[derive(Debug)]
struct BufferedWrite {
    stream_id: String,
    data: Vec<u8>,
    timestamp: std::time::SystemTime,
}

impl WriteBuffer {
    fn new(max_size_mb: usize) -> Self {
        Self {
            data: VecDeque::new(),
            max_size: max_size_mb * 1024 * 1024,
            current_size: 0,
        }
    }
    
    fn push(&mut self, stream_id: String, data: Vec<u8>) -> Result<(), RotationError> {
        let write_size = data.len();
        
        if self.current_size + write_size > self.max_size {
            return Err(RotationError::MigrationFailed(
                "Write buffer overflow".to_string()
            ));
        }
        
        self.data.push_back(BufferedWrite {
            stream_id,
            data,
            timestamp: std::time::SystemTime::now(),
        });
        self.current_size += write_size;
        
        Ok(())
    }
    
    fn drain(&mut self) -> Vec<BufferedWrite> {
        let writes = self.data.drain(..).collect();
        self.current_size = 0;
        writes
    }
}

pub struct DiskRotationManager {
    config: Arc<RwLock<DiskRotationConfig>>,
    disks: Arc<RwLock<HashMap<PathBuf, DiskInfo>>>,
    active_disk: Arc<RwLock<Option<PathBuf>>>,
    rotation_state: Arc<RwLock<RotationState>>,
    rotation_queue: Arc<Mutex<VecDeque<PathBuf>>>,
    write_buffer: Arc<Mutex<WriteBuffer>>,
    event_tx: mpsc::UnboundedSender<RotationEvent>,
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<RotationEvent>>>,
    watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
}

#[derive(Debug, Clone)]
pub enum RotationEvent {
    DiskAdded(PathBuf),
    DiskRemoved(PathBuf),
    DiskUnmounted(PathBuf),
    RotationStarted { from: PathBuf, to: PathBuf },
    RotationCompleted { new_disk: PathBuf },
    RotationFailed { reason: String },
    BufferOverflow,
}

impl DiskRotationManager {
    pub fn new(config: DiskRotationConfig) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Self {
            config: Arc::new(RwLock::new(config.clone())),
            disks: Arc::new(RwLock::new(HashMap::new())),
            active_disk: Arc::new(RwLock::new(None)),
            rotation_state: Arc::new(RwLock::new(RotationState::Idle)),
            rotation_queue: Arc::new(Mutex::new(VecDeque::new())),
            write_buffer: Arc::new(Mutex::new(WriteBuffer::new(config.buffer_size_mb))),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            watcher: Arc::new(Mutex::new(None)),
        }
    }
    
    pub async fn start_monitoring(&self) -> Result<(), RotationError> {
        info!("Starting disk rotation monitoring");
        
        // Start filesystem watcher
        self.start_fs_watcher().await?;
        
        // Start mount point polling
        let manager = self.clone();
        tokio::spawn(async move {
            manager.poll_mount_points().await;
        });
        
        // Start event processor
        let manager = self.clone();
        tokio::spawn(async move {
            manager.process_events().await;
        });
        
        Ok(())
    }
    
    async fn start_fs_watcher(&self) -> Result<(), RotationError> {
        let (tx, mut rx) = mpsc::channel(100);
        
        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;
        
        // Watch mount points
        #[cfg(target_os = "linux")]
        {
            watcher.watch(Path::new("/media"), RecursiveMode::NonRecursive)?;
            watcher.watch(Path::new("/mnt"), RecursiveMode::NonRecursive)?;
            watcher.watch(Path::new("/run/media"), RecursiveMode::Recursive)?;
        }
        
        #[cfg(target_os = "windows")]
        {
            // On Windows, we'll poll drive letters instead
            // No specific paths to watch with notify
        }
        
        *self.watcher.lock().await = Some(watcher);
        
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event.kind {
                    EventKind::Create(_) => {
                        for path in event.paths {
                            let _ = event_tx.send(RotationEvent::DiskAdded(path));
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            let _ = event_tx.send(RotationEvent::DiskRemoved(path));
                        }
                    }
                    _ => {}
                }
            }
        });
        
        Ok(())
    }
    
    async fn poll_mount_points(&self) {
        let config = self.config.read().await;
        let poll_interval = Duration::from_secs(config.poll_interval_secs);
        drop(config);
        
        let mut ticker = interval(poll_interval);
        
        loop {
            ticker.tick().await;
            
            if let Err(e) = self.scan_mount_points().await {
                error!("Failed to scan mount points: {}", e);
            }
        }
    }
    
    async fn scan_mount_points(&self) -> Result<(), RotationError> {
        let mut disks = self.disks.write().await;
        let mut found_paths = Vec::new();
        
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            
            // Parse /proc/mounts
            if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
                for line in mounts.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let mount_point = PathBuf::from(parts[1]);
                        
                        // Filter for relevant mount points
                        if mount_point.starts_with("/media") || 
                           mount_point.starts_with("/mnt") ||
                           mount_point.starts_with("/run/media") {
                            found_paths.push(mount_point);
                        }
                    }
                }
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            
            // Check drive letters
            for letter in b'A'..=b'Z' {
                let drive = format!("{}:\\", letter as char);
                let path = PathBuf::from(&drive);
                
                if path.exists() {
                    found_paths.push(path);
                }
            }
        }
        
        // Update disk info
        for path in &found_paths {
            if !disks.contains_key(path) {
                if let Ok(info) = self.get_disk_info(path).await {
                    disks.insert(path.clone(), info);
                    let _ = self.event_tx.send(RotationEvent::DiskAdded(path.clone()));
                }
            } else {
                // Update existing disk info
                if let Some(disk) = disks.get_mut(path) {
                    if let Ok(info) = self.get_disk_info(path).await {
                        disk.available_space_gb = info.available_space_gb;
                        disk.last_seen = std::time::SystemTime::now();
                    }
                }
            }
        }
        
        // Check for removed disks
        let mut removed = Vec::new();
        for (path, disk) in disks.iter() {
            if !found_paths.contains(path) {
                if disk.mounted {
                    removed.push(path.clone());
                }
            }
        }
        
        for path in removed {
            if let Some(disk) = disks.get_mut(&path) {
                disk.mounted = false;
                let _ = self.event_tx.send(RotationEvent::DiskUnmounted(path));
            }
        }
        
        Ok(())
    }
    
    async fn get_disk_info(&self, path: &Path) -> Result<DiskInfo, RotationError> {
        use std::fs;
        
        let _metadata = fs::metadata(path)?;
        
        // Get available space
        let (available_space_gb, total_space_gb) = self.get_disk_space(path)?;
        
        // Try to get UUID and serial (platform-specific)
        let (uuid, serial) = self.get_disk_identifiers(path).await;
        
        Ok(DiskInfo {
            path: path.to_path_buf(),
            uuid,
            serial,
            mounted: true,
            available_space_gb,
            total_space_gb,
            is_active: false,
            last_seen: std::time::SystemTime::now(),
        })
    }
    
    fn get_disk_space(&self, path: &Path) -> Result<(f64, f64), RotationError> {
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            use std::fs;
            
            let stat = nix::sys::statvfs::statvfs(path)
                .map_err(|e| RotationError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other, e
                )))?;
            
            let available = (stat.blocks_available() * stat.block_size()) as f64 / 1_073_741_824.0;
            let total = (stat.blocks() * stat.block_size()) as f64 / 1_073_741_824.0;
            
            Ok((available, total))
        }
        
        #[cfg(target_os = "windows")]
        {
            use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            
            let path_wide: Vec<u16> = OsStr::new(path.to_str().unwrap())
                .encode_wide()
                .chain(Some(0))
                .collect();
            
            let mut available: u64 = 0;
            let mut total: u64 = 0;
            let mut free: u64 = 0;
            
            unsafe {
                if GetDiskFreeSpaceExW(
                    path_wide.as_ptr(),
                    &mut available as *mut u64,
                    &mut total as *mut u64,
                    &mut free as *mut u64,
                ) == 0 {
                    return Err(RotationError::Io(std::io::Error::last_os_error()));
                }
            }
            
            let available_gb = available as f64 / 1_073_741_824.0;
            let total_gb = total as f64 / 1_073_741_824.0;
            
            Ok((available_gb, total_gb))
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            Ok((0.0, 0.0))
        }
    }
    
    async fn get_disk_identifiers(&self, _path: &Path) -> (Option<String>, Option<String>) {
        // Platform-specific implementation to get UUID and serial
        // This would require additional system calls or parsing
        (None, None)
    }
    
    async fn process_events(&self) {
        let mut event_rx = self.event_rx.lock().await;
        
        while let Some(event) = event_rx.recv().await {
            match event {
                RotationEvent::DiskAdded(path) => {
                    info!("Disk added: {:?}", path);
                    self.handle_disk_added(path).await;
                }
                RotationEvent::DiskRemoved(path) | RotationEvent::DiskUnmounted(path) => {
                    info!("Disk removed/unmounted: {:?}", path);
                    self.handle_disk_removed(path).await;
                }
                RotationEvent::RotationStarted { from, to } => {
                    info!("Rotation started: {:?} -> {:?}", from, to);
                }
                RotationEvent::RotationCompleted { new_disk } => {
                    info!("Rotation completed: {:?}", new_disk);
                }
                RotationEvent::RotationFailed { reason } => {
                    error!("Rotation failed: {}", reason);
                }
                RotationEvent::BufferOverflow => {
                    error!("Write buffer overflow during rotation");
                }
            }
        }
    }
    
    async fn handle_disk_added(&self, path: PathBuf) {
        let mut rotation_queue = self.rotation_queue.lock().await;
        
        // Add to rotation queue if it's a valid storage disk
        if self.is_valid_storage_disk(&path).await {
            rotation_queue.push_back(path);
            
            // Try to start rotation if needed
            drop(rotation_queue);
            let _ = self.try_auto_rotate().await;
        }
    }
    
    async fn handle_disk_removed(&self, path: PathBuf) {
        let active_disk = self.active_disk.read().await;
        
        if let Some(active) = active_disk.as_ref() {
            if active == &path {
                drop(active_disk);
                
                // Active disk removed, need immediate rotation
                warn!("Active disk removed, initiating emergency rotation");
                if let Err(e) = self.emergency_rotate().await {
                    error!("Emergency rotation failed: {}", e);
                }
            }
        }
    }
    
    async fn is_valid_storage_disk(&self, path: &Path) -> bool {
        if let Ok(info) = self.get_disk_info(path).await {
            let config = self.config.read().await;
            info.available_space_gb >= config.min_free_space_gb
        } else {
            false
        }
    }
    
    async fn try_auto_rotate(&self) -> Result<(), RotationError> {
        let config = self.config.read().await;
        if !config.auto_rotate_on_unmount {
            return Ok(());
        }
        drop(config);
        
        let state = self.rotation_state.read().await;
        if !matches!(*state, RotationState::Idle) {
            return Err(RotationError::RotationInProgress);
        }
        drop(state);
        
        // Check if we need rotation
        let active_disk = self.active_disk.read().await;
        if active_disk.is_none() {
            drop(active_disk);
            
            // No active disk, pick one from queue
            let mut queue = self.rotation_queue.lock().await;
            if let Some(target) = queue.pop_front() {
                drop(queue);
                self.rotate_to_disk(target).await?;
            }
        }
        
        Ok(())
    }
    
    pub async fn trigger_rotation(&self, target_disk: Option<PathBuf>) -> Result<(), RotationError> {
        let state = self.rotation_state.read().await;
        if !matches!(*state, RotationState::Idle) {
            return Err(RotationError::RotationInProgress);
        }
        drop(state);
        
        let target = if let Some(disk) = target_disk {
            disk
        } else {
            // Pick from queue
            let mut queue = self.rotation_queue.lock().await;
            queue.pop_front()
                .ok_or(RotationError::NoAlternativeDisk)?
        };
        
        self.rotate_to_disk(target).await
    }
    
    async fn rotate_to_disk(&self, target: PathBuf) -> Result<(), RotationError> {
        info!("Starting rotation to disk: {:?}", target);
        
        // Update state
        *self.rotation_state.write().await = RotationState::Preparing {
            target_disk: target.clone(),
        };
        
        // Get current active disk
        let current = self.active_disk.read().await.clone();
        let from_disk = current.clone().unwrap_or_else(|| PathBuf::from("/tmp"));
        
        // Send rotation started event
        let _ = self.event_tx.send(RotationEvent::RotationStarted {
            from: from_disk.clone(),
            to: target.clone(),
        });
        
        // Update state to migrating
        *self.rotation_state.write().await = RotationState::Migrating {
            from: from_disk.clone(),
            to: target.clone(),
            progress: 0.0,
        };
        
        // Perform migration
        if let Err(e) = self.migrate_recordings(&from_disk, &target).await {
            *self.rotation_state.write().await = RotationState::Failed {
                reason: e.to_string(),
            };
            let _ = self.event_tx.send(RotationEvent::RotationFailed {
                reason: e.to_string(),
            });
            return Err(e);
        }
        
        // Update active disk
        *self.active_disk.write().await = Some(target.clone());
        
        // Update disk info
        let mut disks = self.disks.write().await;
        if let Some(old_disk) = current {
            if let Some(disk) = disks.get_mut(&old_disk) {
                disk.is_active = false;
            }
        }
        if let Some(disk) = disks.get_mut(&target) {
            disk.is_active = true;
        }
        
        // Complete rotation
        *self.rotation_state.write().await = RotationState::Idle;
        let _ = self.event_tx.send(RotationEvent::RotationCompleted {
            new_disk: target,
        });
        
        Ok(())
    }
    
    async fn migrate_recordings(&self, from: &Path, to: &Path) -> Result<(), RotationError> {
        info!("Migrating recordings from {:?} to {:?}", from, to);
        
        // Drain write buffer
        let mut buffer = self.write_buffer.lock().await;
        let buffered_writes = buffer.drain();
        
        // Process buffered writes to new location
        for write in buffered_writes {
            // In a real implementation, this would write to the actual recording files
            debug!("Migrating buffered write for stream {}", write.stream_id);
        }
        
        // Update progress
        for i in 0..=100 {
            *self.rotation_state.write().await = RotationState::Migrating {
                from: from.to_path_buf(),
                to: to.to_path_buf(),
                progress: i as f32 / 100.0,
            };
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        Ok(())
    }
    
    async fn emergency_rotate(&self) -> Result<(), RotationError> {
        warn!("Performing emergency rotation");
        
        // Try to find any available disk
        let mut queue = self.rotation_queue.lock().await;
        if let Some(target) = queue.pop_front() {
            drop(queue);
            self.rotate_to_disk(target).await
        } else {
            // No alternative disk, keep buffering
            Err(RotationError::NoAlternativeDisk)
        }
    }
    
    pub async fn buffer_write(&self, stream_id: String, data: Vec<u8>) -> Result<(), RotationError> {
        let state = self.rotation_state.read().await;
        
        if matches!(*state, RotationState::Migrating { .. }) {
            // Buffer writes during migration
            let mut buffer = self.write_buffer.lock().await;
            buffer.push(stream_id, data)?;
        }
        
        Ok(())
    }
    
    pub async fn get_rotation_state(&self) -> RotationState {
        self.rotation_state.read().await.clone()
    }
    
    pub async fn get_active_disk(&self) -> Option<PathBuf> {
        self.active_disk.read().await.clone()
    }
    
    pub async fn list_disks(&self) -> Vec<DiskInfo> {
        self.disks.read().await.values().cloned().collect()
    }
}

impl Clone for DiskRotationManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            disks: self.disks.clone(),
            active_disk: self.active_disk.clone(),
            rotation_state: self.rotation_state.clone(),
            rotation_queue: self.rotation_queue.clone(),
            write_buffer: self.write_buffer.clone(),
            event_tx: self.event_tx.clone(),
            event_rx: self.event_rx.clone(),
            watcher: self.watcher.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_disk_rotation_manager_creation() {
        let config = DiskRotationConfig::default();
        let manager = DiskRotationManager::new(config);
        
        assert!(manager.get_active_disk().await.is_none());
        assert!(matches!(
            manager.get_rotation_state().await,
            RotationState::Idle
        ));
    }
    
    #[tokio::test]
    async fn test_write_buffer() {
        let mut buffer = WriteBuffer::new(1); // 1MB buffer
        
        // Add some writes
        assert!(buffer.push("stream1".to_string(), vec![0u8; 1024]).is_ok());
        assert!(buffer.push("stream2".to_string(), vec![0u8; 1024]).is_ok());
        
        // Drain buffer
        let writes = buffer.drain();
        assert_eq!(writes.len(), 2);
        assert_eq!(buffer.current_size, 0);
    }
    
    #[tokio::test]
    async fn test_buffer_overflow() {
        let mut buffer = WriteBuffer::new(1); // 1MB buffer
        
        // Try to overflow
        let large_write = vec![0u8; 2 * 1024 * 1024]; // 2MB
        assert!(buffer.push("stream1".to_string(), large_write).is_err());
    }
    
    #[tokio::test]
    async fn test_rotation_state_transitions() {
        let config = DiskRotationConfig::default();
        let manager = DiskRotationManager::new(config);
        
        // Initial state
        assert!(matches!(
            manager.get_rotation_state().await,
            RotationState::Idle
        ));
        
        // Can't rotate without target
        assert!(manager.trigger_rotation(None).await.is_err());
    }
    
    #[tokio::test]
    async fn test_disk_info_creation() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        
        let config = DiskRotationConfig::default();
        let manager = DiskRotationManager::new(config);
        
        if let Ok(info) = manager.get_disk_info(path).await {
            assert_eq!(info.path, path);
            assert!(info.mounted);
            assert!(info.available_space_gb >= 0.0);
            assert!(info.total_space_gb >= 0.0);
        }
    }
    
    #[tokio::test]
    async fn test_disk_space_check() {
        let config = DiskRotationConfig::default();
        let manager = DiskRotationManager::new(config);
        
        let temp_dir = TempDir::new().unwrap();
        let (available, total) = manager.get_disk_space(temp_dir.path()).unwrap();
        
        assert!(available >= 0.0);
        assert!(total >= 0.0);
        assert!(total >= available);
    }
    
    #[tokio::test]
    async fn test_disk_rotation_migration() {
        let config = DiskRotationConfig {
            buffer_size_mb: 10,
            ..Default::default()
        };
        let manager = DiskRotationManager::new(config);
        
        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        
        // Set initial active disk
        *manager.active_disk.write().await = Some(temp1.path().to_path_buf());
        
        // Add target disk to queue
        manager.rotation_queue.lock().await.push_back(temp2.path().to_path_buf());
        
        // Trigger rotation
        let result = manager.trigger_rotation(Some(temp2.path().to_path_buf())).await;
        assert!(result.is_ok());
        
        // Check new active disk
        assert_eq!(
            manager.get_active_disk().await,
            Some(temp2.path().to_path_buf())
        );
    }
}