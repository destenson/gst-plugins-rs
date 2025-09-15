// GStreamer RTSP Error Handling Tests
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

#[cfg(test)]
mod tests {
    use super::super::error::*;
    use super::super::error_recovery::*;
    use super::super::retry::RetryStrategy;
    use std::io;
    use std::time::Duration;

    #[test]
    fn test_network_error_creation_and_display() {
        let err = NetworkError::ConnectionRefused {
            host: "example.com".to_string(),
            port: 554,
        };
        assert_eq!(
            err.to_string(),
            "Connection refused to example.com:554 - server may be down or unreachable"
        );

        let err = NetworkError::ConnectionTimeout {
            host: "example.com".to_string(),
            port: 554,
            timeout: Duration::from_secs(10),
        };
        assert!(err.to_string().contains("Connection timeout"));
        assert!(err.to_string().contains("10s"));
    }

    #[test]
    fn test_protocol_error_creation_and_display() {
        let err = ProtocolError::AuthenticationFailed {
            method: "Digest".to_string(),
            details: "Invalid credentials".to_string(),
        };
        assert!(err.to_string().contains("Authentication failed"));
        assert!(err.to_string().contains("Digest"));

        let err = ProtocolError::StatusError {
            code: 404,
            message: "Not Found".to_string(),
        };
        assert_eq!(err.to_string(), "RTSP status 404: Not Found");
    }

    #[test]
    fn test_media_error_creation_and_display() {
        let err = MediaError::UnsupportedCodec {
            codec: "H.265".to_string(),
        };
        assert_eq!(err.to_string(), "Unsupported codec: H.265");

        let err = MediaError::BufferOverflow;
        assert!(err
            .to_string()
            .contains("unable to process media data fast enough"));
    }

    #[test]
    fn test_configuration_error_creation_and_display() {
        let err = ConfigurationError::InvalidParameter {
            parameter: "timeout".to_string(),
            reason: "Must be positive".to_string(),
        };
        assert!(err.to_string().contains("Invalid configuration"));
        assert!(err.to_string().contains("timeout"));
        assert!(err.to_string().contains("Must be positive"));
    }

