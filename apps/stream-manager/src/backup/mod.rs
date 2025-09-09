//! Recovery and maintenance module
//!
//! This module provides recovery mechanisms for:
//! - Database corruption recovery
//! - File-database synchronization
//! - System reset capabilities

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use crate::database::Database;

/// Recovery configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupConfig {
    /// Enable automatic recovery checks
    pub enabled: bool,
    /// Check interval in seconds
    pub check_interval_secs: u64,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_secs: 300, // 5 minutes
        }
    }
}

/// Recovery manager for database and file system
pub struct BackupManager {
    config: Arc<RwLock<BackupConfig>>,
    database: Option<Arc<Database>>,
    recordings_path: PathBuf,
}

impl BackupManager {
    /// Create a new recovery manager
    pub fn new(config: BackupConfig, recordings_path: PathBuf) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            database: None,
            recordings_path,
        }
    }

    /// Set the database for recovery operations
    pub fn set_database(&mut self, database: Arc<Database>) {
        self.database = Some(database);
    }

    /// Check database integrity and recover if needed
    pub async fn check_database_integrity(&self) -> Result<bool, RecoveryError> {
        let Some(database) = &self.database else {
            return Err(RecoveryError::NoDatabaseConfigured);
        };

        // Run SQLite integrity check
        match sqlx::query("PRAGMA integrity_check")
            .fetch_one(database.pool())
            .await
        {
            Ok(row) => {
                let result: String = row.get(0);
                if result == "ok" {
                    info!("Database integrity check passed");
                    // Even if OK, sync with filesystem to ensure consistency
                    self.sync_recordings().await?;
                    Ok(true)
                } else {
                    warn!("Database integrity check failed: {}", result);
                    self.rebuild_database_from_filesystem().await?;
                    Ok(false)
                }
            }
            Err(e) => {
                error!("Database integrity check error: {}", e);
                self.rebuild_database_from_filesystem().await?;
                Ok(false)
            }
        }
    }

    /// Recover a corrupted database
    async fn recover_database(&self) -> Result<(), RecoveryError> {
        let Some(database) = &self.database else {
            return Err(RecoveryError::NoDatabaseConfigured);
        };

        warn!("Attempting database recovery");

        // Try to recover what we can
        match sqlx::query("PRAGMA writable_schema = ON; PRAGMA integrity_check")
            .execute(database.pool())
            .await
        {
            Ok(_) => {
                info!("Database recovery attempted");
                
                // Re-run migrations to ensure schema is correct
                if let Err(e) = database.run_migrations().await {
                    error!("Failed to re-run migrations: {}", e);
                    // If migrations fail, recreate the database
                    self.recreate_database().await?;
                }
            }
            Err(e) => {
                error!("Database recovery failed: {}, recreating database", e);
                self.recreate_database().await?;
            }
        }

        Ok(())
    }

    /// Recreate the database from scratch
    async fn recreate_database(&self) -> Result<(), RecoveryError> {
        let Some(database) = &self.database else {
            return Err(RecoveryError::NoDatabaseConfigured);
        };

        warn!("Recreating database from scratch");
        
        // Close all connections and recreate
        database.pool().close().await;
        
        // Delete the database file if it exists
        let db_path = PathBuf::from("stream_manager.db");
        if db_path.exists() {
            fs::remove_file(&db_path)
                .map_err(|e| RecoveryError::IoError(e.to_string()))?;
        }

        // Database will be recreated on next connection
        info!("Database recreated successfully");
        Ok(())
    }

    /// Rebuild database from filesystem
    pub async fn rebuild_database_from_filesystem(&self) -> Result<(), RecoveryError> {
        let Some(database) = &self.database else {
            return Err(RecoveryError::NoDatabaseConfigured);
        };

        warn!("Rebuilding database from filesystem");

        // First, clear the recordings table
        sqlx::query!("DELETE FROM recordings")
            .execute(database.pool())
            .await
            .map_err(|e| RecoveryError::DatabaseError(e.to_string()))?;

        let mut files_added = 0;

        // Scan filesystem and rebuild database
        if self.recordings_path.exists() {
            // Walk through all subdirectories
            self.scan_directory_recursive(&self.recordings_path, database, &mut files_added).await?;
        }

        info!("Database rebuilt from filesystem: {} recordings added", files_added);
        Ok(())
    }

    /// Recursively scan directory for video files
    async fn scan_directory_recursive(
        &self,
        dir: &Path,
        database: &Arc<Database>,
        files_added: &mut usize,
    ) -> Result<(), RecoveryError> {
        for entry in fs::read_dir(dir)
            .map_err(|e| RecoveryError::IoError(e.to_string()))?
        {
            let entry = entry.map_err(|e| RecoveryError::IoError(e.to_string()))?;
            let path = entry.path();
            
            if path.is_dir() {
                // Recurse into subdirectory
                Box::pin(self.scan_directory_recursive(&path, database, files_added)).await?;
            } else if path.is_file() && is_video_file(&path) {
                // Parse information from file path
                let file_info = parse_recording_info(&path);
                
                // Add to database
                let metadata = fs::metadata(&path)
                    .map_err(|e| RecoveryError::IoError(e.to_string()))?;
                
                let file_path = path.to_string_lossy().to_string();
                let created_at = metadata.created()
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                    .unwrap_or_else(|_| chrono::Utc::now());
                
                sqlx::query!(
                    "INSERT INTO recordings (stream_id, file_path, size_bytes, created_at) VALUES (?, ?, ?, ?)",
                    file_info.stream_id,
                    file_path,
                    metadata.len() as i64,
                    created_at
                )
                .execute(database.pool())
                .await
                .map_err(|e| RecoveryError::DatabaseError(e.to_string()))?;
                
                *files_added += 1;
            }
        }
        Ok(())
    }

    /// Synchronize database with actual files on disk
    pub async fn sync_recordings(&self) -> Result<SyncResult, RecoveryError> {
        let Some(database) = &self.database else {
            return Err(RecoveryError::NoDatabaseConfigured);
        };

        let mut files_removed = 0;
        let mut files_added = 0;
        let mut db_entries_removed = 0;

        // Get all recording entries from database
        let db_recordings = sqlx::query!("SELECT id, file_path FROM recordings")
            .fetch_all(database.pool())
            .await
            .map_err(|e| RecoveryError::DatabaseError(e.to_string()))?;

        // Check each database entry against filesystem
        for record in db_recordings {
            if let Some(file_path) = record.file_path {
                let path = PathBuf::from(&file_path);
                if !path.exists() {
                    // File doesn't exist, remove from database
                    sqlx::query!("DELETE FROM recordings WHERE id = ?", record.id)
                        .execute(database.pool())
                        .await
                        .map_err(|e| RecoveryError::DatabaseError(e.to_string()))?;
                    
                    info!("Removed missing file from database: {}", file_path);
                    db_entries_removed += 1;
                }
            }
        }

        // Scan filesystem for recordings not in database
        if self.recordings_path.exists() {
            for entry in fs::read_dir(&self.recordings_path)
                .map_err(|e| RecoveryError::IoError(e.to_string()))?
            {
                let entry = entry.map_err(|e| RecoveryError::IoError(e.to_string()))?;
                let path = entry.path();
                
                if path.is_file() && is_video_file(&path) {
                    let file_path = path.to_string_lossy().to_string();
                    
                    // Check if file is in database
                    let exists = sqlx::query!("SELECT COUNT(*) as count FROM recordings WHERE file_path = ?", file_path)
                        .fetch_one(database.pool())
                        .await
                        .map_err(|e| RecoveryError::DatabaseError(e.to_string()))?
                        .count > 0;
                    
                    if !exists {
                        // Add to database
                        let metadata = fs::metadata(&path)
                            .map_err(|e| RecoveryError::IoError(e.to_string()))?;
                        
                        sqlx::query!(
                            "INSERT INTO recordings (stream_id, file_path, size_bytes, created_at) VALUES (?, ?, ?, ?)",
                            "unknown",
                            file_path,
                            metadata.len() as i64,
                            chrono::Utc::now()
                        )
                        .execute(database.pool())
                        .await
                        .map_err(|e| RecoveryError::DatabaseError(e.to_string()))?;
                        
                        info!("Added orphaned file to database: {}", file_path);
                        files_added += 1;
                    }
                }
            }
        }

        Ok(SyncResult {
            files_removed,
            files_added,
            db_entries_removed,
        })
    }

    /// Reset all recordings - delete files and clear database
    pub async fn reset_recordings(&self, confirm: bool) -> Result<ResetResult, RecoveryError> {
        if !confirm {
            return Err(RecoveryError::NotConfirmed);
        }

        let Some(database) = &self.database else {
            return Err(RecoveryError::NoDatabaseConfigured);
        };

        let mut files_deleted = 0;
        let mut db_entries_deleted = 0;

        // Delete all recording files
        if self.recordings_path.exists() {
            for entry in fs::read_dir(&self.recordings_path)
                .map_err(|e| RecoveryError::IoError(e.to_string()))?
            {
                let entry = entry.map_err(|e| RecoveryError::IoError(e.to_string()))?;
                let path = entry.path();
                
                if path.is_file() && is_video_file(&path) {
                    fs::remove_file(&path)
                        .map_err(|e| RecoveryError::IoError(e.to_string()))?;
                    files_deleted += 1;
                }
            }
        }

        // Clear database recordings table
        let result = sqlx::query!("DELETE FROM recordings")
            .execute(database.pool())
            .await
            .map_err(|e| RecoveryError::DatabaseError(e.to_string()))?;
        
        db_entries_deleted = result.rows_affected();

        info!("Reset complete: {} files deleted, {} database entries removed", 
              files_deleted, db_entries_deleted);

        Ok(ResetResult {
            files_deleted,
            db_entries_deleted,
        })
    }

    /// Start periodic integrity checks
    pub async fn start(&mut self) -> Result<(), RecoveryError> {
        let config = self.config.read().await;
        if !config.enabled {
            info!("Recovery manager is disabled");
            return Ok(());
        }

        let check_interval = config.check_interval_secs;
        drop(config);

        let manager = Arc::new(self.clone());
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(check_interval)
            );
            
            loop {
                interval.tick().await;
                
                // Check database integrity
                if let Err(e) = manager.check_database_integrity().await {
                    error!("Database integrity check failed: {}", e);
                }
                
                // Sync recordings
                match manager.sync_recordings().await {
                    Ok(result) if result.has_changes() => {
                        info!("Recording sync completed: {:?}", result);
                    }
                    Err(e) => {
                        error!("Recording sync failed: {}", e);
                    }
                    _ => {} // No changes
                }
            }
        });

        info!("Recovery manager started with {} second interval", check_interval);
        Ok(())
    }

    /// Get backup history (stub for compatibility)
    pub async fn get_backup_history(&self) -> Vec<BackupMetadata> {
        Vec::new()
    }

    /// Get recovery status (stub for compatibility)
    pub async fn get_recovery_status(&self) -> RecoveryStatus {
        RecoveryStatus {
            in_progress: false,
            backup_id: None,
            items_total: 0,
            items_restored: 0,
            errors: Vec::new(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Trigger backup (stub for compatibility)
    pub async fn trigger_backup(&self, _backup_type: BackupType) -> Result<String, BackupError> {
        Err(BackupError::NotImplemented("Backup functionality has been replaced with recovery mechanisms".to_string()))
    }

    /// Verify backup (stub for compatibility)
    pub async fn verify_backup(&self, _backup_id: &str) -> Result<bool, BackupError> {
        Err(BackupError::NotImplemented("Backup functionality has been replaced with recovery mechanisms".to_string()))
    }

    /// Restore from backup (stub for compatibility)
    pub async fn restore_from_backup(&self, _backup_id: &str) -> Result<(), BackupError> {
        Err(BackupError::NotImplemented("Backup functionality has been replaced with recovery mechanisms".to_string()))
    }

    /// Stop the recovery manager
    pub async fn stop(&self) {
        info!("Recovery manager stopped");
    }
}

impl Clone for BackupManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            database: self.database.clone(),
            recordings_path: self.recordings_path.clone(),
        }
    }
}

