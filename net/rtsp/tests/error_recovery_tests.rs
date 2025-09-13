// GStreamer RTSP Error Recovery Tests
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
    ErrorContext, MediaError, NetworkError, ProtocolError, RtspError,
};
use gstrsrtsp::rtspsrc::error_recovery::{ErrorRecovery, RecoveryAction};
use std::time::Duration;

#[test]
fn test_error_recovery_max_attempts() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-max-attempts",
        gst::DebugColorFlags::empty(),
        Some("Test Max Attempts"),
    );
    let mut recovery = ErrorRecovery::new(cat);

    // Simulate maximum recovery attempts
    let err = RtspError::Network(NetworkError::ConnectionTimeout {
        host: "test.local".to_string(),
        port: 554,
        timeout: Duration::from_secs(5),
    });
    let context = ErrorContext::new();

    // Keep trying until we hit the limit
    for i in 0..15 {
        let action = recovery.determine_recovery_action(&err, &context);
        if i < 10 {
            // Should still be retrying
            match action {
                RecoveryAction::Fatal => panic!("Should not be fatal before max attempts"),
                _ => {}
            }
        } else {
            // Should be fatal after max attempts
            match action {
                RecoveryAction::Fatal => {}
                _ => panic!("Should be fatal after max attempts"),
            }
        }
    }
}

#[test]
fn test_transport_fallback_sequence() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-transport-fallback",
        gst::DebugColorFlags::empty(),
        Some("Test Transport Fallback"),
    );
    let mut recovery = ErrorRecovery::new(cat);

    // Test NAT failure -> fallback from UDP to TCP
    let nat_err = RtspError::Network(NetworkError::NatTraversalFailed {
        reason: "UDP ports blocked".to_string(),
    });
    let context = ErrorContext::new();
    let action = recovery.determine_recovery_action(&nat_err, &context);
    match action {
        RecoveryAction::FallbackTransport { from, to } => {
            assert_eq!(
                from,
                gstrsrtsp::rtspsrc::error_recovery::TransportType::Udp
            );
            assert_eq!(
                to,
                gstrsrtsp::rtspsrc::error_recovery::TransportType::Tcp
            );
        }
        _ => panic!("Expected transport fallback for NAT failure"),
    }

    // Test HTTP tunnel failure -> fallback to TCP
    let tunnel_err = RtspError::Network(NetworkError::HttpTunnelError {
        details: "Proxy authentication failed".to_string(),
    });
    let action = recovery.determine_recovery_action(&tunnel_err, &context);
    match action {
        RecoveryAction::FallbackTransport { from, to } => {
            assert_eq!(
                from,
                gstrsrtsp::rtspsrc::error_recovery::TransportType::HttpTunnel
            );
            assert_eq!(
                to,
                gstrsrtsp::rtspsrc::error_recovery::TransportType::Tcp
            );
        }
        _ => panic!("Expected transport fallback for HTTP tunnel error"),
    }

    // Test transport negotiation failure
    let transport_err = RtspError::Protocol(ProtocolError::TransportNegotiationFailed {
        reason: "No compatible transport".to_string(),
    });
    let context_first = ErrorContext::new().with_retry_count(0);
    let action = recovery.determine_recovery_action(&transport_err, &context_first);
    match action {
        RecoveryAction::FallbackTransport { .. } => {}
        _ => panic!("Expected transport fallback on first attempt"),
    }

    // Second attempt should be fatal
    let context_second = ErrorContext::new().with_retry_count(1);
    let action = recovery.determine_recovery_action(&transport_err, &context_second);
    match action {
        RecoveryAction::Fatal => {}
        _ => panic!("Expected fatal on second transport negotiation failure"),
    }
}

#[test]
fn test_session_error_recovery() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-session-recovery",
        gst::DebugColorFlags::empty(),
        Some("Test Session Recovery"),
    );
    let mut recovery = ErrorRecovery::new(cat);

    // Test session error - should reconnect with session reset
    let session_err = RtspError::Protocol(ProtocolError::SessionError {
        details: "Session timeout".to_string(),
    });
    let context = ErrorContext::new();
    let action = recovery.determine_recovery_action(&session_err, &context);
    match action {
        RecoveryAction::Reconnect {
            reset_session,
            delay,
        } => {
            assert!(reset_session);
            assert!(delay >= Duration::from_secs(1));
        }
        _ => panic!("Expected Reconnect with session reset for session error"),
    }

    // Test invalid session ID - should also reconnect
    let invalid_session = RtspError::Protocol(ProtocolError::InvalidSessionId {
        session_id: "expired-session-123".to_string(),
    });
    let action = recovery.determine_recovery_action(&invalid_session, &context);
    match action {
        RecoveryAction::Reconnect {
            reset_session, ..
        } => {
            assert!(reset_session);
        }
        _ => panic!("Expected Reconnect for invalid session ID"),
    }
}

