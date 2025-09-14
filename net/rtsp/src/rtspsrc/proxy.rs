// Proxy Support for RTSP
//
// This module implements HTTP CONNECT and SOCKS5 proxy support for RTSP connections

use super::error::{ConfigurationError, NetworkError, Result, RtspError};
use super::tls::{RtspStream, TlsConfig};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use url::Url;

use std::sync::LazyLock;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "rtspsrc2-proxy",
        gst::DebugColorFlags::empty(),
        Some("RTSP Proxy Support"),
    )
});

/// Type of proxy to use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyType {
    Http,
    Socks5,
}

/// Proxy configuration
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// Proxy URL (e.g., "http://proxy.example.com:8080" or "socks5://proxy.example.com:1080")
    pub url: Url,
    /// Username for authentication (optional)
    pub username: Option<String>,
    /// Password for authentication (optional)
    pub password: Option<String>,
}

impl ProxyConfig {
    /// Create proxy config from string URL
    pub fn from_url(url: &str, username: Option<String>, password: Option<String>) -> Result<Self> {
        let parsed_url = Url::parse(url)?;

        // Validate proxy scheme
        match parsed_url.scheme() {
            "http" | "https" | "socks5" | "socks" => {}
            scheme => return Err(RtspError::Configuration(ConfigurationError::InvalidParameter {
                parameter: "proxy_url".to_string(),
                reason: format!("Unsupported proxy scheme: {}", scheme),
            })),
        }

        Ok(Self {
            url: parsed_url,
            username,
            password,
        })
    }

    /// Create proxy config from environment variables
    pub fn from_env() -> Option<Self> {
        // Check for common proxy environment variables
        let proxy_url = std::env::var("http_proxy")
            .or_else(|_| std::env::var("HTTP_PROXY"))
            .or_else(|_| std::env::var("https_proxy"))
            .or_else(|_| std::env::var("HTTPS_PROXY"))
            .or_else(|_| std::env::var("all_proxy"))
            .or_else(|_| std::env::var("ALL_PROXY"))
            .ok()?;

        // Parse the URL
        let url = Url::parse(&proxy_url).ok()?;

        // Extract credentials from URL if present
        let username = url
            .username()
            .is_empty()
            .then(|| url.username().to_string());
        let password = url.password().map(|p| p.to_string());

        // Check for separate credential variables
        let username = username.or_else(|| std::env::var("PROXY_USER").ok());
        let password = password.or_else(|| std::env::var("PROXY_PASS").ok());

        Some(Self {
            url,
            username,
            password,
        })
    }

    /// Get the proxy type from the URL scheme
    pub fn proxy_type(&self) -> ProxyType {
        match self.url.scheme() {
            "socks5" | "socks" => ProxyType::Socks5,
            _ => ProxyType::Http,
        }
    }

    /// Get proxy host and port
    pub fn host_port(&self) -> Result<(String, u16)> {
        let host = self
            .url
            .host_str()
            .ok_or_else(|| RtspError::Configuration(ConfigurationError::InvalidParameter {
                parameter: "proxy_url".to_string(),
                reason: "No host in proxy URL".to_string(),
            }))?
            .to_string();

        let port = self.url.port().unwrap_or_else(|| match self.url.scheme() {
            "http" => 8080,
            "https" => 8080,
            "socks5" | "socks" => 1080,
            _ => 8080,
        });

        Ok((host, port))
    }
}

/// Proxy connection handler
pub struct ProxyConnection;

impl ProxyConnection {
    /// Connect to target through proxy
    pub async fn connect(
        proxy: &ProxyConfig,
        target_host: &str,
        target_port: u16,
        use_tls: bool,
        tls_config: &TlsConfig,
    ) -> Result<RtspStream> {
        let (proxy_host, proxy_port) = proxy.host_port()?;

        gst::debug!(
            *CAT,
            "Connecting to {}:{} through proxy {}:{}",
            target_host,
            target_port,
            proxy_host,
            proxy_port
        );

        // Connect to proxy server
        let mut tcp_stream = TcpStream::connect((proxy_host.as_str(), proxy_port)).await?;

        // Perform proxy handshake based on type
        match proxy.proxy_type() {
            ProxyType::Http => {
                Self::http_connect_handshake(
                    &mut tcp_stream,
                    target_host,
                    target_port,
                    &proxy.username,
                    &proxy.password,
                )
                .await?;
            }
            ProxyType::Socks5 => {
                Self::socks5_handshake(
                    &mut tcp_stream,
                    target_host,
                    target_port,
                    &proxy.username,
                    &proxy.password,
                )
                .await?;
            }
        }

        gst::info!(
            *CAT,
            "Successfully connected to {}:{} through proxy",
            target_host,
            target_port
        );

        // Upgrade to TLS if needed
        if use_tls {
            Self::upgrade_to_tls(tcp_stream, target_host, tls_config).await
        } else {
            Ok(RtspStream::Plain(tcp_stream))
        }
    }

