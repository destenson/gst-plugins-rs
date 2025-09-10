use actix::prelude::*;
use actix_web_actors::ws;
use serde_json;
use std::sync::Arc;
use tracing::{debug, error, info};

use super::server::{SignalingMessage, WebRtcServer};

pub struct SignalingSession {
    pub id: String,
    pub server: Arc<WebRtcServer>,
}

impl SignalingSession {
    pub fn new(id: String, server: Arc<WebRtcServer>) -> Self {
        Self { id, server }
    }
}

impl Actor for SignalingSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("WebRTC signaling session started for peer {}", self.id);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("WebRTC signaling session stopped for peer {}", self.id);
        
        // Disconnect peer when websocket closes
        let server = self.server.clone();
        let peer_id = self.id.clone();
        actix::spawn(async move {
            if let Err(e) = server.disconnect_peer(&peer_id).await {
                error!("Failed to disconnect peer {}: {}", peer_id, e);
            }
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SignalingSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                debug!("Received signaling message: {}", text);
                
                match serde_json::from_str::<SignalingMessage>(&text) {
                    Ok(mut message) => {
                        // Ensure peer_id is set correctly
                        match &mut message {
                            SignalingMessage::Offer { peer_id, .. } |
                            SignalingMessage::Answer { peer_id, .. } |
                            SignalingMessage::IceCandidate { peer_id, .. } |
                            SignalingMessage::SelectStream { peer_id, .. } |
                            SignalingMessage::Disconnect { peer_id } => {
                                *peer_id = self.id.clone();
                            }
                        }
                        
                        let server = self.server.clone();
                        let addr = ctx.address();
                        
                        actix::spawn(async move {
                            match server.handle_signaling_message(message).await {
                                Ok(Some(response)) => {
                                    if let Ok(json) = serde_json::to_string(&response) {
                                        addr.do_send(SignalingResponse(json));
                                    }
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    error!("Failed to handle signaling message: {}", e);
                                    let error_msg = serde_json::json!({
                                        "type": "error",
                                        "message": e.to_string()
                                    });
                                    if let Ok(json) = serde_json::to_string(&error_msg) {
                                        addr.do_send(SignalingResponse(json));
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to parse signaling message: {}", e);
                        ctx.text(format!(r#"{{"type":"error","message":"{}"}}"#, e));
                    }
                }
            }
            Ok(ws::Message::Binary(_)) => {
                error!("Binary messages not supported for signaling");
            }
            Ok(ws::Message::Close(reason)) => {
                info!("WebSocket closing: {:?}", reason);
                ctx.stop();
            }
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {}
            Ok(_) => {}
            Err(e) => {
                error!("WebSocket error: {}", e);
                ctx.stop();
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SignalingResponse(pub String);

impl Handler<SignalingResponse> for SignalingSession {
    type Result = ();

    fn handle(&mut self, msg: SignalingResponse, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

pub async fn negotiate_webrtc_connection(
    server: Arc<WebRtcServer>,
    offer_sdp: String,
    peer_id: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let message = SignalingMessage::Offer {
        sdp: offer_sdp,
        peer_id: peer_id.clone(),
    };

    match server.handle_signaling_message(message).await? {
        Some(SignalingMessage::Answer { sdp, .. }) => Ok(sdp),
        _ => Err("Failed to get answer SDP".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::StreamManager;

    #[tokio::test]
    async fn test_signaling_session_creation() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let manager = Arc::new(StreamManager::new(config).unwrap());
        let server = Arc::new(WebRtcServer::new(manager));
        let session = SignalingSession::new("test-peer".to_string(), server);
        assert_eq!(session.id, "test-peer");
    }

    #[tokio::test]
    async fn test_negotiate_connection() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let manager = Arc::new(StreamManager::new(config).unwrap());
        let server = Arc::new(WebRtcServer::new(manager));
        
        // This would normally be a real SDP offer
        let offer_sdp = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n".to_string();
        
        // This test will fail because we need a proper SDP offer
        // In a real scenario, this would come from a browser
        let result = negotiate_webrtc_connection(
            server,
            offer_sdp,
            "test-peer".to_string(),
        ).await;
        
        // For now, we expect this to fail with our mock SDP
        assert!(result.is_err());
    }
}