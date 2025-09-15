#![allow(unused)]
// GStreamer RTSP Error Handling Module
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use gst::{self, glib};
use std::io;
use std::time::Duration;
use thiserror::Error;

use super::retry::RetryStrategy;
use crate::rtspsrc;

/// Main RTSP error type with comprehensive error categories
#[derive(Debug, Error)]
pub enum RtspError {
    /// Network-related errors
    #[error(transparent)]
    Network(#[from] NetworkError),

    /// RTSP protocol errors
    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    /// Media/codec related errors
    #[error(transparent)]
    Media(#[from] MediaError),

    /// Configuration errors
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),

    /// Generic internal errors (for compatibility)
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

/// Network-related error types
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Connection refused to {host}:{port} - server may be down or unreachable")]
    ConnectionRefused { host: String, port: u16 },

    #[error("Connection timeout after {timeout:?} to {host}:{port}")]
    ConnectionTimeout {
        host: String,
        port: u16,
        timeout: Duration,
    },

    #[error("DNS resolution failed for {host}: {details}")]
    DnsResolutionFailed { host: String, details: String },

    #[error("Socket error: {message}")]
    SocketError {
        message: String,
        #[source]
        io_error: io::Error,
    },

    #[error("TLS handshake failed: {details}")]
    TlsHandshakeFailed { details: String },

    #[error("Network unreachable: {details}")]
    NetworkUnreachable { details: String },

    #[error("Connection reset by peer")]
    ConnectionReset,

    #[error("NAT traversal failed: {reason}")]
    NatTraversalFailed { reason: String },

    #[error("Proxy connection failed: {details}")]
    ProxyConnectionFailed { details: String },

    #[error("HTTP tunneling error: {details}")]
    HttpTunnelError { details: String },
}

/// RTSP protocol-specific errors
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Invalid RTSP response: {details}")]
    InvalidResponse { details: String },

    #[error("Unsupported RTSP feature: {feature}")]
    UnsupportedFeature { feature: String },

    #[error("Authentication failed: {method} - {details}")]
    AuthenticationFailed { method: String, details: String },

    #[error("Session error: {details}")]
    SessionError { details: String },

    #[error("Invalid session ID: {session_id}")]
    InvalidSessionId { session_id: String },

    #[error("RTSP method not allowed: {method}")]
    MethodNotAllowed { method: String },

    #[error("RTSP status {code}: {message}")]
    StatusError { code: u16, message: String },

    #[error("Missing required header: {header}")]
    MissingHeader { header: String },

    #[error("Invalid URL: {url} - {reason}")]
    InvalidUrl { url: String, reason: String },

    #[error("Transport negotiation failed: {reason}")]
    TransportNegotiationFailed { reason: String },
}

/// Media and codec-related errors
#[derive(Debug, Error)]
pub enum MediaError {
    #[error("Unsupported codec: {codec}")]
    UnsupportedCodec { codec: String },

    #[error("SDP parsing failed: {details}")]
    SdpParsingFailed { details: String },

    #[error("Stream synchronization lost")]
    StreamSyncLost,

    #[error("Buffer overflow - unable to process media data fast enough")]
    BufferOverflow,

    #[error("Invalid media format: {details}")]
    InvalidMediaFormat { details: String },

    #[error("No compatible media streams found")]
    NoCompatibleStreams,

    #[error("RTCP error: {details}")]
    RtcpError { details: String },

    #[error("RTP packet error: {details}")]
    RtpPacketError { details: String },
}

/// Configuration and setup errors
#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("Invalid configuration: {parameter} - {reason}")]
    InvalidParameter { parameter: String, reason: String },

    #[error("Missing required configuration: {parameter}")]
    MissingParameter { parameter: String },

    #[error("Conflicting configuration: {details}")]
    ConflictingConfiguration { details: String },

    #[error("Resource allocation failed: {resource}")]
    ResourceAllocationFailed { resource: String },
}

