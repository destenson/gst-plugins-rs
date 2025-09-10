#![allow(unused)]
use actix_web::web;
use gst::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::manager::StreamManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceConfig {
    pub stun_servers: Vec<String>,
    pub turn_servers: Vec<TurnServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalingMessage {
    #[serde(rename = "offer")]
    Offer { sdp: String, peer_id: String },
    #[serde(rename = "answer")]
    Answer { sdp: String, peer_id: String },
    #[serde(rename = "ice-candidate")]
    IceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u32>,
        peer_id: String,
    },
    #[serde(rename = "select-stream")]
    SelectStream { stream_id: String, peer_id: String },
    #[serde(rename = "disconnect")]
    Disconnect { peer_id: String },
}

#[derive(Debug)]
pub struct PeerConnection {
    pub id: String,
    pub pipeline: gst::Pipeline,
    pub webrtcbin: gst::Element,
    pub stream_id: Option<String>,
    pub ice_gathering_state: gst_webrtc::WebRTCICEGatheringState,
    pub connection_state: gst_webrtc::WebRTCPeerConnectionState,
}

pub struct WebRtcServer {
    peers: Arc<RwLock<HashMap<String, Arc<RwLock<PeerConnection>>>>>,
    ice_config: IceConfig,
    stream_manager: Arc<StreamManager>,
}

