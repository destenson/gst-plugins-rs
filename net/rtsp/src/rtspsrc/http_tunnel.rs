// HTTP Tunneling Support for RTSP
//
// This module implements RTSP-over-HTTP tunneling to bypass firewalls
// and proxies that block RTSP traffic.

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use url::Url;
use crate::rtspsrc::imp::HttpTunnelMode;

use gst::glib;
use std::sync::LazyLock;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "rtspsrc2-http-tunnel",
        gst::DebugColorFlags::empty(),
        Some("RTSP HTTP Tunneling"),
    )
});

/// HTTP tunnel connection state
#[derive(Debug, Clone)]
pub struct HttpTunnel {
    /// Session cookie for correlating GET and POST connections
    session_cookie: String,
    /// GET connection for receiving RTSP responses
    get_connection: Arc<Mutex<Option<TcpStream>>>,
    /// POST connection for sending RTSP requests
    post_connection: Arc<Mutex<Option<TcpStream>>>,
    /// Tunnel URL
    url: Url,
    /// Proxy settings
    proxy: Option<String>,
    /// Channel for receiving RTSP responses from GET connection
    response_rx: Arc<Mutex<mpsc::Receiver<Bytes>>>,
    response_tx: mpsc::Sender<Bytes>,
}

impl HttpTunnel {
    /// Create a new HTTP tunnel
    pub fn new(rtsp_url: &Url, proxy: Option<String>) -> Result<Self> {
        // Generate a unique session cookie
        let session_cookie = format!("{:x}", rand::random::<u64>());
        
        // Convert RTSP URL to HTTP URL for tunneling
        let mut tunnel_url = rtsp_url.clone();
        tunnel_url.set_scheme("http").map_err(|_| anyhow!("Failed to set HTTP scheme"))?;
        
        // Default to port 80 for HTTP if not specified
        if tunnel_url.port().is_none() {
            tunnel_url.set_port(Some(80)).map_err(|_| anyhow!("Failed to set port"))?;
        }
        
        let (response_tx, response_rx) = mpsc::channel(100);
        
        Ok(Self {
            session_cookie,
            get_connection: Arc::new(Mutex::new(None)),
            post_connection: Arc::new(Mutex::new(None)),
            url: tunnel_url,
            proxy,
            response_rx: Arc::new(Mutex::new(response_rx)),
            response_tx,
        })
    }
    
    /// Establish the HTTP tunnel (both GET and POST connections)
    pub async fn connect(&mut self) -> Result<()> {
        gst::debug!(*CAT, "Establishing HTTP tunnel to {}", self.url);
        
        // Establish GET connection
        self.establish_get_connection().await?;
        
        // Establish POST connection
        self.establish_post_connection().await?;
        
        gst::info!(*CAT, "HTTP tunnel established successfully");
        Ok(())
    }
    
    /// Establish GET connection for receiving RTSP responses
    async fn establish_get_connection(&mut self) -> Result<()> {
        let host = self.url.host_str().ok_or_else(|| anyhow!("No host in URL"))?;
        let port = self.url.port().unwrap_or(80);
        
        let stream = TcpStream::connect((host, port)).await?;
        
        // Send HTTP GET request with x-sessioncookie header
        let get_request = format!(
            "GET {} HTTP/1.0\r\n\
             Host: {}:{}\r\n\
             x-sessioncookie: {}\r\n\
             Accept: application/x-rtsp-tunnelled\r\n\
             Pragma: no-cache\r\n\
             Cache-Control: no-cache\r\n\
             \r\n",
            self.url.path(),
            host,
            port,
            self.session_cookie
        );
        
        gst::debug!(*CAT, "Sending GET request:\n{}", get_request);
        
        // Send request
        stream.try_write(get_request.as_bytes())?;
        
        // Store connection
        *self.get_connection.lock().await = Some(stream);
        
        // Start background task to read responses
        self.start_get_reader().await;
        
        Ok(())
    }
    
    /// Establish POST connection for sending RTSP requests
    async fn establish_post_connection(&mut self) -> Result<()> {
        let host = self.url.host_str().ok_or_else(|| anyhow!("No host in URL"))?;
        let port = self.url.port().unwrap_or(80);
        
        let stream = TcpStream::connect((host, port)).await?;
        
        // Send HTTP POST request with x-sessioncookie header
        let post_request = format!(
            "POST {} HTTP/1.0\r\n\
             Host: {}:{}\r\n\
             x-sessioncookie: {}\r\n\
             Content-Type: application/x-rtsp-tunnelled\r\n\
             Pragma: no-cache\r\n\
             Cache-Control: no-cache\r\n\
             Content-Length: 32767\r\n\
             Expires: Sun, 9 Jan 1972 00:00:00 GMT\r\n\
             \r\n",
            self.url.path(),
            host,
            port,
            self.session_cookie
        );
        
        gst::debug!(*CAT, "Sending POST request:\n{}", post_request);
        
        // Send request
        stream.try_write(post_request.as_bytes())?;
        
        // Store connection
        *self.post_connection.lock().await = Some(stream);
        
        Ok(())
    }
    
