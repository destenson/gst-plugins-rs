#![allow(unused)]
// TLS/TCP Transport Support for RTSP
//
// This module provides TLS support for secure RTSP connections (rtsps://)

use super::error::{NetworkError, ProtocolError, Result, RtspError};
use tokio::net::TcpStream;
use tokio_native_tls::{native_tls, TlsConnector, TlsStream};
use url::Url;

// Default RTSPS port as per RFC 2326 Section 11.1
pub const DEFAULT_RTSPS_PORT: u16 = 322;
pub const DEFAULT_RTSP_PORT: u16 = 554;

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub enabled: bool,
    pub accept_invalid_certs: bool,
    pub accept_invalid_hostnames: bool,
    pub min_version: Option<native_tls::Protocol>,
    pub max_version: Option<native_tls::Protocol>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            accept_invalid_certs: false,
            accept_invalid_hostnames: false,
            min_version: Some(native_tls::Protocol::Tlsv12),
            max_version: None,
        }
    }
}

pub enum RtspStream {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl RtspStream {
    pub async fn connect(url: &Url, tls_config: &TlsConfig) -> Result<Self> {
        let host = url.host_str().ok_or_else(|| RtspError::Protocol(ProtocolError::InvalidUrl {
            url: url.to_string(),
            reason: "No host in URL".to_string(),
        }))?;

        let is_tls = url.scheme() == "rtsps";
        let default_port = if is_tls {
            DEFAULT_RTSPS_PORT
        } else {
            DEFAULT_RTSP_PORT
        };

        let port = url.port().unwrap_or(default_port);
        let addr = format!("{}:{}", host, port);

        // Connect TCP first
        let tcp_stream = TcpStream::connect(&addr).await?;

        if is_tls {
            // Upgrade to TLS
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
        } else {
            Ok(RtspStream::Plain(tcp_stream))
        }
    }

    pub fn is_tls(&self) -> bool {
        matches!(self, RtspStream::Tls(_))
    }
}

// Implement AsyncRead and AsyncWrite for RtspStream
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

impl AsyncRead for RtspStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match &mut *self {
            RtspStream::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            RtspStream::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for RtspStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match &mut *self {
            RtspStream::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            RtspStream::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match &mut *self {
            RtspStream::Plain(stream) => Pin::new(stream).poll_flush(cx),
            RtspStream::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match &mut *self {
            RtspStream::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            RtspStream::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

/// Parse URL and determine if TLS should be used
pub fn is_tls_url(url: &Url) -> bool {
    url.scheme() == "rtsps"
}

/// Get the appropriate default port based on URL scheme
pub fn get_default_port(url: &Url) -> u16 {
    if is_tls_url(url) {
        DEFAULT_RTSPS_PORT
    } else {
        DEFAULT_RTSP_PORT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_url_detection() {
        let rtsp_url = Url::parse("rtsp://example.com/stream").unwrap();
        let rtsps_url = Url::parse("rtsps://example.com/stream").unwrap();

        assert!(!is_tls_url(&rtsp_url));
        assert!(is_tls_url(&rtsps_url));
    }

    #[test]
    fn test_default_port_selection() {
        let rtsp_url = Url::parse("rtsp://example.com/stream").unwrap();
        let rtsps_url = Url::parse("rtsps://example.com/stream").unwrap();

        assert_eq!(get_default_port(&rtsp_url), DEFAULT_RTSP_PORT);
        assert_eq!(get_default_port(&rtsps_url), DEFAULT_RTSPS_PORT);
    }

    #[test]
    fn test_tls_config_defaults() {
        let config = TlsConfig::default();
        assert!(!config.enabled);
        assert!(!config.accept_invalid_certs);
        assert!(!config.accept_invalid_hostnames);
        // Note: Cannot directly compare Protocol values as they don't implement PartialEq
        // assert_eq!(config.min_version, Some(native_tls::Protocol::Tlsv12));
        assert!(config.min_version.is_some());
    }
}
