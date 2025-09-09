use crate::api::websocket::{EventBroadcaster, EventType, WebSocketEvent};
use crate::manager::StreamEvent;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

/// Integrates the stream manager events with WebSocket broadcasting
pub struct EventIntegration {
    broadcaster: Arc<EventBroadcaster>,
}

impl EventIntegration {
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self { broadcaster }
    }
    
    /// Start listening to stream manager events
    pub fn start_stream_event_listener(
        &self,
        mut event_rx: mpsc::UnboundedReceiver<StreamEvent>,
    ) {
        let broadcaster = self.broadcaster.clone();
        
        tokio::spawn(async move {
            while let Some(stream_event) = event_rx.recv().await {
                let ws_event = match stream_event {
                    StreamEvent::StreamAdded(stream_id) => {
                        debug!("Broadcasting stream added event for {}", stream_id);
                        WebSocketEvent::new(
                            EventType::StreamAdded,
                            Some(stream_id),
                            serde_json::json!({}),
                        )
                    }
                    StreamEvent::StreamRemoved(stream_id) => {
                        debug!("Broadcasting stream removed event for {}", stream_id);
                        WebSocketEvent::new(
                            EventType::StreamRemoved,
                            Some(stream_id),
                            serde_json::json!({}),
                        )
                    }
                    StreamEvent::StreamHealthChanged(stream_id, health) => {
                        debug!("Broadcasting health changed event for {}", stream_id);
                        WebSocketEvent::new(
                            EventType::StreamHealthChanged,
                            Some(stream_id),
                            serde_json::json!({
                                "health": format!("{:?}", health),
                            }),
                        )
                    }
                    StreamEvent::StreamConnected(stream_id) => {
                        debug!("Broadcasting stream connected event for {}", stream_id);
                        WebSocketEvent::new(
                            EventType::StreamHealthChanged,
                            Some(stream_id),
                            serde_json::json!({
                                "status": "connected",
                            }),
                        )
                    }
                    StreamEvent::StreamReconnecting(stream_id) => {
                        debug!("Broadcasting stream reconnecting event for {}", stream_id);
                        WebSocketEvent::new(
                            EventType::StreamHealthChanged,
                            Some(stream_id),
                            serde_json::json!({
                                "status": "reconnecting",
                            }),
                        )
                    }
                    StreamEvent::StreamError(stream_id, error) => {
                        debug!("Broadcasting error event for {}", stream_id);
                        WebSocketEvent::new(
                            EventType::ErrorOccurred,
                            Some(stream_id),
                            serde_json::json!({
                                "error": error,
                            }),
                        )
                    }
                    StreamEvent::StatisticsUpdate(stream_id, _stats) => {
                        // Don't log statistics updates as they're too frequent
                        // Convert stats to a simpler format to avoid serialization issues
                        WebSocketEvent::new(
                            EventType::StatisticsUpdate,
                            Some(stream_id),
                            serde_json::json!({
                                "message": "Statistics updated",
                            }),
                        )
                    }
                    StreamEvent::ShutdownRequested => {
                        debug!("Broadcasting shutdown event");
                        WebSocketEvent::new(
                            EventType::SystemAlert,
                            None,
                            serde_json::json!({
                                "message": "System shutdown requested",
                                "severity": "warning",
                            }),
                        )
                    }
                };
                
                broadcaster.broadcast(ws_event);
            }
        });
    }
    
    /// Broadcast a system alert
    pub fn broadcast_system_alert(&self, message: String, severity: &str) {
        let event = WebSocketEvent::new(
            EventType::SystemAlert,
            None,
            serde_json::json!({
                "message": message,
                "severity": severity,
            }),
        );
        
        self.broadcaster.broadcast(event);
    }
    
    /// Broadcast a configuration change event
    pub fn broadcast_config_changed(&self) {
        let event = WebSocketEvent::new(
            EventType::ConfigChanged,
            None,
            serde_json::json!({
                "message": "Configuration has been updated",
            }),
        );
        
        self.broadcaster.broadcast(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_event_integration_creation() {
        let broadcaster = Arc::new(EventBroadcaster::new());
        let integration = EventIntegration::new(broadcaster);
        
        // Test system alert
        integration.broadcast_system_alert("Test alert".to_string(), "info");
        
        // Test config changed
        integration.broadcast_config_changed();
    }
}