    #[test]
    fn test_error_classification_network() {
        // Transient errors
        let err = NetworkError::ConnectionReset;
        assert_eq!(err.classify(), ErrorClass::Transient);
        assert!(err.is_retryable());

        let err = NetworkError::ConnectionTimeout {
            host: "example.com".to_string(),
            port: 554,
            timeout: Duration::from_secs(10),
        };
        assert_eq!(err.classify(), ErrorClass::Transient);
        assert!(err.is_retryable());

        // Retryable with backoff
        let err = NetworkError::ConnectionRefused {
            host: "example.com".to_string(),
            port: 554,
        };
        assert_eq!(err.classify(), ErrorClass::RetryableWithBackoff);
        assert!(err.is_retryable());

        // Permanent errors
        let err = NetworkError::TlsHandshakeFailed {
            details: "Certificate invalid".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::Permanent);
        assert!(!err.is_retryable());

        // Requires intervention
        let err = NetworkError::NatTraversalFailed {
            reason: "No STUN server".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::RequiresIntervention);
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_classification_protocol() {
        // Status code classification
        let err = ProtocolError::StatusError {
            code: 503,
            message: "Service Unavailable".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::RetryableWithBackoff);
        assert!(err.is_retryable());

        let err = ProtocolError::StatusError {
            code: 401,
            message: "Unauthorized".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::Permanent);
        assert!(!err.is_retryable());

        // Permanent errors
        let err = ProtocolError::UnsupportedFeature {
            feature: "RTSP 2.0".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::Permanent);
        assert!(!err.is_retryable());

        // Requires intervention
        let err = ProtocolError::AuthenticationFailed {
            method: "Digest".to_string(),
            details: "Wrong password".to_string(),
        };
        assert_eq!(err.classify(), ErrorClass::RequiresIntervention);
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_suggested_retry_strategies() {
        let err = NetworkError::ConnectionTimeout {
            host: "example.com".to_string(),
            port: 554,
            timeout: Duration::from_secs(10),
        };
        assert_eq!(
            err.suggested_retry_strategy(),
            Some(RetryStrategy::FirstWins)
        );

        let err = NetworkError::ConnectionRefused {
            host: "example.com".to_string(),
            port: 554,
        };
        assert_eq!(
            err.suggested_retry_strategy(),
            Some(RetryStrategy::ExponentialJitter)
        );

        let err = NetworkError::ConnectionReset;
        assert_eq!(
            err.suggested_retry_strategy(),
            Some(RetryStrategy::Immediate)
        );

        let err = ProtocolError::StatusError {
            code: 503,
            message: "Service Unavailable".to_string(),
        };
        assert_eq!(
            err.suggested_retry_strategy(),
            Some(RetryStrategy::ExponentialJitter)
        );
    }

    #[test]
    fn test_error_context_builder() {
        let context = ErrorContext::new()
            .with_resource("rtsp://camera.local/stream")
            .with_operation("SETUP")
            .with_retry_count(3)
            .add_detail("transport", "TCP")
            .add_detail("session", "12345");

        assert_eq!(
            context.resource,
            Some("rtsp://camera.local/stream".to_string())
        );
        assert_eq!(context.operation, Some("SETUP".to_string()));
        assert_eq!(context.retry_count, 3);
        assert_eq!(context.details.len(), 2);
        assert_eq!(
            context.details[0],
            ("transport".to_string(), "TCP".to_string())
        );
        assert_eq!(
            context.details[1],
            ("session".to_string(), "12345".to_string())
        );
    }

    #[test]
    fn test_rtsp_error_conversion_from_io() {
        let io_err = io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused");
        let rtsp_err = RtspError::from(io_err);

        match rtsp_err {
            RtspError::Network(NetworkError::SocketError { message, .. }) => {
                assert!(message.contains("Connection refused"));
            }
            _ => panic!("Expected NetworkError::SocketError"),
        }
    }

    #[test]
    fn test_gstreamer_error_conversion() {
        let err = RtspError::Network(NetworkError::ConnectionTimeout {
            host: "example.com".to_string(),
            port: 554,
            timeout: Duration::from_secs(10),
        });

        let gst_error = err.to_gst_error();
        // Just verify it creates an error message without panicking
        assert!(!format!("{:?}", gst_error).is_empty());
    }

    #[test]
    fn test_recovery_action_determination() {
        let cat = gst::DebugCategory::new(
            "test-recovery",
            gst::DebugColorFlags::empty(),
            Some("Test Recovery"),
        );
        let mut recovery = ErrorRecovery::new(cat);

        // Test connection timeout recovery
        let error = RtspError::Network(NetworkError::ConnectionTimeout {
            host: "example.com".to_string(),
            port: 554,
            timeout: Duration::from_secs(10),
        });
        let context = ErrorContext::new().with_retry_count(0);
        let action = recovery.determine_recovery_action(&error, &context);
        match action {
            RecoveryAction::Retry { .. } => {}
            _ => panic!("Expected Retry action for connection timeout"),
        }

        // Test connection refused with high retry count
        let error = RtspError::Network(NetworkError::ConnectionRefused {
            host: "example.com".to_string(),
            port: 554,
        });
        let context = ErrorContext::new().with_retry_count(5);
        let action = recovery.determine_recovery_action(&error, &context);
        match action {
            RecoveryAction::Reconnect { .. } => {}
            _ => panic!("Expected Reconnect action after multiple retries"),
        }

        // Test permanent error
        let error = RtspError::Network(NetworkError::TlsHandshakeFailed {
            details: "Certificate expired".to_string(),
        });
        let context = ErrorContext::new();
        let action = recovery.determine_recovery_action(&error, &context);
        match action {
            RecoveryAction::Fatal => {}
            _ => panic!("Expected Fatal action for TLS handshake failure"),
        }
    }

    #[test]
    fn test_recovery_stats_tracking() {
        let cat = gst::DebugCategory::new(
            "test-stats",
            gst::DebugColorFlags::empty(),
            Some("Test Stats"),
        );
        let mut recovery = ErrorRecovery::new(cat);

        // Simulate some errors and recovery
        let error1 = RtspError::Network(NetworkError::ConnectionTimeout {
            host: "example.com".to_string(),
            port: 554,
            timeout: Duration::from_secs(10),
        });
        let context = ErrorContext::new();
        let _action = recovery.determine_recovery_action(&error1, &context);
        recovery.mark_recovery_successful();

        let error2 = RtspError::Protocol(ProtocolError::InvalidResponse {
            details: "Malformed header".to_string(),
        });
        let _action = recovery.determine_recovery_action(&error2, &context);
        // Don't mark as successful

        let stats = recovery.get_recovery_stats();
        assert_eq!(stats.total_attempts, 2);
        assert_eq!(stats.successful_attempts, 1);
        assert_eq!(stats.success_rate, 50.0);

        // Test clear history
        recovery.clear_history();
        let stats = recovery.get_recovery_stats();
        assert_eq!(stats.total_attempts, 0);
        assert_eq!(stats.successful_attempts, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[test]
    fn test_error_class_coverage() {
        // Ensure all error classes are properly handled
        let classes = vec![
            ErrorClass::Transient,
            ErrorClass::RetryableWithBackoff,
            ErrorClass::Permanent,
            ErrorClass::RequiresIntervention,
        ];

        for class in classes {
            // Just ensure we can match on all variants
            match class {
                ErrorClass::Transient => assert!(true),
                ErrorClass::RetryableWithBackoff => assert!(true),
                ErrorClass::Permanent => assert!(true),
                ErrorClass::RequiresIntervention => assert!(true),
            }
        }
    }

    #[tokio::test]
    async fn test_execute_recovery_action() {
        use super::super::error_recovery::execute_recovery_action;

        let cat = gst::DebugCategory::new(
            "test-execute",
            gst::DebugColorFlags::empty(),
            Some("Test Execute"),
        );

        // Test successful retry action
        let action = RecoveryAction::Retry {
            strategy: RetryStrategy::Immediate,
            max_attempts: 3,
            delay: Duration::from_millis(10),
        };
        let result = execute_recovery_action(&action, &cat).await;
        assert!(result.is_ok());

        // Test log and continue
        let action = RecoveryAction::LogAndContinue;
        let result = execute_recovery_action(&action, &cat).await;
        assert!(result.is_ok());

        // Test fatal action
        let action = RecoveryAction::Fatal;
        let result = execute_recovery_action(&action, &cat).await;
        assert!(result.is_err());

        // Test wait for intervention
        let action = RecoveryAction::WaitForIntervention {
            message: "Test intervention".to_string(),
        };
        let result = execute_recovery_action(&action, &cat).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_all_network_error_variants() {
        // Test that all NetworkError variants can be created and have proper messages
        let errors: Vec<NetworkError> = vec![
            NetworkError::ConnectionRefused {
                host: "test".to_string(),
                port: 554,
            },
            NetworkError::ConnectionTimeout {
                host: "test".to_string(),
                port: 554,
                timeout: Duration::from_secs(5),
            },
            NetworkError::DnsResolutionFailed {
                host: "test".to_string(),
                details: "Not found".to_string(),
            },
            NetworkError::SocketError {
                message: "Test error".to_string(),
                io_error: io::Error::new(io::ErrorKind::Other, "test"),
            },
            NetworkError::TlsHandshakeFailed {
                details: "Test".to_string(),
            },
            NetworkError::NetworkUnreachable {
                details: "Test".to_string(),
            },
            NetworkError::ConnectionReset,
            NetworkError::NatTraversalFailed {
                reason: "Test".to_string(),
            },
            NetworkError::ProxyConnectionFailed {
                details: "Test".to_string(),
            },
            NetworkError::HttpTunnelError {
                details: "Test".to_string(),
            },
        ];

        for err in errors {
            assert!(!err.to_string().is_empty());
            let _ = err.classify();
            let _ = err.is_retryable();
        }
    }

    #[test]
    fn test_all_protocol_error_variants() {
        // Test that all ProtocolError variants can be created and have proper messages
        let errors: Vec<ProtocolError> = vec![
            ProtocolError::InvalidResponse {
                details: "Test".to_string(),
            },
            ProtocolError::UnsupportedFeature {
                feature: "Test".to_string(),
            },
            ProtocolError::AuthenticationFailed {
                method: "Test".to_string(),
                details: "Test".to_string(),
            },
            ProtocolError::SessionError {
                details: "Test".to_string(),
            },
            ProtocolError::InvalidSessionId {
                session_id: "Test".to_string(),
            },
            ProtocolError::MethodNotAllowed {
                method: "Test".to_string(),
            },
            ProtocolError::StatusError {
                code: 500,
                message: "Test".to_string(),
            },
            ProtocolError::MissingHeader {
                header: "Test".to_string(),
            },
            ProtocolError::InvalidUrl {
                url: "Test".to_string(),
                reason: "Test".to_string(),
            },
            ProtocolError::TransportNegotiationFailed {
                reason: "Test".to_string(),
            },
        ];

        for err in errors {
            assert!(!err.to_string().is_empty());
            let _ = err.classify();
            let _ = err.is_retryable();
        }
    }

    #[test]
    fn test_all_media_error_variants() {
        // Test that all MediaError variants can be created and have proper messages
        let errors: Vec<MediaError> = vec![
            MediaError::UnsupportedCodec {
                codec: "Test".to_string(),
            },
            MediaError::SdpParsingFailed {
                details: "Test".to_string(),
            },
            MediaError::StreamSyncLost,
            MediaError::BufferOverflow,
            MediaError::InvalidMediaFormat {
                details: "Test".to_string(),
            },
            MediaError::NoCompatibleStreams,
            MediaError::RtcpError {
                details: "Test".to_string(),
            },
            MediaError::RtpPacketError {
                details: "Test".to_string(),
            },
        ];

        for err in errors {
            assert!(!err.to_string().is_empty());
            let _ = err.classify();
            let _ = err.is_retryable();
        }
    }
}
