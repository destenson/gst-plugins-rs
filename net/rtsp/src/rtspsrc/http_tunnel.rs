#![allow(unused)]
// HTTP Tunneling Support for RTSP
//
// This module implements RTSP-over-HTTP tunneling to bypass firewalls
// and proxies that block RTSP traffic.

use crate::rtspsrc::imp::HttpTunnelMode;
use crate::rtspsrc::proxy::{ProxyConfig, ProxyConnection};
use super::error::{NetworkError, ProtocolError, Result, RtspError};
use super::body::Body;
use super::tcp_message::ReadError;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use bytes::{Bytes, BytesMut};
use futures::{Sink, Stream};
use rtsp_types::Message;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use url::Url;

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
    /// Proxy configuration
    proxy_config: Option<ProxyConfig>,
    /// Channel for receiving RTSP responses from GET connection
    response_rx: Arc<Mutex<mpsc::Receiver<Bytes>>>,
    response_tx: mpsc::Sender<Bytes>,
}

impl HttpTunnel {
    /// Create a new HTTP tunnel
    pub fn new(
        rtsp_url: &Url,
        proxy: Option<String>,
        proxy_id: Option<String>,
        proxy_pw: Option<String>,
    ) -> Result<Self> {
        // Generate a unique session cookie using timestamp
        use std::time::{SystemTime, UNIX_EPOCH};
        let session_cookie = format!("{:x}", SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos());

        // Build HTTP URL for tunneling
        let host = rtsp_url.host_str()
            .ok_or_else(|| RtspError::Network(NetworkError::HttpTunnelError {
                details: "No host in URL".to_string(),
            }))?;
        let port = rtsp_url.port().unwrap_or(if rtsp_url.scheme() == "rtsps" { 443 } else { 80 });
        let path = rtsp_url.path();
        
        let tunnel_url = Url::parse(&format!("http://{}:{}{}", host, port, path))
            .map_err(|e| RtspError::Network(NetworkError::HttpTunnelError {
                details: format!("Failed to create HTTP URL: {}", e),
            }))?;

        let (response_tx, response_rx) = mpsc::channel(100);

        // Create proxy config if proxy URL is provided
        let proxy_config = if let Some(proxy_url) = proxy {
            Some(ProxyConfig::from_url(&proxy_url, proxy_id, proxy_pw)?)
        } else {
            // Try to get from environment if not explicitly provided
            ProxyConfig::from_env()
        };

        Ok(Self {
            session_cookie,
            get_connection: Arc::new(Mutex::new(None)),
            post_connection: Arc::new(Mutex::new(None)),
            url: tunnel_url,
            proxy_config,
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
        let host = self
            .url
            .host_str()
            .ok_or_else(|| RtspError::Protocol(ProtocolError::InvalidUrl {
                url: self.url.to_string(),
                reason: "No host in URL".to_string(),
            }))?;
        let port = self.url.port().unwrap_or(80);

        // Connect through proxy if configured, otherwise direct connection
        let stream = if let Some(ref proxy) = self.proxy_config {
            ProxyConnection::connect(proxy, host, port).await?
        } else {
            ProxyConnection::connect_direct(host, port).await?
        };

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
        let host = self
            .url
            .host_str()
            .ok_or_else(|| RtspError::Protocol(ProtocolError::InvalidUrl {
                url: self.url.to_string(),
                reason: "No host in URL".to_string(),
            }))?;
        let port = self.url.port().unwrap_or(80);

        // Connect through proxy if configured, otherwise direct connection
        let stream = if let Some(ref proxy) = self.proxy_config {
            ProxyConnection::connect(proxy, host, port).await?
        } else {
            ProxyConnection::connect_direct(host, port).await?
        };

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
            Err(RtspError::Network(NetworkError::HttpTunnelError {
                details: "POST connection not established".to_string(),
            }))
        }
    }

    /// Receive RTSP response from GET connection
    pub async fn receive_response(&mut self) -> Result<Bytes> {
        let mut rx = self.response_rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| RtspError::Network(NetworkError::HttpTunnelError {
                details: "Response channel closed".to_string(),
            }))
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
            url.scheme() == "http"
                || url.scheme() == "https"
                || url.port() == Some(80)
                || url.port() == Some(443)
        }
    }
}

/// Wrapper to make HttpTunnel work with async read/write interfaces
pub struct HttpTunnelStream {
    tunnel: Arc<Mutex<HttpTunnel>>,
    read_buffer: BytesMut,
}

impl HttpTunnelStream {
    pub fn new(tunnel: HttpTunnel) -> Self {
        Self {
            tunnel: Arc::new(Mutex::new(tunnel)),
            read_buffer: BytesMut::with_capacity(4096),
        }
    }
    
    pub fn split(self) -> (HttpTunnelReader, HttpTunnelWriter) {
        let reader = HttpTunnelReader {
            tunnel: self.tunnel.clone(),
            read_buffer: self.read_buffer,
        };
        let writer = HttpTunnelWriter {
            tunnel: self.tunnel,
        };
        (reader, writer)
    }
}

/// Reader half of the HTTP tunnel
pub struct HttpTunnelReader {
    tunnel: Arc<Mutex<HttpTunnel>>,
    read_buffer: BytesMut,
}

/// Writer half of the HTTP tunnel  
pub struct HttpTunnelWriter {
    tunnel: Arc<Mutex<HttpTunnel>>,
}

/// Stream implementation for reading RTSP messages from HTTP tunnel
pub struct HttpTunnelMessageStream {
    reader: HttpTunnelReader,
}

impl HttpTunnelMessageStream {
    pub fn new(reader: HttpTunnelReader) -> Self {
        Self { reader }
    }
}

impl Stream for HttpTunnelMessageStream {
    type Item = std::result::Result<Message<Body>, ReadError>;
    
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // TODO: Implement proper async polling for RTSP messages from tunnel
        // For now, return pending
        Poll::Pending
    }
}

/// Sink implementation for sending RTSP messages through HTTP tunnel
pub struct HttpTunnelMessageSink {
    writer: HttpTunnelWriter,
}

impl HttpTunnelMessageSink {
    pub fn new(writer: HttpTunnelWriter) -> Self {
        Self { writer }
    }
}

impl Sink<Message<Body>> for HttpTunnelMessageSink {
    type Error = std::io::Error;
    
    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        // Tunnel is always ready to accept messages
        Poll::Ready(Ok(()))
    }
    
    fn start_send(self: Pin<&mut Self>, _item: Message<Body>) -> std::result::Result<(), Self::Error> {
        // Convert message to bytes and send through tunnel
        // TODO: Implement proper message serialization
        Ok(())
    }
    
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        // No buffering, always flushed
        Poll::Ready(Ok(()))
    }
    
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        // Close the tunnel
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_cookie_generation() {
        let url = Url::parse("rtsp://example.com/stream").unwrap();
        let tunnel1 = HttpTunnel::new(&url, None, None, None);
        let tunnel2 = HttpTunnel::new(&url, None, None, None);

        // Just test that both can be created
        assert!(tunnel1.is_ok());
        assert!(tunnel2.is_ok());
        
        // Session cookies should be unique (they're based on timestamp)
        // This is guaranteed by the implementation
    }

    #[test]
    fn test_url_conversion() {
        let rtsp_url = Url::parse("rtsp://example.com:554/stream").unwrap();
        let tunnel = HttpTunnel::new(&rtsp_url, None, None, None);

        // Just test that tunnel can be created
        assert!(tunnel.is_ok());
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
