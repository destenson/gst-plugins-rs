use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_ws::{Message, Session};
use actix_rt;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// WebSocket event types that can be broadcast to clients
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventType {
    StreamAdded,
    StreamRemoved,
    StreamHealthChanged,
    RecordingStarted,
    RecordingStopped,
    StatisticsUpdate,
    SystemAlert,
    ConfigChanged,
    ErrorOccurred,
}

/// Event that can be sent over WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketEvent {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: EventType,
    pub stream_id: Option<String>,
    pub data: serde_json::Value,
}

impl WebSocketEvent {
    pub fn new(event_type: EventType, stream_id: Option<String>, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            event_type,
            stream_id,
            data,
        }
    }
}

/// Client subscription configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionRequest {
    pub event_types: Option<Vec<EventType>>,
    pub stream_ids: Option<Vec<String>>,
}

/// WebSocket client connection state
pub struct WebSocketClient {
    id: String,
    session: Arc<RwLock<Session>>,
    subscriptions: Arc<RwLock<ClientSubscriptions>>,
    last_ping: Instant,
    event_sender: mpsc::UnboundedSender<WebSocketEvent>,
}

#[derive(Debug, Default)]
struct ClientSubscriptions {
    event_types: HashSet<EventType>,
    stream_ids: HashSet<String>,
}

impl ClientSubscriptions {
    fn should_receive(&self, event: &WebSocketEvent) -> bool {
        // Check event type subscription
        let event_type_match = self.event_types.is_empty() || 
            self.event_types.contains(&event.event_type);
        
        // Check stream ID subscription
        let stream_id_match = self.stream_ids.is_empty() || 
            event.stream_id.as_ref().map_or(true, |id| self.stream_ids.contains(id));
        
        event_type_match && stream_id_match
    }
}

/// WebSocket event broadcaster that manages all client connections
pub struct EventBroadcaster {
    clients: Arc<RwLock<Vec<Arc<WebSocketClient>>>>,
    event_tx: mpsc::UnboundedSender<WebSocketEvent>,
    event_rx: Arc<RwLock<mpsc::UnboundedReceiver<WebSocketEvent>>>,
}

impl EventBroadcaster {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Self {
            clients: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
        }
    }
    
    /// Start the event broadcasting task
    pub fn start(&self) {
        let clients = self.clients.clone();
        let event_rx = self.event_rx.clone();
        
        tokio::spawn(async move {
            let mut rx = event_rx.write().await;
            
            while let Some(event) = rx.recv().await {
                let clients = clients.read().await;
                
                for client in clients.iter() {
                    // Check if client should receive this event
                    let subscriptions = client.subscriptions.read().await;
                    if subscriptions.should_receive(&event) {
                        // Send event to client's queue
                        if let Err(e) = client.event_sender.send(event.clone()) {
                            warn!("Failed to send event to client {}: {}", client.id, e);
                        }
                    }
                }
            }
        });
        
        info!("WebSocket event broadcaster started");
    }
    
    /// Broadcast an event to all subscribed clients
    pub fn broadcast(&self, event: WebSocketEvent) {
        if let Err(e) = self.event_tx.send(event) {
            error!("Failed to broadcast event: {}", e);
        }
    }
    
    /// Add a new client connection
    pub async fn add_client(&self, client: Arc<WebSocketClient>) {
        let mut clients = self.clients.write().await;
        clients.push(client);
        debug!("Added WebSocket client, total: {}", clients.len());
    }
    
    /// Remove a client connection
    pub async fn remove_client(&self, client_id: &str) {
        let mut clients = self.clients.write().await;
        clients.retain(|c| c.id != client_id);
        debug!("Removed WebSocket client {}, remaining: {}", client_id, clients.len());
    }
    
    /// Get the number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

/// WebSocket connection handler
pub async fn websocket_handler(
    req: HttpRequest,
    body: web::Payload,
    broadcaster: web::Data<Arc<EventBroadcaster>>,
) -> Result<HttpResponse, Error> {
    let (response, mut session, stream) = actix_ws::handle(&req, body)?;
    
    let client_id = Uuid::new_v4().to_string();
    info!("New WebSocket connection: {}", client_id);
    
    // Create event channel for this client
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    
    let client = Arc::new(WebSocketClient {
        id: client_id.clone(),
        session: Arc::new(RwLock::new(session.clone())),
        subscriptions: Arc::new(RwLock::new(ClientSubscriptions::default())),
        last_ping: Instant::now(),
        event_sender: event_tx,
    });
    
    // Add client to broadcaster
    broadcaster.add_client(client.clone()).await;
    
    // Spawn task to handle incoming messages from client
    let client_for_incoming = client.clone();
    let broadcaster_for_incoming = broadcaster.clone();
    actix_rt::spawn(async move {
        handle_client_messages(stream, client_for_incoming, broadcaster_for_incoming).await;
    });
    
    // Spawn task to handle outgoing events to client
    let client_for_outgoing = client.clone();
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let msg_text = match serde_json::to_string(&event) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize event: {}", e);
                    continue;
                }
            };
            
            let mut session = client_for_outgoing.session.write().await;
            if let Err(e) = session.text(msg_text).await {
                error!("Failed to send event to client {}: {}", client_for_outgoing.id, e);
                break;
            }
        }
    });
    
    // Send welcome message
    let welcome = WebSocketEvent::new(
        EventType::SystemAlert,
        None,
        serde_json::json!({
            "message": "Connected to Stream Manager WebSocket",
            "client_id": client_id,
        }),
    );
    
    if let Ok(welcome_json) = serde_json::to_string(&welcome) {
        let _ = session.text(welcome_json).await;
    }
    
    Ok(response)
}