/// Check if a path is a video file
fn is_video_file(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => {
            let ext = ext.to_string_lossy().to_lowercase();
            matches!(ext.as_str(), "mp4" | "mkv" | "avi" | "mov" | "webm" | "ts" | "m3u8")
        }
        None => false,
    }
}

/// Information parsed from a recording file path
struct RecordingInfo {
    stream_id: String,
}

/// Parse recording information from file path
/// Expected formats:
/// - recordings/{stream_id}/{timestamp}_{segment}.mp4
/// - recordings/{stream_id}/{date}/{filename}.mp4
/// - recordings/{stream_id}_{timestamp}.mp4
/// - recordings/{filename}.mp4 (fallback to "unknown" stream)
fn parse_recording_info(path: &Path) -> RecordingInfo {
    // Try to extract stream_id from path
    let path_str = path.to_string_lossy();
    let parts: Vec<&str> = path_str.split(['/', '\\']).collect();
    
    // Look for "recordings" directory and take the next component as stream_id
    let stream_id = parts.iter()
        .position(|&p| p == "recordings")
        .and_then(|idx| parts.get(idx + 1))
        .map(|&s| {
            // If it looks like a date or timestamp, skip to next
            if s.len() == 10 && s.chars().all(|c| c.is_numeric() || c == '-') {
                parts.get(parts.iter().position(|&p| p == s).unwrap() + 1)
                    .map(|&s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                // Extract stream_id from filename if it contains underscore
                s.split('_').next().unwrap_or(s).to_string()
            }
        })
        .unwrap_or_else(|| {
            // Fallback: try to extract from filename
            path.file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.split('_').next())
                .unwrap_or("unknown")
                .to_string()
        });

    RecordingInfo { stream_id }
}