/// Error context with additional debugging information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// URL or identifier of the resource
    pub resource: Option<String>,
    /// Current operation being performed
    pub operation: Option<String>,
    /// Number of retry attempts made
    pub retry_count: u32,
    /// Additional key-value pairs for debugging
    pub details: Vec<(String, String)>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            resource: None,
            operation: None,
            retry_count: 0,
            details: Vec::new(),
        }
    }

    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    pub fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    pub fn add_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.push((key.into(), value.into()));
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Error classification for retry logic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// Errors that should be retried immediately
    Transient,
    /// Errors that should be retried with backoff
    RetryableWithBackoff,
    /// Errors that should not be retried
    Permanent,
    /// Errors that require special handling
    RequiresIntervention,
}

/// Trait for classifying errors for retry strategies
pub trait ErrorClassification {
    /// Classify the error for retry logic
    fn classify(&self) -> ErrorClass;

    /// Get suggested retry strategy for this error
    fn suggested_retry_strategy(&self) -> Option<RetryStrategy>;

    /// Check if the error is retryable
    fn is_retryable(&self) -> bool {
        matches!(
            self.classify(),
            ErrorClass::Transient | ErrorClass::RetryableWithBackoff
        )
    }
}

impl ErrorClassification for NetworkError {
    fn classify(&self) -> ErrorClass {
        match self {
            NetworkError::ConnectionRefused { .. } => ErrorClass::RetryableWithBackoff,
            NetworkError::ConnectionTimeout { .. } => ErrorClass::Transient,
            NetworkError::DnsResolutionFailed { .. } => ErrorClass::RetryableWithBackoff,
            NetworkError::SocketError { .. } => ErrorClass::Transient,
            NetworkError::TlsHandshakeFailed { .. } => ErrorClass::Permanent,
            NetworkError::NetworkUnreachable { .. } => ErrorClass::RetryableWithBackoff,
            NetworkError::ConnectionReset => ErrorClass::Transient,
            NetworkError::NatTraversalFailed { .. } => ErrorClass::RequiresIntervention,
            NetworkError::ProxyConnectionFailed { .. } => ErrorClass::RetryableWithBackoff,
            NetworkError::HttpTunnelError { .. } => ErrorClass::RetryableWithBackoff,
        }
    }

    fn suggested_retry_strategy(&self) -> Option<RetryStrategy> {
        match self {
            NetworkError::ConnectionTimeout { .. } => Some(RetryStrategy::FirstWins),
            NetworkError::ConnectionRefused { .. } => Some(RetryStrategy::ExponentialJitter),
            NetworkError::NetworkUnreachable { .. } => Some(RetryStrategy::Exponential),
            NetworkError::ConnectionReset => Some(RetryStrategy::Immediate),
            _ => None,
        }
    }
}

impl ErrorClassification for ProtocolError {
    fn classify(&self) -> ErrorClass {
        match self {
            ProtocolError::InvalidResponse { .. } => ErrorClass::Transient,
            ProtocolError::UnsupportedFeature { .. } => ErrorClass::Permanent,
            ProtocolError::AuthenticationFailed { .. } => ErrorClass::RequiresIntervention,
            ProtocolError::SessionError { .. } => ErrorClass::RetryableWithBackoff,
            ProtocolError::InvalidSessionId { .. } => ErrorClass::RetryableWithBackoff,
            ProtocolError::MethodNotAllowed { .. } => ErrorClass::Permanent,
            ProtocolError::StatusError { code, .. } => {
                if *code >= 500 {
                    ErrorClass::RetryableWithBackoff
                } else if *code >= 400 {
                    ErrorClass::Permanent
                } else {
                    ErrorClass::Transient
                }
            }
            ProtocolError::MissingHeader { .. } => ErrorClass::Permanent,
            ProtocolError::InvalidUrl { .. } => ErrorClass::Permanent,
            ProtocolError::TransportNegotiationFailed { .. } => ErrorClass::RequiresIntervention,
        }
    }