#[test]
fn test_media_error_recovery() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-media-recovery",
        gst::DebugColorFlags::empty(),
        Some("Test Media Recovery"),
    );
    let mut recovery = ErrorRecovery::new(cat);
    let context = ErrorContext::new();

    // Stream sync lost - should reset pipeline
    let sync_err = RtspError::Media(MediaError::StreamSyncLost);
    let action = recovery.determine_recovery_action(&sync_err, &context);
    match action {
        RecoveryAction::ResetPipeline => {}
        _ => panic!("Expected ResetPipeline for stream sync lost"),
    }

    // Buffer overflow - should log and continue
    let buffer_err = RtspError::Media(MediaError::BufferOverflow);
    let action = recovery.determine_recovery_action(&buffer_err, &context);
    match action {
        RecoveryAction::LogAndContinue => {}
        _ => panic!("Expected LogAndContinue for buffer overflow"),
    }

    // Unsupported codec - should be fatal
    let codec_err = RtspError::Media(MediaError::UnsupportedCodec {
        codec: "unknown-codec".to_string(),
    });
    let action = recovery.determine_recovery_action(&codec_err, &context);
    match action {
        RecoveryAction::Fatal => {}
        _ => panic!("Expected Fatal for unsupported codec"),
    }
}

#[test]
fn test_server_error_recovery() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-server-recovery",
        gst::DebugColorFlags::empty(),
        Some("Test Server Recovery"),
    );
    let mut recovery = ErrorRecovery::new(cat);
    let context = ErrorContext::new();

    // 500 Internal Server Error - should retry with backoff
    let err_500 = RtspError::Protocol(ProtocolError::StatusError {
        code: 500,
        message: "Internal Server Error".to_string(),
    });
    let action = recovery.determine_recovery_action(&err_500, &context);
    match action {
        RecoveryAction::Retry { max_attempts, .. } => {
            assert!(max_attempts > 0);
        }
        _ => panic!("Expected Retry for 500 error"),
    }

    // 502 Bad Gateway - should retry with backoff
    let err_502 = RtspError::Protocol(ProtocolError::StatusError {
        code: 502,
        message: "Bad Gateway".to_string(),
    });
    let action = recovery.determine_recovery_action(&err_502, &context);
    match action {
        RecoveryAction::Retry { .. } => {}
        _ => panic!("Expected Retry for 502 error"),
    }

    // 503 Service Unavailable - should retry with backoff
    let err_503 = RtspError::Protocol(ProtocolError::StatusError {
        code: 503,
        message: "Service Unavailable".to_string(),
    });
    let action = recovery.determine_recovery_action(&err_503, &context);
    match action {
        RecoveryAction::Retry { .. } => {}
        _ => panic!("Expected Retry for 503 error"),
    }
}

#[test]
fn test_recovery_history_tracking() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-history",
        gst::DebugColorFlags::empty(),
        Some("Test History"),
    );
    let mut recovery = ErrorRecovery::new(cat);

    // Make several recovery attempts
    let errors = vec![
        RtspError::Network(NetworkError::ConnectionTimeout {
            host: "test1.local".to_string(),
            port: 554,
            timeout: Duration::from_secs(5),
        }),
        RtspError::Protocol(ProtocolError::SessionError {
            details: "Session expired".to_string(),
        }),
        RtspError::Media(MediaError::BufferOverflow),
    ];

    let context = ErrorContext::new();
    for (i, err) in errors.iter().enumerate() {
        recovery.determine_recovery_action(err, &context);
        if i == 0 || i == 2 {
            recovery.mark_recovery_successful();
        }
    }

    let stats = recovery.get_recovery_stats();
    assert_eq!(stats.total_attempts, 3);
    assert_eq!(stats.successful_attempts, 2);
    assert!(stats.success_rate > 66.0 && stats.success_rate < 67.0);

    // Clear history
    recovery.clear_history();
    let stats = recovery.get_recovery_stats();
    assert_eq!(stats.total_attempts, 0);
}

#[test]
fn test_network_unreachable_infinite_retry() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-unreachable",
        gst::DebugColorFlags::empty(),
        Some("Test Unreachable"),
    );
    let mut recovery = ErrorRecovery::new(cat);

    let err = RtspError::Network(NetworkError::NetworkUnreachable {
        details: "No route to host".to_string(),
    });
    let context = ErrorContext::new();
    let action = recovery.determine_recovery_action(&err, &context);

    match action {
        RecoveryAction::Retry { max_attempts, .. } => {
            assert_eq!(max_attempts, -1); // Infinite retries
        }
        _ => panic!("Expected infinite retry for network unreachable"),
    }
}

#[test]
fn test_connection_reset_quick_reconnect() {
    gst::init().unwrap();
    let cat = gst::DebugCategory::new(
        "test-reset",
        gst::DebugColorFlags::empty(),
        Some("Test Reset"),
    );
    let mut recovery = ErrorRecovery::new(cat);

    let err = RtspError::Network(NetworkError::ConnectionReset);
    let context = ErrorContext::new();
    let action = recovery.determine_recovery_action(&err, &context);

    match action {
        RecoveryAction::Reconnect {
            reset_session,
            delay,
        } => {
            assert!(!reset_session); // Don't reset session for connection reset
            assert!(delay <= Duration::from_secs(1)); // Quick reconnect
        }
        _ => panic!("Expected quick reconnect for connection reset"),
    }
}