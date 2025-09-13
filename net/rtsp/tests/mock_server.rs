#![allow(unused)]
// GStreamer RTSP plugin mock server for testing
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use rtsp_types::{headers, Method, Response, StatusCode, Version};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// Mock RTSP server for testing rtspsrc2
pub struct MockRtspServer {
    listener: TcpListener,
    local_addr: SocketAddr,
    sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    sdp_content: String,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Option<mpsc::Receiver<()>>,
}

#[derive(Debug, Clone)]
struct SessionState {
    session_id: String,
    created_at: SystemTime,
    streams: Vec<StreamSetup>,
    state: PlaybackState,
}

#[derive(Debug, Clone)]
struct StreamSetup {
    stream_id: usize,
    client_rtp_port: u16,
    client_rtcp_port: u16,
}

#[derive(Debug, Clone, PartialEq)]
enum PlaybackState {
    Init,
    Ready,
    Playing,
}

impl MockRtspServer {
    /// Create a new mock RTSP server
    pub async fn new() -> Self {
        Self::new_with_port(0).await
    }

    /// Create a new mock RTSP server on a specific port
    pub async fn new_with_port(port: u16) -> Self {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to bind to port");

        let local_addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        // Default SDP for a simple H264 video stream
        let sdp_content = Self::default_sdp(&local_addr);

        Self {
            listener,
            local_addr,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            sdp_content,
            shutdown_tx,
            shutdown_rx: Some(shutdown_rx),
        }
    }

    /// Get the server's address
    pub fn addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Get the RTSP URL for the test stream
    pub fn url(&self) -> String {
        format!("rtsp://{}/test", self.local_addr)
    }

    /// Set custom SDP content
    pub fn set_sdp(&mut self, sdp: String) {
        self.sdp_content = sdp;
    }

    /// Start the server (consumes self)
    pub async fn start(mut self) -> ServerHandle {
        let sessions = self.sessions.clone();
        let sdp_content = self.sdp_content.clone();
        let mut shutdown_rx = self.shutdown_rx.take().unwrap();
        let shutdown_tx = self.shutdown_tx.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    accept_result = self.listener.accept() => {
                        if let Ok((stream, _addr)) = accept_result {
                            let sessions = sessions.clone();
                            let sdp_content = sdp_content.clone();

                            tokio::spawn(async move {
                                if let Err(e) = handle_client(stream, sessions, sdp_content).await {
                                    eprintln!("Error handling client: {}", e);
                                }
                            });
                        }
                    }
                }
            }
        });

        ServerHandle {
            shutdown_tx,
            local_addr: self.local_addr,
        }
    }

    fn default_sdp(addr: &SocketAddr) -> String {
        format!(
            "v=0\r\n\
            o=- 0 0 IN IP4 {}\r\n\
            s=Test Stream\r\n\
            c=IN IP4 {}\r\n\
            t=0 0\r\n\
            a=tool:GStreamer\r\n\
            a=type:broadcast\r\n\
            a=control:rtsp://{}/test\r\n\
            m=video 0 RTP/AVP 96\r\n\
            a=rtpmap:96 H264/90000\r\n\
            a=fmtp:96 packetization-mode=1;profile-level-id=42C01E\r\n\
            a=control:stream=0\r\n",
            addr.ip(),
            addr.ip(),
            addr
        )
    }
}

/// Handle to control the running server
pub struct ServerHandle {
    shutdown_tx: mpsc::Sender<()>,
    local_addr: SocketAddr,
}

impl ServerHandle {
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(()).await;
    }

    pub fn addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn url(&self) -> String {
        format!("rtsp://{}/test", self.local_addr)
    }
}