    fn suggested_retry_strategy(&self) -> Option<RetryStrategy> {
        match self {
            ProtocolError::SessionError { .. } => Some(RetryStrategy::Linear),
            ProtocolError::StatusError { code, .. } if *code >= 500 => {
                Some(RetryStrategy::ExponentialJitter)
            }
            _ => None,
        }
    }
}

impl ErrorClassification for MediaError {
    fn classify(&self) -> ErrorClass {
        match self {
            MediaError::UnsupportedCodec { .. } => ErrorClass::Permanent,
            MediaError::SdpParsingFailed { .. } => ErrorClass::Permanent,
            MediaError::StreamSyncLost => ErrorClass::Transient,
            MediaError::BufferOverflow => ErrorClass::Transient,
            MediaError::InvalidMediaFormat { .. } => ErrorClass::Permanent,
            MediaError::NoCompatibleStreams => ErrorClass::Permanent,
            MediaError::RtcpError { .. } => ErrorClass::Transient,
            MediaError::RtpPacketError { .. } => ErrorClass::Transient,
        }
    }

    fn suggested_retry_strategy(&self) -> Option<RetryStrategy> {
        match self {
            MediaError::StreamSyncLost | MediaError::BufferOverflow => {
                Some(RetryStrategy::Immediate)
            }
            _ => None,
        }
    }
}

impl ErrorClassification for ConfigurationError {
    fn classify(&self) -> ErrorClass {
        ErrorClass::Permanent
    }

    fn suggested_retry_strategy(&self) -> Option<RetryStrategy> {
        None
    }
}

impl ErrorClassification for RtspError {
    fn classify(&self) -> ErrorClass {
        match self {
            RtspError::Network(e) => e.classify(),
            RtspError::Protocol(e) => e.classify(),
            RtspError::Media(e) => e.classify(),
            RtspError::Configuration(e) => e.classify(),
            RtspError::Internal { .. } => ErrorClass::Permanent,
        }
    }

    fn suggested_retry_strategy(&self) -> Option<RetryStrategy> {
        match self {
            RtspError::Network(e) => e.suggested_retry_strategy(),
            RtspError::Protocol(e) => e.suggested_retry_strategy(),
            RtspError::Media(e) => e.suggested_retry_strategy(),
            RtspError::Configuration(e) => e.suggested_retry_strategy(),
            RtspError::Internal { .. } => None,
        }
    }
}