    /// Perform HTTP CONNECT handshake
    async fn http_connect_handshake(
        stream: &mut TcpStream,
        target_host: &str,
        target_port: u16,
        username: &Option<String>,
        password: &Option<String>,
    ) -> Result<()> {
        // Build CONNECT request
        let mut request = format!(
            "CONNECT {}:{} HTTP/1.1\r\n\
             Host: {}:{}\r\n",
            target_host, target_port, target_host, target_port
        );

        // Add authentication header if credentials provided
        if let (Some(user), Some(pass)) = (username, password) {
            let credentials = format!("{}:{}", user, pass);
            let encoded = BASE64.encode(credentials.as_bytes());
            request.push_str(&format!("Proxy-Authorization: Basic {}\r\n", encoded));
        }

        request.push_str("\r\n");

        gst::trace!(*CAT, "Sending HTTP CONNECT request:\n{}", request);

        // Send CONNECT request
        stream.write_all(request.as_bytes()).await?;

        // Read response
        let mut response = vec![0u8; 1024];
        let n = stream.read(&mut response).await?;
        response.truncate(n);

        let response_str = String::from_utf8_lossy(&response);
        gst::trace!(*CAT, "Received HTTP CONNECT response:\n{}", response_str);

        // Check for successful response (200 OK)
        if !response_str.starts_with("HTTP/1.1 200") && !response_str.starts_with("HTTP/1.0 200") {
            return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: format!(
                    "HTTP CONNECT failed: {}",
                    response_str.lines().next().unwrap_or("Unknown error")
                ),
            }));
        }

        Ok(())
    }

    /// Perform SOCKS5 handshake
    async fn socks5_handshake(
        stream: &mut TcpStream,
        target_host: &str,
        target_port: u16,
        username: &Option<String>,
        password: &Option<String>,
    ) -> Result<()> {
        // SOCKS5 authentication methods
        const NO_AUTH: u8 = 0x00;
        const USER_PASS_AUTH: u8 = 0x02;

        // Send greeting with supported authentication methods
        let auth_methods = if username.is_some() && password.is_some() {
            vec![0x05, 0x02, NO_AUTH, USER_PASS_AUTH] // Version 5, 2 methods
        } else {
            vec![0x05, 0x01, NO_AUTH] // Version 5, 1 method (no auth)
        };

        stream.write_all(&auth_methods).await?;

        // Read server response
        let mut response = [0u8; 2];
        stream.read_exact(&mut response).await?;

        if response[0] != 0x05 {
            return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: format!("Invalid SOCKS5 version: {}", response[0]),
            }));
        }

        // Handle authentication if required
        match response[1] {
            NO_AUTH => {
                gst::trace!(*CAT, "SOCKS5: No authentication required");
            }
            USER_PASS_AUTH => {
                if let (Some(user), Some(pass)) = (username, password) {
                    gst::trace!(*CAT, "SOCKS5: Performing username/password authentication");

                    // Send username/password authentication
                    let mut auth_data = Vec::new();
                    auth_data.push(0x01); // Version
                    auth_data.push(user.len() as u8);
                    auth_data.extend_from_slice(user.as_bytes());
                    auth_data.push(pass.len() as u8);
                    auth_data.extend_from_slice(pass.as_bytes());

                    stream.write_all(&auth_data).await?;

                    // Read authentication response
                    let mut auth_response = [0u8; 2];
                    stream.read_exact(&mut auth_response).await?;

                    if auth_response[0] != 0x01 || auth_response[1] != 0x00 {
                        return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                            details: "SOCKS5 authentication failed".to_string(),
                        }));
                    }
                } else {
                    return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                        details: "SOCKS5 server requires authentication but no credentials provided".to_string(),
                    }));
                }
            }
            0xFF => {
                return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                    details: "SOCKS5: No acceptable authentication methods".to_string(),
                }));
            }
            method => {
                return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                    details: format!("SOCKS5: Unsupported authentication method: {}", method),
                }));
            }
        }

        // Send connection request
        let mut request = Vec::new();
        request.push(0x05); // Version
        request.push(0x01); // Connect command
        request.push(0x00); // Reserved

        // Determine address type and encode target address
        if let Ok(ip) = target_host.parse::<std::net::IpAddr>() {
            match ip {
                std::net::IpAddr::V4(ipv4) => {
                    request.push(0x01); // IPv4
                    request.extend_from_slice(&ipv4.octets());
                }
                std::net::IpAddr::V6(ipv6) => {
                    request.push(0x04); // IPv6
                    request.extend_from_slice(&ipv6.octets());
                }
            }
        } else {
            // Domain name
            request.push(0x03); // Domain name
            request.push(target_host.len() as u8);
            request.extend_from_slice(target_host.as_bytes());
        }

        // Add port (network byte order)
        request.extend_from_slice(&target_port.to_be_bytes());

        stream.write_all(&request).await?;

        // Read connection response
        let mut conn_response = [0u8; 10]; // Minimum response size
        stream.read_exact(&mut conn_response[..4]).await?;

        if conn_response[0] != 0x05 {
            return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: "Invalid SOCKS5 version in response".to_string(),
            }));
        }

        // Check reply code
        match conn_response[1] {
            0x00 => {} // Success
            0x01 => return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: "SOCKS5: General SOCKS server failure".to_string(),
            })),
            0x02 => return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: "SOCKS5: Connection not allowed by ruleset".to_string(),
            })),
            0x03 => return Err(RtspError::Network(NetworkError::NetworkUnreachable {
                details: "SOCKS5: Network unreachable".to_string(),
            })),
            0x04 => return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: "SOCKS5: Host unreachable".to_string(),
            })),
            0x05 => return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: "SOCKS5: Connection refused".to_string(),
            })),
            0x06 => return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: "SOCKS5: TTL expired".to_string(),
            })),
            0x07 => return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: "SOCKS5: Command not supported".to_string(),
            })),
            0x08 => return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: "SOCKS5: Address type not supported".to_string(),
            })),
            code => return Err(RtspError::Network(NetworkError::ProxyConnectionFailed {
                details: format!("SOCKS5: Unknown error code: {}", code),
            })),
        }

        // Skip the rest of the response (bound address)
        match conn_response[3] {
            0x01 => {
                // IPv4
                let mut addr = [0u8; 6]; // 4 bytes IP + 2 bytes port
                stream.read_exact(&mut addr).await?;
            }
            0x03 => {
                // Domain name
                let mut len = [0u8; 1];
                stream.read_exact(&mut len).await?;
                let mut addr = vec![0u8; len[0] as usize + 2]; // domain + port
                stream.read_exact(&mut addr).await?;
            }
            0x04 => {
                // IPv6
                let mut addr = [0u8; 18]; // 16 bytes IP + 2 bytes port
                stream.read_exact(&mut addr).await?;
            }
            _ => {}
        }

        gst::trace!(*CAT, "SOCKS5 connection established");

        Ok(())
    }

    /// Connect directly without proxy
    pub async fn connect_direct(
        host: &str, 
        port: u16,
        use_tls: bool,
        tls_config: &TlsConfig,
    ) -> Result<RtspStream> {
        gst::debug!(*CAT, "Connecting directly to {}:{} (TLS: {})", host, port, use_tls);
        let tcp_stream = TcpStream::connect((host, port)).await?;
        
        if use_tls {
            Self::upgrade_to_tls(tcp_stream, host, tls_config).await
        } else {
            Ok(RtspStream::Plain(tcp_stream))
        }
    }

    /// Upgrade a TCP connection to TLS
    async fn upgrade_to_tls(
        tcp_stream: TcpStream,
        host: &str,
        tls_config: &TlsConfig,
    ) -> Result<RtspStream> {
        use tokio_native_tls::{native_tls, TlsConnector};
        
        gst::debug!(*CAT, "Upgrading connection to TLS for host: {}", host);
        
        let mut builder = native_tls::TlsConnector::builder();
        
        if tls_config.accept_invalid_certs {
            builder.danger_accept_invalid_certs(true);
        }
        
        if tls_config.accept_invalid_hostnames {
            builder.danger_accept_invalid_hostnames(true);
        }
        
        if let Some(min_version) = tls_config.min_version {
            builder.min_protocol_version(Some(min_version));
        }
        
        if let Some(max_version) = tls_config.max_version {
            builder.max_protocol_version(Some(max_version));
        }
        
        let tls_connector = builder.build()?;
        let tls_connector = TlsConnector::from(tls_connector);
        
        let tls_stream = tls_connector.connect(host, tcp_stream).await?;
        Ok(RtspStream::Tls(tls_stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_config_from_url() {
        // HTTP proxy
        let config = ProxyConfig::from_url(
            "http://proxy.example.com:8080",
            Some("user".to_string()),
            Some("pass".to_string()),
        )
        .unwrap();

        assert_eq!(config.proxy_type(), ProxyType::Http);
        assert_eq!(
            config.host_port().unwrap(),
            ("proxy.example.com".to_string(), 8080)
        );
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));

        // SOCKS5 proxy
        let config = ProxyConfig::from_url("socks5://socks.example.com:1080", None, None).unwrap();

        assert_eq!(config.proxy_type(), ProxyType::Socks5);
        assert_eq!(
            config.host_port().unwrap(),
            ("socks.example.com".to_string(), 1080)
        );
        assert_eq!(config.username, None);
        assert_eq!(config.password, None);
    }

    #[test]
    fn test_proxy_default_ports() {
        // HTTP proxy without port
        let config = ProxyConfig::from_url("http://proxy.example.com", None, None).unwrap();
        assert_eq!(config.host_port().unwrap().1, 8080);

        // SOCKS5 proxy without port
        let config = ProxyConfig::from_url("socks5://socks.example.com", None, None).unwrap();
        assert_eq!(config.host_port().unwrap().1, 1080);
    }

    #[test]
    fn test_invalid_proxy_scheme() {
        let result = ProxyConfig::from_url("ftp://proxy.example.com", None, None);
        assert!(result.is_err());
    }
}
