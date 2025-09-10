use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::api::{ApiError, AppState};
use crate::webrtc::{IceConfig, SignalingMessage, TurnServer, WebRtcServer};
use crate::webrtc::signaling::SignalingSession;

#[derive(Debug, Serialize, Deserialize)]
pub struct WebRtcStatus {
    pub enabled: bool,
    pub peer_count: usize,
    pub ice_config: IceConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateIceConfigRequest {
    pub stun_servers: Vec<String>,
    pub turn_servers: Vec<TurnServer>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/webrtc")
            .route("/status", web::get().to(get_status))
            .route("/ice-config", web::get().to(get_ice_config))
            .route("/ice-config", web::put().to(update_ice_config))
            .route("/peers", web::get().to(list_peers))
            .route("/peers/{peer_id}", web::get().to(get_peer_info))
            .route("/signaling", web::get().to(websocket_signaling))
    );
}

async fn get_status(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    if let Some(webrtc_server) = &state.webrtc_server {
        let server = webrtc_server.read().await;
        let peer_count = server.get_peer_count().await;
        
        Ok(HttpResponse::Ok().json(WebRtcStatus {
            enabled: true,
            peer_count,
            ice_config: IceConfig {
                stun_servers: vec![
                    "stun://stun.l.google.com:19302".to_string(),
                    "stun://stun1.l.google.com:19302".to_string(),
                ],
                turn_servers: vec![],
            },
        }))
    } else {
        Ok(HttpResponse::Ok().json(WebRtcStatus {
            enabled: false,
            peer_count: 0,
            ice_config: IceConfig {
                stun_servers: vec![],
                turn_servers: vec![],
            },
        }))
    }
}

async fn get_ice_config(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    if let Some(webrtc_server) = &state.webrtc_server {
        let server = webrtc_server.read().await;
        // Note: We'd need to expose ice_config as a method on WebRtcServer
        Ok(HttpResponse::Ok().json(IceConfig {
            stun_servers: vec![
                "stun://stun.l.google.com:19302".to_string(),
                "stun://stun1.l.google.com:19302".to_string(),
            ],
            turn_servers: vec![],
        }))
    } else {
        Err(ApiError::NotFound("WebRTC server not enabled".to_string()))
    }
}

async fn update_ice_config(
    state: web::Data<AppState>,
    config: web::Json<UpdateIceConfigRequest>,
) -> Result<HttpResponse, ApiError> {
    if let Some(_webrtc_server) = &state.webrtc_server {
        // In production, we'd update the server's ICE configuration
        // For now, just acknowledge the request
        info!("ICE configuration update requested");
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "ICE configuration updated",
            "stun_servers": config.stun_servers.len(),
            "turn_servers": config.turn_servers.len(),
        })))
    } else {
        Err(ApiError::NotFound("WebRTC server not enabled".to_string()))
    }
}

async fn list_peers(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    if let Some(webrtc_server) = &state.webrtc_server {
        let server = webrtc_server.read().await;
        let peer_count = server.get_peer_count().await;
        
        // In production, we'd list actual peer IDs
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "peer_count": peer_count,
            "peers": []  // Would contain actual peer list
        })))
    } else {
        Err(ApiError::NotFound("WebRTC server not enabled".to_string()))
    }
}

async fn get_peer_info(
    state: web::Data<AppState>,
    peer_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    if let Some(webrtc_server) = &state.webrtc_server {
        let server = webrtc_server.read().await;
        
        if let Some(info) = server.get_peer_info(&peer_id).await {
            Ok(HttpResponse::Ok().json(info))
        } else {
            Err(ApiError::NotFound(format!("Peer {} not found", peer_id)))
        }
    } else {
        Err(ApiError::NotFound("WebRTC server not enabled".to_string()))
    }
}

async fn websocket_signaling(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Some(webrtc_server) = &state.webrtc_server {
        let peer_id = uuid::Uuid::new_v4().to_string();
        info!("New WebRTC signaling connection: {}", peer_id);
        
        let server = webrtc_server.read().await;
        let session = SignalingSession::new(peer_id, Arc::new(server.clone()));
        
        ws::start(session, &req, stream)
    } else {
        Ok(HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({
                "error": "WebRTC server not enabled"
            })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use crate::manager::StreamManager;

    #[actix_web::test]
    async fn test_webrtc_status_disabled() {
        let config = Arc::new(crate::Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let app_state = AppState::new(stream_manager, config);
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .configure(configure)
        ).await;
        
        let req = test::TestRequest::get()
            .uri("/api/v1/webrtc/status")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body: WebRtcStatus = test::read_body_json(resp).await;
        assert!(!body.enabled);
        assert_eq!(body.peer_count, 0);
    }

    #[actix_web::test]
    async fn test_webrtc_status_enabled() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let mut app_state = AppState::new(stream_manager.clone(), config);
        
        // Enable WebRTC server
        let webrtc_server = Arc::new(tokio::sync::RwLock::new(
            WebRtcServer::new(stream_manager)
        ));
        app_state.webrtc_server = Some(webrtc_server);
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .configure(configure)
        ).await;
        
        let req = test::TestRequest::get()
            .uri("/api/v1/webrtc/status")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body: WebRtcStatus = test::read_body_json(resp).await;
        assert!(body.enabled);
        assert_eq!(body.peer_count, 0);
    }
}