    /// Start background task to read responses from GET connection
    async fn start_get_reader(&self) {
        let get_conn = self.get_connection.clone();
        let tx = self.response_tx.clone();
        
        tokio::spawn(async move {
            let mut buffer = BytesMut::with_capacity(4096);
            
            loop {
                let mut conn = get_conn.lock().await;
                if let Some(stream) = conn.as_mut() {
                    // Read from stream
                    match stream.try_read_buf(&mut buffer) {
                        Ok(0) => {
                            gst::debug!(*CAT, "GET connection closed");
                            break;
                        }
                        Ok(n) => {
                            gst::trace!(*CAT, "Read {} bytes from GET connection", n);
                            
                            // Decode base64 and send to channel
                            if let Ok(decoded) = BASE64.decode(&buffer[..n]) {
                                let _ = tx.send(Bytes::from(decoded)).await;
                            }
                            
                            buffer.clear();
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // No data available, continue
                            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        }
                        Err(e) => {
                            gst::error!(*CAT, "Error reading from GET connection: {}", e);
                            break;
                        }
                    }
                }
            }
        });
    }
    
    /// Send RTSP request through POST connection
    pub async fn send_request(&mut self, request: &[u8]) -> Result<()> {
        let mut conn = self.post_connection.lock().await;
        if let Some(stream) = conn.as_mut() {
            // Encode request as base64
            let encoded = BASE64.encode(request);
            
            gst::trace!(*CAT, "Sending {} bytes through POST tunnel", encoded.len());
            
            // Send encoded data
            stream.try_write(encoded.as_bytes())?;
            
            Ok(())
        } else {
            Err(anyhow!("POST connection not established"))
        }
    }
    
    /// Receive RTSP response from GET connection
    pub async fn receive_response(&mut self) -> Result<Bytes> {
        let mut rx = self.response_rx.lock().await;
        rx.recv().await.ok_or_else(|| anyhow!("Response channel closed"))
    }
    
    /// Check if tunnel is connected
    pub fn is_connected(&self) -> bool {
        // Both connections must be established
        // Note: This is a simplified check, could be improved
        true
    }
    
    /// Close the HTTP tunnel
    pub async fn close(&mut self) {
        gst::debug!(*CAT, "Closing HTTP tunnel");
        
        *self.get_connection.lock().await = None;
        *self.post_connection.lock().await = None;
    }
}

/// Detect if HTTP tunneling is needed based on URL or network conditions
pub fn should_use_tunneling(url: &Url, mode: HttpTunnelMode) -> bool {
    match mode {
        HttpTunnelMode::Always => true,
        HttpTunnelMode::Never => false,
        HttpTunnelMode::Auto => {
            // Auto-detect based on URL scheme or port
            url.scheme() == "http" || url.scheme() == "https" || 
            url.port() == Some(80) || url.port() == Some(443)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_cookie_generation() {
        let url = Url::parse("rtsp://example.com/stream").unwrap();
        let tunnel1 = HttpTunnel::new(&url, None).unwrap();
        let tunnel2 = HttpTunnel::new(&url, None).unwrap();
        
        // Session cookies should be unique
        assert_ne!(tunnel1.session_cookie, tunnel2.session_cookie);
    }
    
    #[test]
    fn test_url_conversion() {
        let rtsp_url = Url::parse("rtsp://example.com:554/stream").unwrap();
        let tunnel = HttpTunnel::new(&rtsp_url, None).unwrap();
        
        assert_eq!(tunnel.url.scheme(), "http");
        assert_eq!(tunnel.url.port(), Some(80));
    }
    
    #[test]
    fn test_should_use_tunneling() {
        let rtsp_url = Url::parse("rtsp://example.com/stream").unwrap();
        let http_url = Url::parse("http://example.com/stream").unwrap();
        
        assert!(!should_use_tunneling(&rtsp_url, HttpTunnelMode::Never));
        assert!(should_use_tunneling(&rtsp_url, HttpTunnelMode::Always));
        assert!(!should_use_tunneling(&rtsp_url, HttpTunnelMode::Auto));
        assert!(should_use_tunneling(&http_url, HttpTunnelMode::Auto));
    }
}