/// Helper functions for error conversion and context
impl RtspError {
    /// Create an internal error with a message
    pub fn internal(message: impl Into<String>) -> Self {
        RtspError::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Create an internal error with a source
    pub fn internal_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        RtspError::Internal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Convert to a GStreamer error message with appropriate domain and code
    pub fn to_gst_error(&self) -> gst::ErrorMessage {
        match self {
            RtspError::Network(e) => match e {
                NetworkError::ConnectionRefused { host, port } => {
                    gst::error_msg!(
                        gst::ResourceError::OpenRead,
                        [
                            "Connection refused to {}:{} - server may be down or unreachable",
                            host,
                            port
                        ]
                    )
                }
                NetworkError::ConnectionTimeout {
                    host,
                    port,
                    timeout,
                } => {
                    gst::error_msg!(
                        gst::ResourceError::OpenRead,
                        [
                            "Connection timeout after {:?} to {}:{}",
                            timeout,
                            host,
                            port
                        ]
                    )
                }
                NetworkError::DnsResolutionFailed { host, details } => {
                    gst::error_msg!(
                        gst::ResourceError::NotFound,
                        ["DNS resolution failed for {}: {}", host, details]
                    )
                }
                NetworkError::SocketError { message, .. } => {
                    gst::error_msg!(gst::ResourceError::OpenRead, ["Socket error: {}", message])
                }
                NetworkError::TlsHandshakeFailed { details } => {
                    gst::error_msg!(
                        gst::ResourceError::OpenRead,
                        ["TLS handshake failed: {}", details]
                    )
                }
                NetworkError::NetworkUnreachable { details } => {
                    gst::error_msg!(
                        gst::ResourceError::OpenRead,
                        ["Network unreachable: {}", details]
                    )
                }
                NetworkError::ConnectionReset => {
                    gst::error_msg!(gst::ResourceError::Read, ["Connection reset by peer"])
                }
                NetworkError::NatTraversalFailed { reason } => {
                    gst::error_msg!(
                        gst::ResourceError::OpenRead,
                        ["NAT traversal failed: {}", reason]
                    )
                }
                NetworkError::ProxyConnectionFailed { details } => {
                    gst::error_msg!(
                        gst::ResourceError::OpenRead,
                        ["Proxy connection failed: {}", details]
                    )
                }
                NetworkError::HttpTunnelError { details } => {
                    gst::error_msg!(
                        gst::ResourceError::OpenRead,
                        ["HTTP tunneling error: {}", details]
                    )
                }
            },
            RtspError::Protocol(e) => match e {
                ProtocolError::InvalidResponse { details } => {
                    gst::error_msg!(
                        gst::ResourceError::Failed,
                        ["Invalid RTSP response: {}", details]
                    )
                }
                ProtocolError::UnsupportedFeature { feature } => {
                    gst::error_msg!(
                        gst::ResourceError::NotFound,
                        ["Unsupported RTSP feature: {}", feature]
                    )
                }
                ProtocolError::AuthenticationFailed { method, details } => {
                    gst::error_msg!(
                        gst::ResourceError::NotAuthorized,
                        ["Authentication failed ({}): {}", method, details]
                    )
                }
                ProtocolError::SessionError { details } => {
                    gst::error_msg!(gst::ResourceError::Failed, ["Session error: {}", details])
                }
                ProtocolError::InvalidSessionId { session_id } => {
                    gst::error_msg!(
                        gst::ResourceError::Failed,
                        ["Invalid session ID: {}", session_id]
                    )
                }
                ProtocolError::MethodNotAllowed { method } => {
                    gst::error_msg!(
                        gst::ResourceError::Failed,
                        ["RTSP method not allowed: {}", method]
                    )
                }
                ProtocolError::StatusError { code, message } => {
                    let error_type = if *code >= 500 {
                        gst::ResourceError::Failed
                    } else if *code == 401 || *code == 403 {
                        gst::ResourceError::NotAuthorized
                    } else if *code == 404 {
                        gst::ResourceError::NotFound
                    } else {
                        gst::ResourceError::Failed
                    };
                    gst::error_msg!(error_type, ["RTSP error {}: {}", code, message])
                }
                ProtocolError::MissingHeader { header } => {
                    gst::error_msg!(
                        gst::ResourceError::Failed,
                        ["Missing required header: {}", header]
                    )
                }
                ProtocolError::InvalidUrl { url, reason } => {
                    gst::error_msg!(
                        gst::ResourceError::Settings,
                        ["Invalid URL '{}': {}", url, reason]
                    )
                }
                ProtocolError::TransportNegotiationFailed { reason } => {
                    gst::error_msg!(
                        gst::ResourceError::Failed,
                        ["Transport negotiation failed: {}", reason]
                    )
                }
            },
            RtspError::Media(e) => match e {
                MediaError::UnsupportedCodec { codec } => {
                    gst::error_msg!(
                        gst::StreamError::TypeNotFound,
                        ["Unsupported codec: {}", codec]
                    )
                }
                MediaError::SdpParsingFailed { details } => {
                    gst::error_msg!(
                        gst::StreamError::Decode,
                        ["SDP parsing failed: {}", details]
                    )
                }
                MediaError::StreamSyncLost => {
                    gst::error_msg!(gst::StreamError::Failed, ["Stream synchronization lost"])
                }
                MediaError::BufferOverflow => {
                    gst::error_msg!(
                        gst::StreamError::Failed,
                        ["Buffer overflow - unable to process media data fast enough"]
                    )
                }
                MediaError::InvalidMediaFormat { details } => {
                    gst::error_msg!(
                        gst::StreamError::Format,
                        ["Invalid media format: {}", details]
                    )
                }
                MediaError::NoCompatibleStreams => {
                    gst::error_msg!(
                        gst::StreamError::TypeNotFound,
                        ["No compatible media streams found"]
                    )
                }
                MediaError::RtcpError { details } => {
                    gst::error_msg!(gst::StreamError::Failed, ["RTCP error: {}", details])
                }
                MediaError::RtpPacketError { details } => {
                    gst::error_msg!(gst::StreamError::Failed, ["RTP packet error: {}", details])
                }
            },
            RtspError::Configuration(e) => match e {
                ConfigurationError::InvalidParameter { parameter, reason } => {
                    gst::error_msg!(
                        gst::ResourceError::Settings,
                        [
                            "Invalid configuration parameter '{}': {}",
                            parameter,
                            reason
                        ]
                    )
                }
                ConfigurationError::MissingParameter { parameter } => {
                    gst::error_msg!(
                        gst::ResourceError::Settings,
                        ["Missing required configuration parameter: {}", parameter]
                    )
                }
                ConfigurationError::ConflictingConfiguration { details } => {
                    gst::error_msg!(
                        gst::ResourceError::Settings,
                        ["Conflicting configuration: {}", details]
                    )
                }
                ConfigurationError::ResourceAllocationFailed { resource } => {
                    gst::error_msg!(
                        gst::ResourceError::NoSpaceLeft,
                        ["Resource allocation failed: {}", resource]
                    )
                }
            },
            RtspError::Internal { message, .. } => {
                gst::error_msg!(gst::CoreError::Failed, ["Internal error: {}", message])
            }
        }
    }

    /// Log the error with appropriate GST_DEBUG level
    pub fn log_with_context(&self, cat: &gst::DebugCategory, context: &ErrorContext) {
        let mut message = format!("{}", self);

        if let Some(ref resource) = context.resource {
            message.push_str(&format!(" [Resource: {}]", resource));
        }

        if let Some(ref operation) = context.operation {
            message.push_str(&format!(" [Operation: {}]", operation));
        }

        if context.retry_count > 0 {
            message.push_str(&format!(" [Retry attempt: {}]", context.retry_count));
        }

        for (key, value) in &context.details {
            message.push_str(&format!(" [{}={}]", key, value));
        }

        let classification_info = format!(
            "Classification: {:?}, Retryable: {}, Suggested strategy: {:?}",
            self.classify(),
            self.is_retryable(),
            self.suggested_retry_strategy()
        );

        match self.classify() {
            ErrorClass::Transient => {
                gst::info!(*cat, "{} - {}", message, classification_info);
            }
            ErrorClass::RetryableWithBackoff => {
                gst::warning!(*cat, "{} - {}", message, classification_info);
            }
            ErrorClass::Permanent | ErrorClass::RequiresIntervention => {
                gst::error!(*cat, "{} - {}", message, classification_info);
            }
        }
    }
}

/// Result type alias for RTSP operations
pub type Result<T> = std::result::Result<T, RtspError>;

/// Extension trait for adding context to errors
pub trait ErrorContextExt<T> {
    /// Add context to the error
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> ErrorContext;
}

impl<T> ErrorContextExt<T> for Result<T> {
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> ErrorContext,
    {
        self.map_err(|e| {
            let _context = f();
            // In a real implementation, we'd store the context with the error
            // For now, we just return the error as-is
            e
        })
    }
}

/// Conversion from anyhow::Error for backward compatibility
impl From<anyhow::Error> for RtspError {
    fn from(err: anyhow::Error) -> Self {
        RtspError::Internal {
            message: err.to_string(),
            source: None,
        }
    }
}

/// Conversion from io::Error
impl From<io::Error> for RtspError {
    fn from(err: io::Error) -> Self {
        RtspError::Network(NetworkError::SocketError {
            message: err.to_string(),
            io_error: err,
        })
    }
}

/// Conversion from url::ParseError
impl From<url::ParseError> for RtspError {
    fn from(err: url::ParseError) -> Self {
        RtspError::Protocol(ProtocolError::InvalidUrl {
            url: String::new(),
            reason: err.to_string(),
        })
    }
}

/// Conversion from native_tls::Error (for TLS operations)
impl From<tokio_native_tls::native_tls::Error> for RtspError {
    fn from(err: tokio_native_tls::native_tls::Error) -> Self {
        RtspError::Network(NetworkError::TlsHandshakeFailed {
            details: err.to_string(),
        })
    }
}

/// Conversion from glib::Error
impl From<glib::Error> for RtspError {
    fn from(err: glib::Error) -> Self {
        RtspError::internal(err.to_string())
    }
}

/// Conversion from glib::BoolError
impl From<glib::BoolError> for RtspError {
    fn from(err: glib::BoolError) -> Self {
        RtspError::internal(err.to_string())
    }
}

/// Conversion from gst::FlowError
impl From<gst::FlowError> for RtspError {
    fn from(err: gst::FlowError) -> Self {
        RtspError::internal(format!("GStreamer flow error: {:?}", err))
    }
}

/// Conversion from HeaderParseError (for RTSP headers)
impl From<rtsp_types::headers::HeaderParseError> for RtspError {
    fn from(err: rtsp_types::headers::HeaderParseError) -> Self {
        RtspError::Protocol(ProtocolError::InvalidResponse {
            details: format!("Header parse error: {}", err),
        })
    }
}

/// Conversion from super::tcp_message::ReadError  
impl From<rtspsrc::tcp_message::ReadError> for RtspError {
    fn from(err: rtspsrc::tcp_message::ReadError) -> Self {
        RtspError::internal(format!("Read error: {:?}", err))
    }
}

/// Conversion from sdp_types::ParserError
impl From<sdp_types::ParserError> for RtspError {
    fn from(err: sdp_types::ParserError) -> Self {
        RtspError::Media(MediaError::SdpParsingFailed {
            details: err.to_string(),
        })
    }
}

/// Conversion from OldRtspError for backward compatibility
impl From<super::imp::OldRtspError> for RtspError {
    fn from(err: super::imp::OldRtspError) -> Self {
        use super::imp::OldRtspError;
        match err {
            OldRtspError::IOGeneric(e) => RtspError::from(e),
            OldRtspError::Read(e) => RtspError::from(e),
            OldRtspError::HeaderParser(e) => RtspError::from(e),
            OldRtspError::SDPParser(e) => RtspError::from(e),
            OldRtspError::UnexpectedMessage(expected, _msg) => {
                RtspError::Protocol(ProtocolError::InvalidResponse {
                    details: format!("Unexpected message: expected {}", expected),
                })
            }
            OldRtspError::InvalidMessage(msg) => {
                RtspError::Protocol(ProtocolError::InvalidResponse {
                    details: msg.to_string(),
                })
            }
            OldRtspError::Fatal(msg) => RtspError::internal(msg),
        }
    }
}

/// Conversion from gst::PadLinkError
impl From<gst::PadLinkError> for RtspError {
    fn from(err: gst::PadLinkError) -> Self {
        RtspError::internal(format!("Pad link error: {:?}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let err = NetworkError::ConnectionTimeout {
            host: "example.com".to_string(),
            port: 554,
            timeout: Duration::from_secs(10),
        };
        assert_eq!(err.classify(), ErrorClass::Transient);
        assert!(err.is_retryable());
        assert_eq!(
            err.suggested_retry_strategy(),
            Some(RetryStrategy::FirstWins)
        );
    }

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new()
            .with_resource("rtsp://camera.local/stream")
            .with_operation("SETUP")
            .with_retry_count(3)
            .add_detail("transport", "TCP");

        assert_eq!(
            context.resource,
            Some("rtsp://camera.local/stream".to_string())
        );
        assert_eq!(context.operation, Some("SETUP".to_string()));
        assert_eq!(context.retry_count, 3);
        assert_eq!(context.details.len(), 1);
    }

