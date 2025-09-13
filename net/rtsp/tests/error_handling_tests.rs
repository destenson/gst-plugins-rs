// GStreamer RTSP Error Handling Tests
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use gst::prelude::*;
use gstrsrtsp::rtspsrc::error::{
    ConfigurationError, ErrorClass, ErrorClassification, ErrorContext, MediaError, NetworkError,
    ProtocolError, RtspError,
};
use gstrsrtsp::rtspsrc::error_recovery::{ErrorRecovery, RecoveryAction, TransportType};
use std::time::Duration;

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

    // Test NAT traversal - requires intervention
    let err = NetworkError::NatTraversalFailed {
        reason: "STUN server unreachable".to_string(),
    };
    assert_eq!(err.classify(), ErrorClass::RequiresIntervention);
    assert!(!err.is_retryable());
}

#[test]
fn test_protocol_error_classification() {
    // Test 503 Service Unavailable - should be retryable with backoff
    let err = ProtocolError::StatusError {
        code: 503,
        message: "Service Unavailable".to_string(),
    };
    assert_eq!(err.classify(), ErrorClass::RetryableWithBackoff);
    assert!(err.is_retryable());

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
    // Test unsupported codec - permanent
    let err = MediaError::UnsupportedCodec {
        codec: "H.265".to_string(),
    };
    assert_eq!(err.classify(), ErrorClass::Permanent);
    assert!(!err.is_retryable());

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
fn test_error_context() {
    let context = ErrorContext::new()
        .with_resource("rtsp://192.168.1.100:554/stream1")
        .with_operation("DESCRIBE")
        .with_retry_count(3)
        .add_detail("transport", "TCP")
        .add_detail("session", "ABC123");

    assert_eq!(
        context.resource,
        Some("rtsp://192.168.1.100:554/stream1".to_string())
    );
    assert_eq!(context.operation, Some("DESCRIBE".to_string()));
    assert_eq!(context.retry_count, 3);
    assert_eq!(context.details.len(), 2);
    assert_eq!(context.details[0], ("transport".to_string(), "TCP".to_string()));
    assert_eq!(context.details[1], ("session".to_string(), "ABC123".to_string()));
}

#[test]
fn test_rtsp_error_conversion_to_gst_error() {
    gst::init().unwrap();

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

    // Test media error conversion
    let err = RtspError::Media(MediaError::UnsupportedCodec {
        codec: "VP9".to_string(),
    });
    let gst_err = err.to_gst_error();
    assert!(gst_err.to_string().contains("Unsupported codec"));

    // Test configuration error conversion
    let err = RtspError::Configuration(ConfigurationError::InvalidParameter {
        parameter: "timeout".to_string(),
        reason: "Must be positive".to_string(),
    });
    let gst_err = err.to_gst_error();
    assert!(gst_err.to_string().contains("Invalid configuration"));
}

#[test]
fn test_error_recovery_actions() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-error-recovery",
        gst::DebugColorFlags::empty(),
        Some("Test Error Recovery"),
    );
    let mut recovery = ErrorRecovery::new(cat);

    // Test network timeout recovery - should suggest retry
    let err = RtspError::Network(NetworkError::ConnectionTimeout {
        host: "camera.local".to_string(),
        port: 554,
        timeout: Duration::from_secs(10),
    });
    let context = ErrorContext::new().with_retry_count(0);
    let action = recovery.determine_recovery_action(&err, &context);
    match action {
        RecoveryAction::Retry { .. } => {}
        _ => panic!("Expected Retry action for connection timeout"),
    }

    // Test connection refused with multiple retries - should suggest reconnect
    let err = RtspError::Network(NetworkError::ConnectionRefused {
        host: "camera.local".to_string(),
        port: 554,
    });
    let context = ErrorContext::new().with_retry_count(3);
    let action = recovery.determine_recovery_action(&err, &context);
    match action {
        RecoveryAction::Reconnect { .. } => {}
        _ => panic!("Expected Reconnect action after multiple retries"),
    }

    // Test NAT traversal failure - should suggest transport fallback
    let err = RtspError::Network(NetworkError::NatTraversalFailed {
        reason: "UDP blocked".to_string(),
    });
    let context = ErrorContext::new();
    let action = recovery.determine_recovery_action(&err, &context);
    match action {
        RecoveryAction::FallbackTransport { from, to } => {
            assert_eq!(from, TransportType::Udp);
            assert_eq!(to, TransportType::Tcp);
        }
        _ => panic!("Expected FallbackTransport action for NAT failure"),
    }

    // Test authentication failure - should wait for intervention
    let err = RtspError::Protocol(ProtocolError::AuthenticationFailed {
        method: "Digest".to_string(),
        details: "Invalid credentials".to_string(),
    });
    let context = ErrorContext::new();
    let action = recovery.determine_recovery_action(&err, &context);
    match action {
        RecoveryAction::WaitForIntervention { .. } => {}
        _ => panic!("Expected WaitForIntervention for auth failure"),
    }

    // Test server error (503) - should retry with backoff
    let err = RtspError::Protocol(ProtocolError::StatusError {
        code: 503,
        message: "Service Unavailable".to_string(),
    });
    let context = ErrorContext::new();
    let action = recovery.determine_recovery_action(&err, &context);
    match action {
        RecoveryAction::Retry { .. } => {}
        _ => panic!("Expected Retry action for 503 error"),
    }

    // Test stream sync lost - should reset pipeline
    let err = RtspError::Media(MediaError::StreamSyncLost);
    let context = ErrorContext::new();
    let action = recovery.determine_recovery_action(&err, &context);
    match action {
        RecoveryAction::ResetPipeline => {}
        _ => panic!("Expected ResetPipeline for stream sync lost"),
    }

    // Test unsupported feature - should be fatal
    let err = RtspError::Protocol(ProtocolError::UnsupportedFeature {
        feature: "RTSP/2.0".to_string(),
    });
    let context = ErrorContext::new();
    let action = recovery.determine_recovery_action(&err, &context);
    match action {
        RecoveryAction::Fatal => {}
        _ => panic!("Expected Fatal action for unsupported feature"),
    }
}