async fn handle_client(
    stream: TcpStream,
    sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    sdp_content: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();
    let mut next_session_num = 1u32;

    loop {
        buffer.clear();

        // Read request line
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            break; // Connection closed
        }
        buffer.extend_from_slice(line.as_bytes());

        // Parse the request line to get method
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        // Read headers until empty line
        loop {
            line.clear();
            reader.read_line(&mut line).await?;
            buffer.extend_from_slice(line.as_bytes());
            if line.trim().is_empty() {
                break;
            }
        }

        // Parse the complete request
        let request = match rtsp_types::Message::<Vec<u8>>::parse(&buffer) {
            Ok((msg, _)) => {
                if let rtsp_types::Message::Request(req) = msg {
                    req
                } else {
                    continue;
                }
            }
            Err(_) => continue,
        };

        // Extract CSeq for response
        let cseq = request
            .headers()
            .find(|(name, _)| name == &headers::CSEQ)
            .map(|(_, value)| value.as_str())
            .unwrap_or("1");

        // Handle the request based on method
        let response = match request.method() {
            Method::Options => Response::builder(Version::V1_0, StatusCode::Ok)
                .header(headers::CSEQ, cseq)
                .header(headers::PUBLIC, "OPTIONS, DESCRIBE, SETUP, PLAY, TEARDOWN")
                .build(Vec::new()),
            Method::Describe => {
                let sdp_bytes = sdp_content.as_bytes().to_vec();
                Response::builder(Version::V1_0, StatusCode::Ok)
                    .header(headers::CSEQ, cseq)
                    .header(headers::CONTENT_TYPE, "application/sdp")
                    .header(headers::CONTENT_LENGTH, sdp_bytes.len().to_string())
                    .build(sdp_bytes)
            }
            Method::Setup => {
                // Parse transport header
                let transport = request
                    .headers()
                    .find(|(name, _)| name == &headers::TRANSPORT)
                    .map(|(_, value)| value.as_str())
                    .unwrap_or("RTP/AVP;unicast;client_port=5002-5003");

                // Generate session ID
                let session_id = format!("session{}", next_session_num);
                next_session_num += 1;

                // Create session state
                let session_state = SessionState {
                    session_id: session_id.clone(),
                    created_at: SystemTime::now(),
                    streams: vec![],
                    state: PlaybackState::Ready,
                };

                sessions
                    .lock()
                    .unwrap()
                    .insert(session_id.clone(), session_state);

                // Build transport response with server ports
                let response_transport = format!(
                    "{};server_port=5000-5001;client_port=5002-5003",
                    transport.split(';').next().unwrap_or("RTP/AVP")
                );

                Response::builder(Version::V1_0, StatusCode::Ok)
                    .header(headers::CSEQ, cseq)
                    .header(headers::SESSION, session_id.as_str())
                    .header(headers::TRANSPORT, response_transport)
                    .build(Vec::new())
            }
            Method::Play => {
                // Get session from header
                if let Some(session_id) = request
                    .headers()
                    .find(|(name, _)| name == &headers::SESSION)
                    .map(|(_, value)| value.as_str())
                {
                    // Update session state
                    if let Some(session) = sessions.lock().unwrap().get_mut(session_id) {
                        session.state = PlaybackState::Playing;
                    }

                    Response::builder(Version::V1_0, StatusCode::Ok)
                        .header(headers::CSEQ, cseq)
                        .header(headers::SESSION, session_id)
                        .build(Vec::new())
                } else {
                    Response::builder(Version::V1_0, StatusCode::BadRequest)
                        .header(headers::CSEQ, cseq)
                        .build(Vec::new())
                }
            }
            Method::Teardown => {
                // Get session from header and remove it
                if let Some(session_id) = request
                    .headers()
                    .find(|(name, _)| name == &headers::SESSION)
                    .map(|(_, value)| value.as_str())
                {
                    sessions.lock().unwrap().remove(session_id);
                }

                Response::builder(Version::V1_0, StatusCode::Ok)
                    .header(headers::CSEQ, cseq)
                    .build(Vec::new())
            }
            Method::GetParameter => {
                // Handle GET_PARAMETER request
                let body = request.body();
                let response_body = if !body.is_empty() {
                    // Parse requested parameters and return dummy values
                    let params_str = String::from_utf8_lossy(body);
                    let mut response_params = Vec::new();
                    for line in params_str.lines() {
                        if !line.trim().is_empty() {
                            // Return dummy values for requested parameters
                            response_params.push(format!("{}: dummy_value", line.trim()));
                        }
                    }
                    response_params.join("\r\n").into_bytes()
                } else {
                    // Empty body for keep-alive
                    Vec::new()
                };
                
                Response::builder(Version::V1_0, StatusCode::Ok)
                    .header(headers::CSEQ, cseq)
                    .header(headers::CONTENT_TYPE, "text/parameters")
                    .header(headers::CONTENT_LENGTH, response_body.len().to_string())
                    .build(response_body)
            }
            Method::SetParameter => {
                // Handle SET_PARAMETER request
                // Just acknowledge the parameters were set
                Response::builder(Version::V1_0, StatusCode::Ok)
                    .header(headers::CSEQ, cseq)
                    .build(Vec::new())
            }
            _ => {
                // Method not implemented
                Response::builder(Version::V1_0, StatusCode::NotImplemented)
                    .header(headers::CSEQ, cseq)
                    .build(Vec::new())
            }
        };

        // Send response
        let mut response_bytes = Vec::new();
        response.write(&mut response_bytes).unwrap();
        writer.write_all(&response_bytes).await?;
        writer.flush().await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_startup() {
        let server = MockRtspServer::new().await;
        let addr = server.addr();
        assert!(addr.port() > 0);

        let handle = server.start().await;

        // Try to connect to ensure it's listening
        let stream = TcpStream::connect(addr).await;
        assert!(stream.is_ok());

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_options_request() {
        let server = MockRtspServer::new().await;
        let addr = server.addr();
        let handle = server.start().await;

        let mut stream = TcpStream::connect(addr).await.unwrap();

        // Send OPTIONS request
        let request = format!(
            "OPTIONS rtsp://{}/test RTSP/1.0\r\n\
             CSeq: 1\r\n\
             \r\n",
            addr
        );
        stream.write_all(request.as_bytes()).await.unwrap();

        // Read response
        let mut buffer = vec![0u8; 1024];
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);

        assert!(response.contains("RTSP/1.0 200 Ok"));
        assert!(response.contains("Public:"));

        handle.shutdown().await;
    }
}