    #[test]
    fn test_network_error_classification() {
        // Test connection refused - should be retryable with backoff
        let err = NetworkError::ConnectionRefused {
            host: "192.168.1.100".to_string(),
            port: 554,
        };
        assert_eq!(err.classify(), ErrorClass::RetryableWithBackoff);
        assert!(err.is_retryable());
        assert!(err.suggested_retry_strategy().is_some());

        // Test connection timeout - should be transient
        let err = NetworkError::ConnectionTimeout {
            host: "camera.local".to_string(),
            port: 554,
            timeout: Duration::from_secs(10),
        };
        assert_eq!(err.classify(), ErrorClass::Transient);
        assert!(err.is_retryable());

        // Test TLS handshake failure - should be permanent
        let err = NetworkError::TlsHandshakeFailed {
            details: "Certificate verification failed".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::Permanent);
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_protocol_error_classification() {
        // Test 401 Unauthorized - requires intervention
        let err = ProtocolError::AuthenticationFailed {
            method: "Digest".to_string(),
            details: "Invalid credentials".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::RequiresIntervention);
        assert!(!err.is_retryable());

        // Test 404 Not Found - permanent
        let err = ProtocolError::StatusError {
            code: 404,
            message: "Stream not found".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::Permanent);
        assert!(!err.is_retryable());

        // Test invalid session - retryable with backoff
        let err = ProtocolError::InvalidSessionId {
            session_id: "12345".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::RetryableWithBackoff);
        assert!(err.is_retryable());
    }

    #[test]
    fn test_media_error_classification() {
        // Test stream sync lost - transient
        let err = MediaError::StreamSyncLost;
        assert_eq!(err.classify(), ErrorClass::Transient);
        assert!(err.is_retryable());

        // Test buffer overflow - transient
        let err = MediaError::BufferOverflow;
        assert_eq!(err.classify(), ErrorClass::Transient);
        assert!(err.is_retryable());
    }

    #[test]
    fn test_configuration_error_classification() {
        // All configuration errors should be permanent
        let err = ConfigurationError::InvalidParameter {
            parameter: "proxy-url".to_string(),
            reason: "Invalid URL format".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::Permanent);
        assert!(!err.is_retryable());

        let err = ConfigurationError::MissingParameter {
            parameter: "location".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::Permanent);
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_rtsp_error_conversion_to_gst_error() {
        // Test network error conversion
        let err = RtspError::Network(NetworkError::ConnectionRefused {
            host: "camera.local".to_string(),
            port: 554,
        });
        let gst_err = err.to_gst_error();
        assert!(gst_err.to_string().contains("Connection refused"));

        // Test protocol error conversion
        let err = RtspError::Protocol(ProtocolError::AuthenticationFailed {
            method: "Basic".to_string(),
            details: "Invalid username or password".to_string(),
        });
        let gst_err = err.to_gst_error();
        assert!(gst_err.to_string().contains("Authentication failed"));

        // Test configuration error conversion
        let err = RtspError::Configuration(ConfigurationError::InvalidParameter {
            parameter: "timeout".to_string(),
            reason: "Must be positive".to_string(),
        });
        let gst_err = err.to_gst_error();
        assert!(gst_err.to_string().contains("Invalid configuration"));
    }
}