impl WebRtcServer {
    pub fn new(stream_manager: Arc<StreamManager>) -> Self {
        let ice_config = IceConfig {
            stun_servers: vec![
                "stun://stun.l.google.com:19302".to_string(),
                "stun://stun1.l.google.com:19302".to_string(),
            ],
            turn_servers: vec![],
        };

        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            ice_config,
            stream_manager,
        }
    }

    pub fn with_ice_config(mut self, config: IceConfig) -> Self {
        self.ice_config = config;
        self
    }

    pub async fn handle_signaling_message(
        &self,
        message: SignalingMessage,
    ) -> Result<Option<SignalingMessage>, Box<dyn std::error::Error>> {
        match message {
            SignalingMessage::Offer { sdp, peer_id } => {
                info!("Received offer from peer {}", peer_id);
                self.handle_offer(&peer_id, &sdp).await
            }
            SignalingMessage::IceCandidate {
                candidate,
                sdp_mid,
                sdp_mline_index,
                peer_id,
            } => {
                info!("Received ICE candidate from peer {}", peer_id);
                self.handle_ice_candidate(&peer_id, &candidate, sdp_mid, sdp_mline_index)
                    .await?;
                Ok(None)
            }
            SignalingMessage::SelectStream { stream_id, peer_id } => {
                info!("Peer {} selecting stream {}", peer_id, stream_id);
                self.select_stream(&peer_id, &stream_id).await?;
                Ok(None)
            }
            SignalingMessage::Disconnect { peer_id } => {
                info!("Peer {} disconnecting", peer_id);
                self.disconnect_peer(&peer_id).await?;
                Ok(None)
            }
            _ => {
                warn!("Unhandled signaling message type");
                Ok(None)
            }
        }
    }

    async fn handle_offer(
        &self,
        peer_id: &str,
        sdp: &str,
    ) -> Result<Option<SignalingMessage>, Box<dyn std::error::Error>> {
        let peer = self.create_peer_connection(peer_id).await?;
        
        let webrtcbin = peer.read().await.webrtcbin.clone();
        
        // Set remote description (offer)
        let offer = gst_sdp::SDPMessage::parse_buffer(sdp.as_bytes())?;
        let offer_webrtc = gst_webrtc::WebRTCSessionDescription::new(
            gst_webrtc::WebRTCSDPType::Offer,
            offer,
        );
        webrtcbin.emit_by_name::<()>("set-remote-description", &[&offer_webrtc]);

        // Create answer
        let promise = gst::Promise::with_change_func(move |reply| {
            if let Ok(Some(reply)) = reply {
                if let Ok(answer) = reply.value("answer") {
                    debug!("Answer created: {:?}", answer);
                }
            }
        });
        
        webrtcbin.emit_by_name::<()>("create-answer", &[&None::<gst::Structure>, &promise]);
        
        // Wait for answer to be created
        let answer = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.wait_for_answer(&webrtcbin),
        )
        .await??;

        Ok(Some(SignalingMessage::Answer {
            sdp: answer,
            peer_id: peer_id.to_string(),
        }))
    }

    async fn wait_for_answer(
        &self,
        webrtcbin: &gst::Element,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // This is simplified - in production, you'd use proper async signaling
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let local_desc = webrtcbin
            .property::<Option<gst_webrtc::WebRTCSessionDescription>>("local-description")
            .ok_or("No local description")?;
        
        Ok(local_desc.sdp().to_string())
    }

    async fn handle_ice_candidate(
        &self,
        peer_id: &str,
        candidate: &str,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let peers = self.peers.read().await;
        let peer = peers
            .get(peer_id)
            .ok_or_else(|| format!("Peer {} not found", peer_id))?;
        
        let peer = peer.read().await;
        let mline_index = sdp_mline_index.unwrap_or(0);
        
        peer.webrtcbin
            .emit_by_name::<()>("add-ice-candidate", &[&mline_index, &candidate]);
        
        Ok(())
    }

    async fn create_peer_connection(
        &self,
        peer_id: &str,
    ) -> Result<Arc<RwLock<PeerConnection>>, Box<dyn std::error::Error>> {
        let pipeline = gst::Pipeline::new();
        
        // Create webrtcbin element
        let webrtcbin = gst::ElementFactory::make("webrtcbin")
            .property_from_str("bundle-policy", "max-bundle")
            .property("stun-server", &self.ice_config.stun_servers[0])
            .name(&format!("webrtcbin-{}", peer_id))
            .build()?;

        // Setup ICE gathering
        webrtcbin.connect("on-ice-candidate", false, {
            let peer_id = peer_id.to_string();
            move |values| {
                let _webrtcbin = values[0].get::<gst::Element>().unwrap();
                let mline_index = values[1].get::<u32>().unwrap();
                let candidate = values[2].get::<String>().unwrap();
                
                info!(
                    "ICE candidate for peer {}: {} (mline: {})",
                    peer_id, candidate, mline_index
                );
                
                None
            }
        });

        // Setup connection state monitoring
        webrtcbin.connect_notify(Some("connection-state"), {
            let peer_id = peer_id.to_string();
            move |webrtcbin, _pspec| {
                let state = webrtcbin
                    .property::<gst_webrtc::WebRTCPeerConnectionState>("connection-state");
                info!("Peer {} connection state: {:?}", peer_id, state);
            }
        });

        // Setup ICE gathering state monitoring
        webrtcbin.connect_notify(Some("ice-gathering-state"), {
            let peer_id = peer_id.to_string();
            move |webrtcbin, _pspec| {
                let state = webrtcbin
                    .property::<gst_webrtc::WebRTCICEGatheringState>("ice-gathering-state");
                info!("Peer {} ICE gathering state: {:?}", peer_id, state);
            }
        });

        // Create test source for now (will be replaced with actual stream)
        let videotestsrc = gst::ElementFactory::make("videotestsrc")
            .property("is-live", true)
            .property_from_str("pattern", "smpte")
            .build()?;

        let videoconvert = gst::ElementFactory::make("videoconvert").build()?;
        
        let vp8enc = gst::ElementFactory::make("vp8enc")
            .property("deadline", 1i64)
            .property("target-bitrate", 1000000i32)
            .build()?;

        let rtpvp8pay = gst::ElementFactory::make("rtpvp8pay")
            .property("pt", 96u32)
            .build()?;

        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("application/x-rtp")
                    .field("media", "video")
                    .field("encoding-name", "VP8")
                    .field("payload", 96i32)
                    .build(),
            )
            .build()?;

        // Add elements to pipeline
        pipeline.add_many([
            &videotestsrc,
            &videoconvert,
            &vp8enc,
            &rtpvp8pay,
            &capsfilter,
            &webrtcbin,
        ])?;

        // Link elements
        gst::Element::link_many([
            &videotestsrc,
            &videoconvert,
            &vp8enc,
            &rtpvp8pay,
            &capsfilter,
        ])?;

        capsfilter.link_pads(Some("src"), &webrtcbin, Some("sink_0"))?;

        // Start pipeline
        pipeline.set_state(gst::State::Playing)?;

        let peer = Arc::new(RwLock::new(PeerConnection {
            id: peer_id.to_string(),
            pipeline,
            webrtcbin,
            stream_id: None,
            ice_gathering_state: gst_webrtc::WebRTCICEGatheringState::New,
            connection_state: gst_webrtc::WebRTCPeerConnectionState::New,
        }));

        let mut peers = self.peers.write().await;
        peers.insert(peer_id.to_string(), peer.clone());

        Ok(peer)
    }

    async fn select_stream(
        &self,
        peer_id: &str,
        stream_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let peers = self.peers.read().await;
        let peer = peers
            .get(peer_id)
            .ok_or_else(|| format!("Peer {} not found", peer_id))?;

        let mut peer = peer.write().await;
        peer.stream_id = Some(stream_id.to_string());

        // TODO: Connect to actual stream from stream manager
        // For now, just log the selection
        info!("Peer {} selected stream {}", peer_id, stream_id);

        Ok(())
    }

    pub async fn disconnect_peer(&self, peer_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut peers = self.peers.write().await;
        
        if let Some(peer) = peers.remove(peer_id) {
            let peer = peer.read().await;
            peer.pipeline.set_state(gst::State::Null)?;
            info!("Disconnected peer {}", peer_id);
        }

        Ok(())
    }

    pub async fn get_peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    pub async fn get_peer_info(&self, peer_id: &str) -> Option<PeerInfo> {
        let peers = self.peers.read().await;
        if let Some(peer) = peers.get(peer_id) {
            let peer = peer.read().await;
            Some(PeerInfo {
                id: peer.id.clone(),
                stream_id: peer.stream_id.clone(),
                ice_gathering_state: format!("{:?}", peer.ice_gathering_state),
                connection_state: format!("{:?}", peer.connection_state),
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerInfo {
    pub id: String,
    pub stream_id: Option<String>,
    pub ice_gathering_state: String,
    pub connection_state: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webrtc_server_creation() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let manager = Arc::new(StreamManager::new(config).unwrap());
        let server = WebRtcServer::new(manager);
        assert_eq!(server.get_peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_ice_config() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let manager = Arc::new(StreamManager::new(config).unwrap());
        let ice_config = IceConfig {
            stun_servers: vec!["stun://custom.stun.server:3478".to_string()],
            turn_servers: vec![TurnServer {
                urls: vec!["turn://turn.server:3478".to_string()],
                username: Some("user".to_string()),
                credential: Some("pass".to_string()),
            }],
        };
        let server = WebRtcServer::new(manager).with_ice_config(ice_config.clone());
        assert_eq!(server.ice_config.stun_servers.len(), 1);
        assert_eq!(server.ice_config.turn_servers.len(), 1);
    }

    #[tokio::test]
    async fn test_peer_connection_creation() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let manager = Arc::new(StreamManager::new(config).unwrap());
        let server = WebRtcServer::new(manager);
        
        let peer = server.create_peer_connection("test-peer").await.unwrap();
        assert_eq!(server.get_peer_count().await, 1);
        
        let peer = peer.read().await;
        assert_eq!(peer.id, "test-peer");
        assert!(peer.stream_id.is_none());
    }

    #[tokio::test]
    async fn test_disconnect_peer() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let manager = Arc::new(StreamManager::new(config).unwrap());
        let server = WebRtcServer::new(manager);
        
        server.create_peer_connection("test-peer").await.unwrap();
        assert_eq!(server.get_peer_count().await, 1);
        
        server.disconnect_peer("test-peer").await.unwrap();
        assert_eq!(server.get_peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_select_stream() {
        gst::init().unwrap();
        let config = Arc::new(crate::Config::default());
        let manager = Arc::new(StreamManager::new(config).unwrap());
        let server = WebRtcServer::new(manager);
        
        server.create_peer_connection("test-peer").await.unwrap();
        server.select_stream("test-peer", "stream-1").await.unwrap();
        
        let info = server.get_peer_info("test-peer").await.unwrap();
        assert_eq!(info.stream_id, Some("stream-1".to_string()));
    }
}
