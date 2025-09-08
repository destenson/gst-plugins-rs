use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Stream {
    pub id: String,
    pub name: String,
    pub source_uri: String,
    pub status: StreamStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamStatus {
    Idle,
    Connecting,
    Active,
    Recording,
    Error(String),
}

pub struct StreamManager {
    streams: Arc<RwLock<Vec<Stream>>>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            streams: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_stream(&self, name: String, source_uri: String) -> crate::Result<String> {
        let id = Uuid::new_v4().to_string();
        let stream = Stream {
            id: id.clone(),
            name,
            source_uri,
            status: StreamStatus::Idle,
        };
        
        let mut streams = self.streams.write().await;
        streams.push(stream);
        
        Ok(id)
    }

    pub async fn get_stream(&self, id: &str) -> Option<Stream> {
        let streams = self.streams.read().await;
        streams.iter().find(|s| s.id == id).cloned()
    }

    pub async fn list_streams(&self) -> Vec<Stream> {
        self.streams.read().await.clone()
    }
}