async fn handle_client_messages(
    mut stream: actix_ws::MessageStream,
    client: Arc<WebSocketClient>,
    broadcaster: web::Data<Arc<EventBroadcaster>>,
) {
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received text message from {}: {}", client.id, text);
                
                // Try to parse as subscription request
                if let Ok(subscription) = serde_json::from_str::<SubscriptionRequest>(&text) {
                    handle_subscription(client.clone(), subscription).await;
                }
            }
            Ok(Message::Ping(bytes)) => {
                debug!("Received ping from {}", client.id);
                let mut session = client.session.write().await;
                let _ = session.pong(&bytes).await;
            }
            Ok(Message::Pong(_)) => {
                debug!("Received pong from {}", client.id);
            }
            Ok(Message::Close(reason)) => {
                info!("Client {} closing connection: {:?}", client.id, reason);
                break;
            }
            Err(e) => {
                error!("WebSocket error for client {}: {}", client.id, e);
                break;
            }
            _ => {}
        }
    }
    
    // Remove client on disconnect
    broadcaster.remove_client(&client.id).await;
    info!("WebSocket client {} disconnected", client.id);
}

async fn handle_subscription(client: Arc<WebSocketClient>, request: SubscriptionRequest) {
    let mut subscriptions = client.subscriptions.write().await;
    
    // Update event type subscriptions
    if let Some(event_types) = request.event_types {
        subscriptions.event_types = event_types.into_iter().collect();
        debug!("Client {} subscribed to event types: {:?}", client.id, subscriptions.event_types);
    }
    
    // Update stream ID subscriptions
    if let Some(stream_ids) = request.stream_ids {
        subscriptions.stream_ids = stream_ids.into_iter().collect();
        debug!("Client {} subscribed to streams: {:?}", client.id, subscriptions.stream_ids);
    }
    
    // Send confirmation
    let confirmation = WebSocketEvent::new(
        EventType::SystemAlert,
        None,
        serde_json::json!({
            "message": "Subscription updated",
            "event_types": subscriptions.event_types.iter().collect::<Vec<_>>(),
            "stream_ids": subscriptions.stream_ids.iter().collect::<Vec<_>>(),
        }),
    );
    
    if let Err(e) = client.event_sender.send(confirmation) {
        error!("Failed to send subscription confirmation: {}", e);
    }
}

/// Configure WebSocket routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/ws", web::get().to(websocket_handler));
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::App;
    
    #[test]
    fn test_event_creation() {
        let event = WebSocketEvent::new(
            EventType::StreamAdded,
            Some("stream-1".to_string()),
            serde_json::json!({"name": "Test Stream"}),
        );
        
        assert_eq!(event.event_type, EventType::StreamAdded);
        assert_eq!(event.stream_id, Some("stream-1".to_string()));
        assert!(!event.id.is_empty());
    }
    
    #[test]
    fn test_subscription_filtering() {
        let mut subs = ClientSubscriptions::default();
        subs.event_types.insert(EventType::StreamAdded);
        subs.stream_ids.insert("stream-1".to_string());
        
        // Should receive: matching event type and stream
        let event1 = WebSocketEvent::new(
            EventType::StreamAdded,
            Some("stream-1".to_string()),
            serde_json::json!({}),
        );
        assert!(subs.should_receive(&event1));
        
        // Should not receive: wrong event type
        let event2 = WebSocketEvent::new(
            EventType::StreamRemoved,
            Some("stream-1".to_string()),
            serde_json::json!({}),
        );
        assert!(!subs.should_receive(&event2));
        
        // Should not receive: wrong stream ID
        let event3 = WebSocketEvent::new(
            EventType::StreamAdded,
            Some("stream-2".to_string()),
            serde_json::json!({}),
        );
        assert!(!subs.should_receive(&event3));
    }
    
    #[tokio::test]
    async fn test_event_broadcaster() {
        let broadcaster = EventBroadcaster::new();
        
        // Initially no clients
        assert_eq!(broadcaster.client_count().await, 0);
        
        // Broadcast an event (should not crash even with no clients)
        let event = WebSocketEvent::new(
            EventType::SystemAlert,
            None,
            serde_json::json!({"test": true}),
        );
        broadcaster.broadcast(event);
    }
    
    #[actix_web::test]
    async fn test_websocket_upgrade() {
        let broadcaster = Arc::new(EventBroadcaster::new());
        
        let app = actix_web::test::init_service(
            App::new()
                .app_data(web::Data::new(broadcaster))
                .configure(configure)
        ).await;
        
        let req = actix_web::test::TestRequest::get()
            .uri("/ws")
            .insert_header(("Connection", "Upgrade"))
            .insert_header(("Upgrade", "websocket"))
            .insert_header(("Sec-WebSocket-Version", "13"))
            .insert_header(("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ=="))
            .to_request();
        
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), 101); // Switching Protocols
    }
}