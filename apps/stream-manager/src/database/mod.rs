use sqlx::{SqlitePool, sqlite::{SqlitePoolOptions, SqliteConnectOptions}, Row};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use serde_json;
use chrono::{DateTime, Utc, Local};

pub mod schema;
pub mod queries;
pub mod migrations;

pub use schema::*;
pub use queries::*;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    
    #[error("Migration error: {0}")]
    Migration(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

pub type Result<T> = std::result::Result<T, DatabaseError>;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub enable_wal: bool,
    pub enable_foreign_keys: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://stream_manager.db".to_string(),
            max_connections: 10,
            min_connections: 2,
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
            enable_wal: true,
            enable_foreign_keys: true,
        }
    }
}

pub struct Database {
    pool: SqlitePool,
    config: DatabaseConfig,
}

impl Database {
    /// Get access to the underlying SQLite pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        let pool = Self::create_pool(&config).await?;
        
        let mut db = Self { pool, config };
        
        // Run migrations
        db.run_migrations().await?;
        
        // Initialize database settings
        db.initialize_settings().await?;
        
        Ok(db)
    }

    pub async fn from_url(url: &str) -> Result<Self> {
        let config = DatabaseConfig {
            url: url.to_string(),
            ..Default::default()
        };
        Self::new(config).await
    }

    async fn create_pool(config: &DatabaseConfig) -> Result<SqlitePool> {
        let options = SqliteConnectOptions::new()
            .filename(&config.url.replace("sqlite://", ""))
            .create_if_missing(true)
            .journal_mode(if config.enable_wal {
                sqlx::sqlite::SqliteJournalMode::Wal
            } else {
                sqlx::sqlite::SqliteJournalMode::Delete
            })
            .foreign_keys(config.enable_foreign_keys)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.connect_timeout)
            .idle_timeout(Some(config.idle_timeout))
            .max_lifetime(Some(config.max_lifetime))
            .connect_with(options)
            .await?;

        Ok(pool)
    }

    async fn run_migrations(&mut self) -> Result<()> {
        info!("Running database migrations");
        
        // Use the internal migration system instead of sqlx::migrate! macro
        // since we're in a library context
        migrations::run_migrations(&self.pool).await?;
        
        info!("Database migrations completed");
        Ok(())
    }

    async fn initialize_settings(&mut self) -> Result<()> {
        // Enable WAL mode for better concurrency
        if self.config.enable_wal {
            sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&self.pool)
                .await?;
        }

        // Enable foreign keys
        if self.config.enable_foreign_keys {
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(&self.pool)
                .await?;
        }

        // Set synchronous mode for performance
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&self.pool)
            .await?;

        // Set cache size (negative value means KB)
        sqlx::query("PRAGMA cache_size = -64000")
            .execute(&self.pool)
            .await?;

        // Set temp store to memory
        sqlx::query("PRAGMA temp_store = MEMORY")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Stream operations
    pub async fn save_stream(&self, stream: &StreamRecord) -> Result<i64> {
        let config_json = serde_json::to_string(&stream.config)?;
        
        let result = sqlx::query(
            r#"
            INSERT INTO streams (id, uri, config, status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                uri = excluded.uri,
                config = excluded.config,
                status = excluded.status,
                updated_at = excluded.updated_at
            "#
        )
        .bind(&stream.id)
        .bind(&stream.uri)
        .bind(&config_json)
        .bind(&stream.status)
        .bind(stream.created_at)
        .bind(stream.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn get_stream(&self, id: &str) -> Result<StreamRecord> {
        let row = sqlx::query_as::<_, StreamRecordRow>(
            "SELECT * FROM streams WHERE id = ?1"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DatabaseError::NotFound(format!("Stream {} not found", id)),
            _ => e.into(),
        })?;

        row.try_into().map_err(DatabaseError::from)
    }

    pub async fn list_streams(&self, active_only: bool) -> Result<Vec<StreamRecord>> {
        let query = if active_only {
            "SELECT * FROM streams WHERE status = 'active' ORDER BY created_at DESC"
        } else {
            "SELECT * FROM streams ORDER BY created_at DESC"
        };

        let rows = sqlx::query_as::<_, StreamRecordRow>(query)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter()
            .map(|row| row.try_into().map_err(DatabaseError::from))
            .collect()
    }

    pub async fn update_stream_status(&self, id: &str, status: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sqlx::query(
            "UPDATE streams SET status = ?1, updated_at = ?2 WHERE id = ?3"
        )
        .bind(status)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_stream(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM streams WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Recording operations
    pub async fn save_recording(&self, recording: &RecordingRecord) -> Result<i64> {
        let metadata_json = recording.metadata.as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()?;

        let result = sqlx::query(
            r#"
            INSERT INTO recordings (
                id, stream_id, path, start_time, end_time,
                size_bytes, duration_ms, status, metadata
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#
        )
        .bind(&recording.id)
        .bind(&recording.stream_id)
        .bind(&recording.path)
        .bind(recording.start_time)
        .bind(recording.end_time)
        .bind(recording.size_bytes)
        .bind(recording.duration_ms)
        .bind(&recording.status)
        .bind(metadata_json)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn update_recording(&self, id: &str, end_time: i64, size_bytes: i64, duration_ms: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE recordings 
            SET end_time = ?1, size_bytes = ?2, duration_ms = ?3, status = 'completed'
            WHERE id = ?4
            "#
        )
        .bind(end_time)
        .bind(size_bytes)
        .bind(duration_ms)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_recordings(&self, stream_id: Option<&str>, limit: i64) -> Result<Vec<RecordingRecord>> {
        let query = if let Some(sid) = stream_id {
            sqlx::query_as::<_, RecordingRecordRow>(
                "SELECT * FROM recordings WHERE stream_id = ?1 ORDER BY start_time DESC LIMIT ?2"
            )
            .bind(sid)
            .bind(limit)
        } else {
            sqlx::query_as::<_, RecordingRecordRow>(
                "SELECT * FROM recordings ORDER BY start_time DESC LIMIT ?1"
            )
            .bind(limit)
        };

        let rows = query.fetch_all(&self.pool).await?;
        
        rows.into_iter()
            .map(|row| row.try_into().map_err(DatabaseError::from))
            .collect()
    }

    pub async fn cleanup_old_recordings(&self, days: i64) -> Result<u64> {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64 - (days * 86400);

        let result = sqlx::query("DELETE FROM recordings WHERE start_time < ?1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // State persistence - for application state that needs to survive restarts
    pub async fn save_state(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let value_str = serde_json::to_string(value)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sqlx::query(
            r#"
            INSERT INTO state (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at
            "#
        )
        .bind(key)
        .bind(&value_str)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_state(&self, key: &str) -> Result<serde_json::Value> {
        let row = sqlx::query("SELECT value FROM state WHERE key = ?1")
            .bind(key)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => DatabaseError::NotFound(format!("State key {} not found", key)),
                _ => e.into(),
            })?;

        let value_str: String = row.get(0);
        Ok(serde_json::from_str(&value_str)?)
    }

    pub async fn delete_state(&self, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM state WHERE key = ?1")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Database maintenance
    pub async fn vacuum(&self) -> Result<()> {
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn analyze(&self) -> Result<()> {
        sqlx::query("ANALYZE")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_database_size(&self) -> Result<i64> {
        let row = sqlx::query("SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get(0))
    }

    pub async fn checkpoint(&self) -> Result<()> {
        if self.config.enable_wal {
            sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    // Restore streams on startup
    pub async fn restore_active_streams(&self) -> Result<Vec<StreamRecord>> {
        self.list_streams(true).await
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_db() -> Result<Database> {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let url = format!("sqlite://{}", db_path.display());
        
        Database::from_url(&url).await
    }

    #[tokio::test]
    async fn test_database_creation() {
        let db = create_test_db().await;
        assert!(db.is_ok());
    }

    #[tokio::test]
    async fn test_stream_operations() {
        let db = create_test_db().await.unwrap();
        
        let stream = StreamRecord {
            id: "test-stream".to_string(),
            uri: "rtsp://example.com/stream".to_string(),
            config: serde_json::json!({"quality": "high"}),
            status: "active".to_string(),
            created_at: 1000,
            updated_at: 1000,
        };
        
        // Save stream
        let id = db.save_stream(&stream).await.unwrap();
        assert!(id >= 0);
        
        // Get stream
        let retrieved = db.get_stream("test-stream").await.unwrap();
        assert_eq!(retrieved.id, stream.id);
        assert_eq!(retrieved.uri, stream.uri);
        
        // Update status
        db.update_stream_status("test-stream", "stopped").await.unwrap();
        
        let updated = db.get_stream("test-stream").await.unwrap();
        assert_eq!(updated.status, "stopped");
        
        // List streams
        let streams = db.list_streams(false).await.unwrap();
        assert_eq!(streams.len(), 1);
        
        // Delete stream
        db.delete_stream("test-stream").await.unwrap();
        
        let result = db.get_stream("test-stream").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_recording_operations() {
        let db = create_test_db().await.unwrap();
        
        // First create a stream that the recording will reference
        let stream = StreamRecord {
            id: "stream-1".to_string(),
            uri: "rtsp://example.com/stream".to_string(),
            config: serde_json::json!({}),
            status: "active".to_string(),
            created_at: 1000,
            updated_at: 1000,
        };
        db.save_stream(&stream).await.unwrap();
        
        let recording = RecordingRecord {
            id: "rec-1".to_string(),
            stream_id: "stream-1".to_string(),
            path: "/recordings/rec-1.mp4".to_string(),
            start_time: 1000,
            end_time: Some(2000),
            size_bytes: Some(1024000),
            duration_ms: Some(1000),
            status: "completed".to_string(),
            metadata: Some(serde_json::json!({"format": "mp4"})),
        };
        
        // Save recording
        let id = db.save_recording(&recording).await.unwrap();
        assert!(id >= 0);
        
        // List recordings
        let recordings = db.list_recordings(Some("stream-1"), 10).await.unwrap();
        assert_eq!(recordings.len(), 1);
        assert_eq!(recordings[0].id, "rec-1");
    }

    #[tokio::test]
    async fn test_state_persistence() {
        let db = create_test_db().await.unwrap();
        
        let state = serde_json::json!({
            "last_stream_id": "stream-123",
            "total_recordings": 42
        });
        
        // Save state
        db.save_state("app_state", &state).await.unwrap();
        
        // Get state
        let retrieved = db.get_state("app_state").await.unwrap();
        assert_eq!(retrieved["last_stream_id"], "stream-123");
        assert_eq!(retrieved["total_recordings"], 42);
        
        // Delete state
        db.delete_state("app_state").await.unwrap();
        
        let result = db.get_state("app_state").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_old_recordings() {
        let db = create_test_db().await.unwrap();
        
        // First create a stream
        let stream = StreamRecord {
            id: "stream-1".to_string(),
            uri: "rtsp://example.com/stream".to_string(),
            config: serde_json::json!({}),
            status: "active".to_string(),
            created_at: 100,
            updated_at: 100,
        };
        db.save_stream(&stream).await.unwrap();
        
        // Add old recording
        let old_recording = RecordingRecord {
            id: "old-rec".to_string(),
            stream_id: "stream-1".to_string(),
            path: "/recordings/old.mp4".to_string(),
            start_time: 100, // Very old timestamp
            end_time: Some(200),
            size_bytes: Some(1024),
            duration_ms: Some(100),
            status: "completed".to_string(),
            metadata: None,
        };
        
        db.save_recording(&old_recording).await.unwrap();
        
        // Cleanup recordings older than 1 day
        let deleted = db.cleanup_old_recordings(1).await.unwrap();
        assert_eq!(deleted, 1);
        
        // Verify deletion
        let recordings = db.list_recordings(None, 10).await.unwrap();
        assert!(recordings.is_empty());
    }

    #[tokio::test]
    async fn test_database_maintenance() {
        let db = create_test_db().await.unwrap();
        
        // These should not fail
        db.vacuum().await.unwrap();
        db.analyze().await.unwrap();
        db.checkpoint().await.unwrap();
        
        // Check database size
        let size = db.get_database_size().await.unwrap();
        assert!(size > 0);
    }

    #[tokio::test]
    async fn test_restore_active_streams() {
        let db = create_test_db().await.unwrap();
        
        // Add active and inactive streams
        let active_stream = StreamRecord {
            id: "active-1".to_string(),
            uri: "rtsp://example.com/active".to_string(),
            config: serde_json::json!({}),
            status: "active".to_string(),
            created_at: 1000,
            updated_at: 1000,
        };
        
        let inactive_stream = StreamRecord {
            id: "inactive-1".to_string(),
            uri: "rtsp://example.com/inactive".to_string(),
            config: serde_json::json!({}),
            status: "stopped".to_string(),
            created_at: 1000,
            updated_at: 1000,
        };
        
        db.save_stream(&active_stream).await.unwrap();
        db.save_stream(&inactive_stream).await.unwrap();
        
        // Restore only active streams
        let restored = db.restore_active_streams().await.unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, "active-1");
    }
}