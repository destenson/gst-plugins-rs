use serde::{Serialize, Deserialize};
use serde_json;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRecord {
    pub id: String,
    pub uri: String,
    pub config: serde_json::Value,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, FromRow)]
pub struct StreamRecordRow {
    pub id: String,
    pub uri: String,
    pub config: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl TryFrom<StreamRecordRow> for StreamRecord {
    type Error = serde_json::Error;

    fn try_from(row: StreamRecordRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            uri: row.uri,
            config: serde_json::from_str(&row.config)?,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingRecord {
    pub id: String,
    pub stream_id: String,
    pub path: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub size_bytes: Option<i64>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, FromRow)]
pub struct RecordingRecordRow {
    pub id: String,
    pub stream_id: String,
    pub path: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub size_bytes: Option<i64>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub metadata: Option<String>,
}

impl TryFrom<RecordingRecordRow> for RecordingRecord {
    type Error = serde_json::Error;

    fn try_from(row: RecordingRecordRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            stream_id: row.stream_id,
            path: row.path,
            start_time: row.start_time,
            end_time: row.end_time,
            size_bytes: row.size_bytes,
            duration_ms: row.duration_ms,
            status: row.status,
            metadata: row.metadata
                .map(|m| serde_json::from_str(&m))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateRecord {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: i64,
}

#[derive(Debug, FromRow)]
pub struct StateRecordRow {
    pub key: String,
    pub value: String,
    pub updated_at: i64,
}

impl TryFrom<StateRecordRow> for StateRecord {
    type Error = serde_json::Error;

    fn try_from(row: StateRecordRow) -> Result<Self, Self::Error> {
        Ok(Self {
            key: row.key,
            value: serde_json::from_str(&row.value)?,
            updated_at: row.updated_at,
        })
    }
}