#[test]
fn test_recovery_statistics() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-recovery-stats",
        gst::DebugColorFlags::empty(),
        Some("Test Recovery Stats"),
    );
    let mut recovery = ErrorRecovery::new(cat);

    // Simulate some recovery attempts
    let err1 = RtspError::Network(NetworkError::ConnectionTimeout {
        host: "test.local".to_string(),
        port: 554,
        timeout: Duration::from_secs(5),
    });
    let context = ErrorContext::new();
    recovery.determine_recovery_action(&err1, &context);
    recovery.mark_recovery_successful();

    let err2 = RtspError::Protocol(ProtocolError::SessionError {
        details: "Session expired".to_string(),
    });
    recovery.determine_recovery_action(&err2, &context);
    // Don't mark as successful

    let stats = recovery.get_recovery_stats();
    assert_eq!(stats.total_attempts, 2);
    assert_eq!(stats.successful_attempts, 1);
    assert_eq!(stats.success_rate, 50.0);

    // Clear history and check stats
    recovery.clear_history();
    let stats = recovery.get_recovery_stats();
    assert_eq!(stats.total_attempts, 0);
    assert_eq!(stats.successful_attempts, 0);
    assert_eq!(stats.success_rate, 0.0);
}

#[tokio::test]
async fn test_execute_recovery_action() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-execute-recovery",
        gst::DebugColorFlags::empty(),
        Some("Test Execute Recovery"),
    );

    // Test retry action
    let action = RecoveryAction::Retry {
        strategy: gstrsrtsp::rtspsrc::retry::RetryStrategy::Linear,
        max_attempts: 3,
        delay: Duration::from_millis(10),
    };
    let start = std::time::Instant::now();
    let result =
        gstrsrtsp::rtspsrc::error_recovery::execute_recovery_action(&action, &cat).await;
    let elapsed = start.elapsed();
    assert!(result.is_ok());
    assert!(elapsed >= Duration::from_millis(10));

    // Test log and continue
    let action = RecoveryAction::LogAndContinue;
    let result =
        gstrsrtsp::rtspsrc::error_recovery::execute_recovery_action(&action, &cat).await;
    assert!(result.is_ok());

    // Test fatal action
    let action = RecoveryAction::Fatal;
    let result =
        gstrsrtsp::rtspsrc::error_recovery::execute_recovery_action(&action, &cat).await;
    assert!(result.is_err());

    // Test wait for intervention
    let action = RecoveryAction::WaitForIntervention {
        message: "Test intervention".to_string(),
    };
    let result =
        gstrsrtsp::rtspsrc::error_recovery::execute_recovery_action(&action, &cat).await;
    assert!(result.is_err());
}

#[test]
fn test_error_from_anyhow() {
    // Test conversion from anyhow::Error
    let anyhow_err = anyhow::anyhow!("Test error message");
    let rtsp_err: RtspError = anyhow_err.into();
    match rtsp_err {
        RtspError::Internal { message, .. } => {
            assert_eq!(message, "Test error message");
        }
        _ => panic!("Expected Internal error from anyhow conversion"),
    }
}

#[test]
fn test_error_from_io() {
    use std::io;

    // Test conversion from io::Error
    let io_err = io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused");
    let rtsp_err: RtspError = io_err.into();
    match rtsp_err {
        RtspError::Network(NetworkError::SocketError { .. }) => {}
        _ => panic!("Expected Network::SocketError from io::Error conversion"),
    }
}

#[test]
fn test_error_messages() {
    // Test that error messages are user-friendly and informative
    let err = NetworkError::ConnectionTimeout {
        host: "192.168.1.100".to_string(),
        port: 554,
        timeout: Duration::from_secs(30),
    };
    let msg = err.to_string();
    assert!(msg.contains("192.168.1.100"));
    assert!(msg.contains("554"));
    assert!(msg.contains("30s"));

    let err = ProtocolError::AuthenticationFailed {
        method: "Digest".to_string(),
        details: "realm=\"AXIS_ACCC8E012345\"".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("Digest"));
    assert!(msg.contains("realm=\"AXIS_ACCC8E012345\""));

    let err = MediaError::UnsupportedCodec {
        codec: "H.265/HEVC".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("H.265/HEVC"));
    assert!(msg.contains("Unsupported codec"));
}