/// Result of a sync operation
#[derive(Debug)]
pub struct SyncResult {
    pub files_removed: usize,
    pub files_added: usize,
    pub db_entries_removed: usize,
}

impl SyncResult {
    pub fn has_changes(&self) -> bool {
        self.files_removed > 0 || self.files_added > 0 || self.db_entries_removed > 0
    }
}

/// Result of a reset operation
#[derive(Debug)]
pub struct ResetResult {
    pub files_deleted: usize,
    pub db_entries_deleted: u64,
}

/// Recovery error types
#[derive(Debug, thiserror::Error)]
pub enum RecoveryError {
    #[error("No database configured")]
    NoDatabaseConfigured,
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Operation not confirmed")]
    NotConfirmed,
}

/// Backup error types (for compatibility)
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Backup types (for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupType {
    Full,
    Incremental,
    Configuration,
    Database,
    Recovery,
}

/// Backup metadata (for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub id: String,
    pub timestamp: std::time::SystemTime,
    pub backup_type: BackupType,
}

/// Recovery status (for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStatus {
    pub in_progress: bool,
    pub backup_id: Option<String>,
    pub items_total: usize,
    pub items_restored: usize,
    pub errors: Vec<String>,
    pub started_at: Option<std::time::SystemTime>,
    pub completed_at: Option<std::time::SystemTime>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_recovery_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = BackupConfig::default();
        
        let manager = BackupManager::new(
            config,
            temp_dir.path().to_path_buf()
        );
        
        assert!(manager.get_backup_history().await.is_empty());
    }

    #[tokio::test]
    async fn test_reset_recordings() {
        let temp_dir = TempDir::new().unwrap();
        let recordings_path = temp_dir.path().join("recordings");
        fs::create_dir_all(&recordings_path).unwrap();
        
        // Create some test video files
        fs::write(recordings_path.join("test1.mp4"), "video data").unwrap();
        fs::write(recordings_path.join("test2.mkv"), "video data").unwrap();
        
        let config = BackupConfig::default();
        let manager = BackupManager::new(config, recordings_path);
        
        // Reset without confirmation should fail
        assert!(manager.reset_recordings(false).await.is_err());
        
        // Reset with confirmation should succeed
        let result = manager.reset_recordings(true).await.unwrap();
        assert_eq!(result.files_deleted, 2);
    }
}