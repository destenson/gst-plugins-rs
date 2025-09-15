// GStreamer RTSP Error Migration Example
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

//! Example showing how to migrate from anyhow to the new error handling system

use super::error::{ErrorContext, NetworkError, ProtocolError, Result, RtspError};
use super::error_recovery::{ErrorRecovery, RecoveryAction};
use gst::prelude::*;
use std::time::Duration;

/// Example: Converting a connection function from anyhow to new error types
///
/// Before:
/// ```rust
/// use anyhow::Result;
///
/// async fn connect_to_server(host: &str, port: u16) -> Result<TcpStream> {
///     let addr = format!("{}:{}", host, port);
///     let stream = TcpStream::connect(&addr).await?;
///     Ok(stream)
/// }
/// ```
///
/// After:
#[allow(dead_code)]
async fn connect_to_server(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<tokio::net::TcpStream> {
    use tokio::time::timeout as tokio_timeout;

    let addr = format!("{}:{}", host, port);

    match tokio_timeout(timeout, tokio::net::TcpStream::connect(&addr)).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(io_err)) => {
            // Convert IO errors to specific network errors
            if io_err.kind() == std::io::ErrorKind::ConnectionRefused {
                Err(RtspError::Network(NetworkError::ConnectionRefused {
                    host: host.to_string(),
                    port,
                }))
            } else if io_err.kind() == std::io::ErrorKind::UnexpectedEof {
                Err(RtspError::Network(NetworkError::ConnectionReset))
            } else {
                Err(RtspError::Network(NetworkError::SocketError {
                    message: format!("Failed to connect to {}:{}", host, port),
                    io_error: io_err,
                }))
            }
        }
        Err(_) => Err(RtspError::Network(NetworkError::ConnectionTimeout {
            host: host.to_string(),
            port,
            timeout,
        })),
    }
}

/// Example: Handling RTSP responses with proper error types
///
/// Before:
/// ```rust
/// use anyhow::{bail, ensure};
///
/// fn handle_rtsp_response(code: u16, message: &str) -> Result<()> {
///     ensure!(code != 401, "Authentication required");
///     ensure!(code < 400, "Client error: {} {}", code, message);
///     ensure!(code < 500, "Server error: {} {}", code, message);
///     Ok(())
/// }
/// ```
///
/// After:
#[allow(dead_code)]
fn handle_rtsp_response(code: u16, message: &str) -> Result<()> {
    match code {
        200..=299 => Ok(()),
        401 => Err(RtspError::Protocol(ProtocolError::AuthenticationFailed {
            method: "Unknown".to_string(),
            details: message.to_string(),
        })),
        403 => Err(RtspError::Protocol(ProtocolError::StatusError {
            code,
            message: format!("Forbidden: {}", message),
        })),
        404 => Err(RtspError::Protocol(ProtocolError::StatusError {
            code,
            message: format!("Not Found: {}", message),
        })),
        405 => Err(RtspError::Protocol(ProtocolError::MethodNotAllowed {
            method: message.to_string(),
        })),
        454 => Err(RtspError::Protocol(ProtocolError::InvalidSessionId {
            session_id: message.to_string(),
        })),
        500..=599 => Err(RtspError::Protocol(ProtocolError::StatusError {
            code,
            message: format!("Server Error: {}", message),
        })),
        _ => Err(RtspError::Protocol(ProtocolError::StatusError {
            code,
            message: message.to_string(),
        })),
    }
}

/// Example: Using error context for better debugging
#[allow(dead_code)]
async fn perform_rtsp_setup(url: &str, transport: &str, retry_count: u32) -> Result<String> {
    // Create context for this operation
    let context = ErrorContext::new()
        .with_resource(url)
        .with_operation("SETUP")
        .with_retry_count(retry_count)
        .add_detail("transport", transport);

    // Simulate an operation that might fail
    let result = setup_internal(url, transport).await;

    // Add context to any errors
    result.map_err(|e| {
        // Log the error with full context
        let cat = gst::DebugCategory::new(
            "rtsp-setup",
            gst::DebugColorFlags::empty(),
            Some("RTSP Setup"),
        );
        e.log_with_context(&cat, &context);
        e
    })
}

