use sqlx::{SqlitePool, Row};
use serde_json;
use std::collections::HashMap;
use super::{DatabaseError, Result};

pub struct QueryBuilder {
    pool: SqlitePool,
}

impl QueryBuilder {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // Aggregate queries for recordings
    pub async fn get_recording_stats(&self, stream_id: Option<&str>) -> Result<RecordingStats> {
        let query = if let Some(sid) = stream_id {
            sqlx::query(
                r#"
                SELECT 
                    COUNT(*) as total_count,
                    SUM(size_bytes) as total_size,
                    SUM(duration_ms) as total_duration,
                    AVG(size_bytes) as avg_size,
                    AVG(duration_ms) as avg_duration
                FROM recordings
                WHERE stream_id = ?1 AND status = 'completed'
                "#
            )
            .bind(sid)
        } else {
            sqlx::query(
                r#"
                SELECT 
                    COUNT(*) as total_count,
                    SUM(size_bytes) as total_size,
                    SUM(duration_ms) as total_duration,
                    AVG(size_bytes) as avg_size,
                    AVG(duration_ms) as avg_duration
                FROM recordings
                WHERE status = 'completed'
                "#
            )
        };

        let row = query.fetch_one(&self.pool).await?;

        Ok(RecordingStats {
            total_count: row.get(0),
            total_size_bytes: row.get::<Option<i64>, _>(1).unwrap_or(0),
            total_duration_ms: row.get::<Option<i64>, _>(2).unwrap_or(0),
            avg_size_bytes: row.get::<Option<f64>, _>(3).unwrap_or(0.0),
            avg_duration_ms: row.get::<Option<f64>, _>(4).unwrap_or(0.0),
        })
    }

    // Search recordings by path pattern
    pub async fn search_recordings(&self, pattern: &str, limit: i64) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT id, path
            FROM recordings
            WHERE path LIKE ?1
            ORDER BY start_time DESC
            LIMIT ?2
            "#
        )
        .bind(format!("%{}%", pattern))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            results.push(format!("{}: {}", row.get::<String, _>(0), row.get::<String, _>(1)));
        }

        Ok(results)
    }

    // Get storage usage per stream
    pub async fn get_storage_by_stream(&self) -> Result<Vec<StorageUsage>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                stream_id,
                COUNT(*) as recording_count,
                SUM(size_bytes) as total_size
            FROM recordings
            WHERE status = 'completed'
            GROUP BY stream_id
            ORDER BY total_size DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut usage = Vec::new();
        for row in rows {
            usage.push(StorageUsage {
                stream_id: row.get(0),
                recording_count: row.get(1),
                total_size_bytes: row.get::<Option<i64>, _>(2).unwrap_or(0),
            });
        }

        Ok(usage)
    }

    // Get recording gaps (useful for detecting missing segments)
    pub async fn find_recording_gaps(&self, stream_id: &str, threshold_seconds: i64) -> Result<Vec<RecordingGap>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                r1.id as prev_id,
                r1.end_time as prev_end,
                r2.id as next_id,
                r2.start_time as next_start
            FROM recordings r1
            INNER JOIN recordings r2 ON r1.stream_id = r2.stream_id
            WHERE r1.stream_id = ?1
                AND r1.end_time IS NOT NULL
                AND r2.start_time > r1.end_time
                AND (r2.start_time - r1.end_time) > ?2
                AND NOT EXISTS (
                    SELECT 1 FROM recordings r3
                    WHERE r3.stream_id = r1.stream_id
                    AND r3.start_time > r1.end_time
                    AND r3.start_time < r2.start_time
                )
            ORDER BY r1.end_time
            "#
        )
        .bind(stream_id)
        .bind(threshold_seconds)
        .fetch_all(&self.pool)
        .await?;

        let mut gaps = Vec::new();
        for row in rows {
            let prev_end: i64 = row.get(1);
            let next_start: i64 = row.get(3);
            
            gaps.push(RecordingGap {
                previous_recording_id: row.get(0),
                next_recording_id: row.get(2),
                gap_start: prev_end,
                gap_end: next_start,
                gap_duration_seconds: next_start - prev_end,
            });
        }

        Ok(gaps)
    }
}

#[derive(Debug, Clone)]
pub struct RecordingStats {
    pub total_count: i64,
    pub total_size_bytes: i64,
    pub total_duration_ms: i64,
    pub avg_size_bytes: f64,
    pub avg_duration_ms: f64,
}

#[derive(Debug, Clone)]
pub struct StorageUsage {
    pub stream_id: String,
    pub recording_count: i64,
    pub total_size_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct RecordingGap {
    pub previous_recording_id: String,
    pub next_recording_id: String,
    pub gap_start: i64,
    pub gap_end: i64,
    pub gap_duration_seconds: i64,
}