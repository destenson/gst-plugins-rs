#![allow(unused)]
use actix_web::{http::StatusCode, web, HttpRequest, HttpResponse};
use gst::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::api::ApiError;
use crate::manager::StreamManager;
use crate::webrtc::WebRtcServer;

#[derive(Debug, Clone)]
pub struct WhipWhepSession {
    pub id: String,
    pub stream_id: String,
    pub session_type: SessionType,
    pub sdp_answer: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub peer_connection: Option<Arc<gst::Pipeline>>,
    pub ice_candidates: Vec<IceCandidate>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionType {
    Whip,  // Ingestion
    Whep,  // Playback
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u32>,
}

pub struct WhipWhepHandler {
    sessions: Arc<RwLock<HashMap<String, WhipWhepSession>>>,
    webrtc_server: Arc<WebRtcServer>,
    stream_manager: Arc<StreamManager>,
}

impl WhipWhepHandler {
    pub fn new(
        webrtc_server: Arc<WebRtcServer>,
        stream_manager: Arc<StreamManager>,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            webrtc_server,
            stream_manager,
        }
    }

    // WHIP Ingestion - POST /whip/{stream_id}
    pub async fn handle_whip_post(
        &self,
        stream_id: String,
        sdp_offer: String,
        auth_token: Option<String>,
    ) -> Result<HttpResponse, ApiError> {
        info!("WHIP ingestion request for stream {}", stream_id);

        // Validate authentication if provided
        if let Some(token) = auth_token {
            if !self.validate_auth_token(&token).await {
                return Err(ApiError::Unauthorized("Invalid authentication token".to_string()));
            }
        }

        // Create session ID
        let session_id = Uuid::new_v4().to_string();

        // Create WebRTC pipeline for ingestion
        let pipeline = self.create_whip_pipeline(&stream_id, &sdp_offer).await?;

        // Generate SDP answer
        let sdp_answer = self.generate_sdp_answer(&sdp_offer, SessionType::Whip).await?;

        // Store session
        let session = WhipWhepSession {
            id: session_id.clone(),
            stream_id: stream_id.clone(),
            session_type: SessionType::Whip,
            sdp_answer: sdp_answer.clone(),
            created_at: chrono::Utc::now(),
            peer_connection: Some(Arc::new(pipeline)),
            ice_candidates: Vec::new(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        // Return 201 Created with Location header
        Ok(HttpResponse::Created()
            .insert_header(("Location", format!("/whip/{}/{}", stream_id, session_id)))
            .insert_header(("Content-Type", "application/sdp"))
            .body(sdp_answer))
    }

    // WHEP Playback - POST /whep/{stream_id}
    pub async fn handle_whep_post(
        &self,
        stream_id: String,
        sdp_offer: String,
        auth_token: Option<String>,
    ) -> Result<HttpResponse, ApiError> {
        info!("WHEP playback request for stream {}", stream_id);

        // Validate authentication if provided
        if let Some(token) = auth_token {
            if !self.validate_auth_token(&token).await {
                return Err(ApiError::Unauthorized("Invalid authentication token".to_string()));
            }
        }

        // Check if stream exists
        let streams = self.stream_manager.list_streams().await;
        if !streams.iter().any(|s| s.id == stream_id) {
            return Err(ApiError::NotFound(format!("Stream {} not found", stream_id)));
        }

        // Create session ID
        let session_id = Uuid::new_v4().to_string();

        // Create WebRTC pipeline for playback
        let pipeline = self.create_whep_pipeline(&stream_id, &sdp_offer).await?;

        // Generate SDP answer
        let sdp_answer = self.generate_sdp_answer(&sdp_offer, SessionType::Whep).await?;

        // Store session
        let session = WhipWhepSession {
            id: session_id.clone(),
            stream_id: stream_id.clone(),
            session_type: SessionType::Whep,
            sdp_answer: sdp_answer.clone(),
            created_at: chrono::Utc::now(),
            peer_connection: Some(Arc::new(pipeline)),
            ice_candidates: Vec::new(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        // Return 201 Created with Location header
        Ok(HttpResponse::Created()
            .insert_header(("Location", format!("/whep/{}/{}", stream_id, session_id)))
            .insert_header(("Content-Type", "application/sdp"))
            .body(sdp_answer))
    }

    // DELETE endpoint for teardown
    pub async fn handle_delete(
        &self,
        session_id: String,
    ) -> Result<HttpResponse, ApiError> {
        info!("Teardown request for session {}", session_id);

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(&session_id) {
            // Stop the pipeline
            if let Some(pipeline) = session.peer_connection {
                if let Err(e) = pipeline.set_state(gst::State::Null) {
                    error!("Failed to stop pipeline: {}", e);
                }
            }

            Ok(HttpResponse::NoContent().finish())
        } else {
            Err(ApiError::NotFound(format!("Session {} not found", session_id)))
        }
    }

    // PATCH endpoint for ICE trickle
    pub async fn handle_patch(
        &self,
        session_id: String,
        content_type: String,
        body: String,
    ) -> Result<HttpResponse, ApiError> {
        debug!("PATCH request for session {}: {}", session_id, content_type);

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            if content_type.contains("application/trickle-ice-sdpfrag") {
                // Parse ICE candidate from SDP fragment
                let candidate = self.parse_ice_candidate_from_sdp_fragment(&body)?;
                session.ice_candidates.push(candidate);
                
                // Apply ICE candidate to pipeline
                // TODO: Implement ICE candidate application
                
                Ok(HttpResponse::NoContent().finish())
            } else {
                Err(ApiError::BadRequest("Unsupported content type for PATCH".to_string()))
            }
        } else {
            Err(ApiError::NotFound(format!("Session {} not found", session_id)))
        }
    }

    async fn create_whip_pipeline(
        &self,
        stream_id: &str,
        sdp_offer: &str,
    ) -> Result<gst::Pipeline, ApiError> {
        let pipeline = gst::Pipeline::new();
        
        // Create webrtcbin element
        let webrtcbin = gst::ElementFactory::make("webrtcbin")
            .property("bundle-policy", "max-bundle")
            .name(&format!("whip-webrtcbin-{}", stream_id))
            .build()
            .map_err(|e| ApiError::InternalError(format!("Failed to create webrtcbin: {}", e)))?;

        // Parse and set remote description
        let offer = gst_sdp::SDPMessage::parse_buffer(sdp_offer.as_bytes())
            .map_err(|e| ApiError::BadRequest(format!("Invalid SDP offer: {}", e)))?;
        
        let offer_webrtc = gst_webrtc::WebRTCSessionDescription::new(
            gst_webrtc::WebRTCSDPType::Offer,
            offer,
        );
        
        webrtcbin.emit_by_name::<()>("set-remote-description", &[&offer_webrtc]);

        // Create decoding pipeline
        let decodebin = gst::ElementFactory::make("decodebin")
            .name(&format!("whip-decode-{}", stream_id))
            .build()
            .map_err(|e| ApiError::InternalError(format!("Failed to create decodebin: {}", e)))?;

        // Connect to stream manager for output
        // This would connect to the stream's tee element for recording/inference
        
        pipeline.add_many([&webrtcbin, &decodebin])
            .map_err(|e| ApiError::InternalError(format!("Failed to add elements: {}", e)))?;

        // Link webrtcbin to decodebin
        webrtcbin.connect_pad_added(move |_webrtc, pad| {
            let sink_pad = decodebin.static_pad("sink").unwrap();
            if !sink_pad.is_linked() {
                pad.link(&sink_pad).ok();
            }
        });

        pipeline.set_state(gst::State::Playing)
            .map_err(|e| ApiError::InternalError(format!("Failed to start pipeline: {}", e)))?;

        Ok(pipeline)
    }

    async fn create_whep_pipeline(
        &self,
        stream_id: &str,
        sdp_offer: &str,
    ) -> Result<gst::Pipeline, ApiError> {
        let pipeline = gst::Pipeline::new();
        
        // Create webrtcbin element
        let webrtcbin = gst::ElementFactory::make("webrtcbin")
            .property("bundle-policy", "max-bundle")
            .name(&format!("whep-webrtcbin-{}", stream_id))
            .build()
            .map_err(|e| ApiError::InternalError(format!("Failed to create webrtcbin: {}", e)))?;

        // Parse and set remote description
        let offer = gst_sdp::SDPMessage::parse_buffer(sdp_offer.as_bytes())
            .map_err(|e| ApiError::BadRequest(format!("Invalid SDP offer: {}", e)))?;
        
        let offer_webrtc = gst_webrtc::WebRTCSessionDescription::new(
            gst_webrtc::WebRTCSDPType::Offer,
            offer,
        );
        
        webrtcbin.emit_by_name::<()>("set-remote-description", &[&offer_webrtc]);

        // For WHEP, we need to get the stream from stream manager
        // and encode it for WebRTC
        
        // Create test source for now
        let videotestsrc = gst::ElementFactory::make("videotestsrc")
            .property("is-live", true)
            .build()
            .map_err(|e| ApiError::InternalError(format!("Failed to create source: {}", e)))?;

        let videoconvert = gst::ElementFactory::make("videoconvert").build()
            .map_err(|e| ApiError::InternalError(format!("Failed to create videoconvert: {}", e)))?;

        let vp8enc = gst::ElementFactory::make("vp8enc")
            .property("deadline", 1i64)
            .build()
            .map_err(|e| ApiError::InternalError(format!("Failed to create encoder: {}", e)))?;

        let rtpvp8pay = gst::ElementFactory::make("rtpvp8pay")
            .property("pt", 96u32)
            .build()
            .map_err(|e| ApiError::InternalError(format!("Failed to create payloader: {}", e)))?;

        pipeline.add_many([&videotestsrc, &videoconvert, &vp8enc, &rtpvp8pay, &webrtcbin])
            .map_err(|e| ApiError::InternalError(format!("Failed to add elements: {}", e)))?;

        gst::Element::link_many([&videotestsrc, &videoconvert, &vp8enc, &rtpvp8pay])
            .map_err(|e| ApiError::InternalError(format!("Failed to link elements: {}", e)))?;

        rtpvp8pay.link_pads(Some("src"), &webrtcbin, Some("sink_0"))
            .map_err(|e| ApiError::InternalError(format!("Failed to link to webrtcbin: {}", e)))?;

        pipeline.set_state(gst::State::Playing)
            .map_err(|e| ApiError::InternalError(format!("Failed to start pipeline: {}", e)))?;

        Ok(pipeline)
    }

    async fn generate_sdp_answer(
        &self,
        sdp_offer: &str,
        session_type: SessionType,
    ) -> Result<String, ApiError> {
        // For now, return a simple SDP answer
        // In production, this would be generated by webrtcbin
        let answer = format!(
            "v=0\r\n\
            o=- {} 0 IN IP4 0.0.0.0\r\n\
            s=Stream Manager {}\r\n\
            t=0 0\r\n\
            a=group:BUNDLE 0\r\n\
            m=video 9 UDP/TLS/RTP/SAVPF 96\r\n\
            c=IN IP4 0.0.0.0\r\n\
            a=rtcp:9 IN IP4 0.0.0.0\r\n\
            a=ice-ufrag:4cPF\r\n\
            a=ice-pwd:by5GZGG1lw+040DWA6hXM5Bz\r\n\
            a=ice-options:trickle\r\n\
            a=fingerprint:sha-256 DE:AD:BE:EF:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00\r\n\
            a=setup:passive\r\n\
            a=mid:0\r\n\
            a=sendonly\r\n\
            a=rtcp-mux\r\n\
            a=rtcp-rsize\r\n\
            a=rtpmap:96 VP8/90000\r\n",
            chrono::Utc::now().timestamp(),
            if session_type == SessionType::Whip { "WHIP" } else { "WHEP" }
        );

        Ok(answer)
    }

    async fn validate_auth_token(&self, token: &str) -> bool {
        // Simple bearer token validation
        // In production, this would check against a token store or auth service
        !token.is_empty() && token.len() >= 32
    }

    fn parse_ice_candidate_from_sdp_fragment(&self, fragment: &str) -> Result<IceCandidate, ApiError> {
        // Parse ICE candidate from SDP fragment format
        // Example: a=candidate:1 1 UDP 2130706431 192.168.1.1 54321 typ host
        
        if fragment.starts_with("a=candidate:") {
            let candidate = fragment.trim_start_matches("a=").to_string();
            Ok(IceCandidate {
                candidate,
                sdp_mid: None,
                sdp_mline_index: Some(0),
            })
        } else {
            Err(ApiError::BadRequest("Invalid ICE candidate format".to_string()))
        }
    }

    pub async fn get_session_info(&self, session_id: &str) -> Option<WhipWhepSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn list_sessions(&self) -> Vec<WhipWhepSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }
}

// HTTP endpoint handlers
pub async fn whip_post(
    req: HttpRequest,
    stream_id: web::Path<String>,
    body: String,
    handler: web::Data<Arc<WhipWhepHandler>>,
) -> Result<HttpResponse, ApiError> {
    let auth_token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    handler.handle_whip_post(stream_id.into_inner(), body, auth_token).await
}

pub async fn whep_post(
    req: HttpRequest,
    stream_id: web::Path<String>,
    body: String,
    handler: web::Data<Arc<WhipWhepHandler>>,
) -> Result<HttpResponse, ApiError> {
    let auth_token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    handler.handle_whep_post(stream_id.into_inner(), body, auth_token).await
}

pub async fn session_delete(
    path: web::Path<(String, String)>,
    handler: web::Data<Arc<WhipWhepHandler>>,
) -> Result<HttpResponse, ApiError> {
    let (_protocol, session_id) = path.into_inner();
    handler.handle_delete(session_id).await
}

pub async fn session_patch(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: String,
    handler: web::Data<Arc<WhipWhepHandler>>,
) -> Result<HttpResponse, ApiError> {
    let (_protocol, session_id) = path.into_inner();
    let content_type = req
        .headers()
        .get("Content-Type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();

    handler.handle_patch(session_id, content_type, body).await
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/whip/{stream_id}", web::post().to(whip_post))
            .route("/whep/{stream_id}", web::post().to(whep_post))
            .route("/whip/{stream_id}/{session_id}", web::delete().to(session_delete))
            .route("/whep/{stream_id}/{session_id}", web::delete().to(session_delete))
            .route("/whip/{stream_id}/{session_id}", web::patch().to(session_patch))
            .route("/whep/{stream_id}/{session_id}", web::patch().to(session_patch))
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_whip_whep_handler_creation() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let webrtc_server = Arc::new(WebRtcServer::new(stream_manager.clone()));
        
        let handler = WhipWhepHandler::new(webrtc_server, stream_manager);
        let sessions = handler.list_sessions().await;
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_auth_token_validation() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let webrtc_server = Arc::new(WebRtcServer::new(stream_manager.clone()));
        
        let handler = WhipWhepHandler::new(webrtc_server, stream_manager);
        
        // Valid token (32+ characters)
        assert!(handler.validate_auth_token("12345678901234567890123456789012").await);
        
        // Invalid token (too short)
        assert!(!handler.validate_auth_token("short").await);
        
        // Empty token
        assert!(!handler.validate_auth_token("").await);
    }

    #[tokio::test]
    async fn test_ice_candidate_parsing() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let webrtc_server = Arc::new(WebRtcServer::new(stream_manager.clone()));
        
        let handler = WhipWhepHandler::new(webrtc_server, stream_manager);
        
        let fragment = "a=candidate:1 1 UDP 2130706431 192.168.1.1 54321 typ host";
        let result = handler.parse_ice_candidate_from_sdp_fragment(fragment);
        
        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert_eq!(candidate.candidate, "candidate:1 1 UDP 2130706431 192.168.1.1 54321 typ host");
        assert_eq!(candidate.sdp_mline_index, Some(0));
    }

    #[tokio::test]
    async fn test_sdp_answer_generation() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let webrtc_server = Arc::new(WebRtcServer::new(stream_manager.clone()));
        
        let handler = WhipWhepHandler::new(webrtc_server, stream_manager);
        
        let sdp_offer = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n";
        let result = handler.generate_sdp_answer(sdp_offer, SessionType::Whip).await;
        
        assert!(result.is_ok());
        let answer = result.unwrap();
        assert!(answer.contains("v=0"));
        assert!(answer.contains("Stream Manager"));
    }
}