#[allow(dead_code)]
async fn setup_internal(url: &str, transport: &str) -> Result<String> {
    // Simulate various error conditions
    if url.is_empty() {
        return Err(RtspError::Configuration(
            super::error::ConfigurationError::MissingParameter {
                parameter: "URL".to_string(),
            },
        ));
    }

    if !transport.contains("RTP") {
        return Err(RtspError::Protocol(
            ProtocolError::TransportNegotiationFailed {
                reason: format!("Invalid transport: {}", transport),
            },
        ));
    }

    Ok("session-id-123".to_string())
}

/// Example: Using the error recovery system
#[allow(dead_code)]
async fn connect_with_recovery(
    host: &str,
    port: u16,
    cat: gst::DebugCategory,
) -> Result<tokio::net::TcpStream> {
    let mut recovery = ErrorRecovery::new(cat);
    let mut attempts = 0;
    let max_attempts = 5;

    loop {
        let context = ErrorContext::new()
            .with_resource(&format!("tcp://{}:{}", host, port))
            .with_operation("connect")
            .with_retry_count(attempts);

        match connect_to_server(host, port, Duration::from_secs(10)).await {
            Ok(stream) => {
                recovery.mark_recovery_successful();
                recovery.clear_history();
                return Ok(stream);
            }
            Err(err) => {
                let action = recovery.determine_recovery_action(&err, &context);

                match action {
                    RecoveryAction::Retry { delay, .. } => {
                        if attempts >= max_attempts {
                            return Err(err);
                        }
                        gst::info!(cat, "Retrying connection after {:?}", delay);
                        tokio::time::sleep(delay).await;
                        attempts += 1;
                    }
                    RecoveryAction::Fatal => {
                        gst::error!(cat, "Fatal error, cannot recover: {}", err);
                        return Err(err);
                    }
                    _ => {
                        // Handle other recovery actions as needed
                        return Err(err);
                    }
                }
            }
        }
    }
}

/// Example: Converting errors to GStreamer messages
#[allow(dead_code)]
fn post_error_to_bus(element: &gst::Element, error: RtspError) {
    let gst_error = error.to_gst_error();
    element.post_error_message(gst_error);
}

/// Example: Pattern matching on specific error types
#[allow(dead_code)]
fn handle_error(error: RtspError) {
    match error {
        RtspError::Network(NetworkError::ConnectionTimeout { host, port, .. }) => {
            println!("Connection to {}:{} timed out", host, port);
        }
        RtspError::Protocol(ProtocolError::AuthenticationFailed { method, .. }) => {
            println!("Authentication failed using method: {}", method);
        }
        RtspError::Media(super::error::MediaError::UnsupportedCodec { codec }) => {
            println!("Codec '{}' is not supported", codec);
        }
        RtspError::Configuration(_) => {
            println!("Configuration error - check your settings");
        }
        _ => {
            println!("Error occurred: {}", error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_error_conversion() {
        // Test that our example functions compile and work correctly
        let result = handle_rtsp_response(404, "Stream not found");
        assert!(result.is_err());
        match result {
            Err(RtspError::Protocol(ProtocolError::StatusError { code, .. })) => {
                assert_eq!(code, 404);
            }
            _ => panic!("Expected protocol error"),
        }
    }

    #[tokio::test]
    async fn test_context_example() {
        let result = perform_rtsp_setup("", "TCP", 0).await;
        assert!(result.is_err());
        match result {
            Err(RtspError::Configuration(_)) => {}
            _ => panic!("Expected configuration error"),
        }

        let result = perform_rtsp_setup("rtsp://example.com", "TCP", 0).await;
        assert!(result.is_err());
        match result {
            Err(RtspError::Protocol(_)) => {}
            _ => panic!("Expected protocol error"),
        }

        let result = perform_rtsp_setup("rtsp://example.com", "RTP/AVP", 0).await;
        assert!(result.is_ok());
    }
}
