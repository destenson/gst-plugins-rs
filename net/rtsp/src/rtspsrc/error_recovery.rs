#![allow(unused)]
// GStreamer RTSP Error Recovery Module
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use std::time::Duration;

use super::error::{ErrorClass, ErrorContext, NetworkError, ProtocolError, RtspError};
use super::retry::{RetryConfig, RetryStrategy};

/// Recovery action to take for an error
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Retry the operation with the specified strategy
    Retry {
        strategy: RetryStrategy,
        max_attempts: i32,
        delay: Duration,
    },
    /// Reconnect to the server and retry
    Reconnect {
        reset_session: bool,
        delay: Duration,
    },
    /// Try an alternative transport method
    FallbackTransport {
        from: TransportType,
        to: TransportType,
    },
    /// Reset the entire pipeline
    ResetPipeline,
    /// Log and continue (non-fatal error)
    LogAndContinue,
    /// Fatal error, stop the pipeline
    Fatal,
    /// Wait for user intervention
    WaitForIntervention { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    Tcp,
    Udp,
    UdpMulticast,
    HttpTunnel,
}

/// Error recovery manager
pub struct ErrorRecovery {
    cat: gst::DebugCategory,
    retry_configs: std::collections::HashMap<ErrorClass, RetryConfig>,
    recovery_history: Vec<RecoveryAttempt>,
    max_recovery_attempts: u32,
}

#[derive(Debug, Clone)]
struct RecoveryAttempt {
    error: String,
    action: RecoveryAction,
    timestamp: std::time::Instant,
    successful: bool,
}

impl ErrorRecovery {
    pub fn new(cat: gst::DebugCategory) -> Self {
        let mut retry_configs = std::collections::HashMap::new();

        // Configure retry strategies for different error classes
        retry_configs.insert(
            ErrorClass::Transient,
            RetryConfig {
                strategy: RetryStrategy::Immediate,
                max_attempts: 3,
                initial_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(5),
                linear_step: Duration::from_millis(500),
            },
        );

        retry_configs.insert(
            ErrorClass::RetryableWithBackoff,
            RetryConfig {
                strategy: RetryStrategy::ExponentialJitter,
                max_attempts: 5,
                initial_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(30),
                linear_step: Duration::from_secs(2),
            },
        );

        Self {
            cat,
            retry_configs,
            recovery_history: Vec::new(),
            max_recovery_attempts: 10,
        }
    }

    /// Determine the recovery action for an error
    pub fn determine_recovery_action(
        &mut self,
        error: &RtspError,
        context: &ErrorContext,
    ) -> RecoveryAction {
        // Log the error with context
        error.log_with_context(&self.cat, context);

        // Check if we've exceeded max recovery attempts
        if self.recovery_history.len() >= self.max_recovery_attempts as usize {
            gst::error!(
                self.cat,
                "Exceeded maximum recovery attempts ({})",
                self.max_recovery_attempts
            );
            return RecoveryAction::Fatal;
        }

        // Determine action based on error type and classification
        let action = match error {
            RtspError::Network(net_err) => self.handle_network_error(net_err, context),
            RtspError::Protocol(proto_err) => self.handle_protocol_error(proto_err, context),
            RtspError::Media(media_err) => self.handle_media_error(media_err),
            RtspError::Configuration(_) => RecoveryAction::Fatal,
            RtspError::Internal { .. } => RecoveryAction::Fatal,
        };

        // Record the recovery attempt
        self.recovery_history.push(RecoveryAttempt {
            error: error.to_string(),
            action: action.clone(),
            timestamp: std::time::Instant::now(),
            successful: false, // Will be updated after recovery attempt
        });

        gst::info!(
            self.cat,
            "Recovery action for error '{}': {:?}",
            error,
            action
        );

        action
    }

    fn handle_network_error(&self, error: &NetworkError, context: &ErrorContext) -> RecoveryAction {
        match error {
            NetworkError::ConnectionRefused { .. } | NetworkError::ConnectionTimeout { .. } => {
                if context.retry_count < 3 {
                    RecoveryAction::Retry {
                        strategy: RetryStrategy::ExponentialJitter,
                        max_attempts: 5,
                        delay: Duration::from_secs(1),
                    }
                } else {
                    RecoveryAction::Reconnect {
                        reset_session: true,
                        delay: Duration::from_secs(5),
                    }
                }
            }
            NetworkError::DnsResolutionFailed { .. } => RecoveryAction::Retry {
                strategy: RetryStrategy::Linear,
                max_attempts: 3,
                delay: Duration::from_secs(2),
            },
            NetworkError::ConnectionReset | NetworkError::SocketError { .. } => {
                RecoveryAction::Reconnect {
                    reset_session: false,
                    delay: Duration::from_millis(500),
                }
            }
            NetworkError::TlsHandshakeFailed { .. } => RecoveryAction::Fatal,
            NetworkError::NetworkUnreachable { .. } => RecoveryAction::Retry {
                strategy: RetryStrategy::Exponential,
                max_attempts: -1, // Infinite retries
                delay: Duration::from_secs(5),
            },
            NetworkError::NatTraversalFailed { .. } => RecoveryAction::FallbackTransport {
                from: TransportType::Udp,
                to: TransportType::Tcp,
            },
            NetworkError::ProxyConnectionFailed { .. } | NetworkError::HttpTunnelError { .. } => {
                RecoveryAction::FallbackTransport {
                    from: TransportType::HttpTunnel,
                    to: TransportType::Tcp,
                }
            }
        }
    }

    fn handle_protocol_error(
        &self,
        error: &ProtocolError,
        context: &ErrorContext,
    ) -> RecoveryAction {
        match error {
            ProtocolError::InvalidResponse { .. } => RecoveryAction::LogAndContinue,
            ProtocolError::UnsupportedFeature { .. } => RecoveryAction::Fatal,
            ProtocolError::AuthenticationFailed { .. } => RecoveryAction::WaitForIntervention {
                message: "Authentication failed. Please check credentials.".to_string(),
            },
            ProtocolError::SessionError { .. } | ProtocolError::InvalidSessionId { .. } => {
                RecoveryAction::Reconnect {
                    reset_session: true,
                    delay: Duration::from_secs(1),
                }
            }
            ProtocolError::MethodNotAllowed { .. } => RecoveryAction::Fatal,
            ProtocolError::StatusError { code, .. } => {
                if *code >= 500 {
                    // Server error - retry with backoff
                    RecoveryAction::Retry {
                        strategy: RetryStrategy::ExponentialJitter,
                        max_attempts: 5,
                        delay: Duration::from_secs(2),
                    }
                } else if *code == 401 || *code == 403 {
                    // Authentication/authorization error
                    RecoveryAction::WaitForIntervention {
                        message: format!("HTTP {} error. Check credentials.", code),
                    }
                } else {
                    RecoveryAction::Fatal
                }
            }
            ProtocolError::MissingHeader { .. } => RecoveryAction::LogAndContinue,
            ProtocolError::InvalidUrl { .. } => RecoveryAction::Fatal,
            ProtocolError::TransportNegotiationFailed { .. } => {
                if context.retry_count == 0 {
                    RecoveryAction::FallbackTransport {
                        from: TransportType::Udp,
                        to: TransportType::Tcp,
                    }
                } else {
                    RecoveryAction::Fatal
                }
            }
        }
    }

    fn handle_media_error(&self, error: &super::error::MediaError) -> RecoveryAction {
        use super::error::MediaError;
        match error {
            MediaError::UnsupportedCodec { .. } => RecoveryAction::Fatal,
            MediaError::SdpParsingFailed { .. } => RecoveryAction::Fatal,
            MediaError::StreamSyncLost => RecoveryAction::ResetPipeline,
            MediaError::BufferOverflow => RecoveryAction::LogAndContinue,
            MediaError::InvalidMediaFormat { .. } => RecoveryAction::Fatal,
            MediaError::NoCompatibleStreams => RecoveryAction::Fatal,
            MediaError::RtcpError { .. } | MediaError::RtpPacketError { .. } => {
                RecoveryAction::LogAndContinue
            }
        }
    }

    /// Mark the last recovery attempt as successful
    pub fn mark_recovery_successful(&mut self) {
        if let Some(last) = self.recovery_history.last_mut() {
            last.successful = true;
            gst::info!(self.cat, "Recovery successful for error: {}", last.error);
        }
    }

    /// Clear recovery history (useful after successful connection)
    pub fn clear_history(&mut self) {
        self.recovery_history.clear();
        gst::debug!(self.cat, "Recovery history cleared");
    }

    /// Get statistics about recovery attempts
    pub fn get_recovery_stats(&self) -> RecoveryStats {
        let total_attempts = self.recovery_history.len();
        let successful_attempts = self
            .recovery_history
            .iter()
            .filter(|a| a.successful)
            .count();

        RecoveryStats {
            total_attempts,
            successful_attempts,
            success_rate: if total_attempts > 0 {
                (successful_attempts as f64 / total_attempts as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecoveryStats {
    pub total_attempts: usize,
    pub successful_attempts: usize,
    pub success_rate: f64,
}

/// Helper function to execute a recovery action
pub async fn execute_recovery_action(
    action: &RecoveryAction,
    cat: &gst::DebugCategory,
) -> Result<(), RtspError> {
    match action {
        RecoveryAction::Retry {
            strategy,
            max_attempts,
            delay,
        } => {
            gst::info!(
                *cat,
                "Retrying with strategy {:?}, max attempts: {}, delay: {:?}",
                strategy,
                max_attempts,
                delay
            );
            tokio::time::sleep(*delay).await;
            Ok(())
        }
        RecoveryAction::Reconnect {
            reset_session,
            delay,
        } => {
            gst::info!(
                *cat,
                "Reconnecting (reset_session: {}) after {:?}",
                reset_session,
                delay
            );
            tokio::time::sleep(*delay).await;
            Ok(())
        }
        RecoveryAction::FallbackTransport { from, to } => {
            gst::info!(*cat, "Switching transport from {:?} to {:?}", from, to);
            Ok(())
        }
        RecoveryAction::ResetPipeline => {
            gst::warning!(*cat, "Resetting pipeline due to error");
            Ok(())
        }
        RecoveryAction::LogAndContinue => {
            gst::debug!(*cat, "Non-fatal error, continuing");
            Ok(())
        }
        RecoveryAction::Fatal => {
            gst::error!(*cat, "Fatal error, cannot recover");
            Err(RtspError::internal("Fatal error, cannot recover"))
        }
        RecoveryAction::WaitForIntervention { message } => {
            gst::error!(*cat, "User intervention required: {}", message);
            Err(RtspError::internal(format!(
                "User intervention required: {}",
                message
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_error_recovery() {
        let cat = gst::DebugCategory::new(
            "test-recovery",
            gst::DebugColorFlags::empty(),
            Some("Test Recovery"),
        );
        let mut recovery = ErrorRecovery::new(cat);

        let error = RtspError::Network(NetworkError::ConnectionTimeout {
            host: "example.com".to_string(),
            port: 554,
            timeout: Duration::from_secs(10),
        });

        let context = ErrorContext::new()
            .with_resource("rtsp://example.com/stream")
            .with_retry_count(0);

        let action = recovery.determine_recovery_action(&error, &context);
        match action {
            RecoveryAction::Retry { .. } => {}
            _ => panic!("Expected Retry action for connection timeout"),
        }
    }

    #[test]
    fn test_protocol_error_recovery() {
        let cat = gst::DebugCategory::new(
            "test-recovery",
            gst::DebugColorFlags::empty(),
            Some("Test Recovery"),
        );
        let mut recovery = ErrorRecovery::new(cat);

        let error = RtspError::Protocol(ProtocolError::StatusError {
            code: 503,
            message: "Service Unavailable".to_string(),
        });

        let context = ErrorContext::new();

        let action = recovery.determine_recovery_action(&error, &context);
        match action {
            RecoveryAction::Retry { strategy, .. } => {
                assert_eq!(strategy, RetryStrategy::ExponentialJitter);
            }
            _ => panic!("Expected Retry action for 503 error"),
        }
    }

    #[test]
    fn test_recovery_stats() {
        let cat = gst::DebugCategory::new(
            "test-recovery",
            gst::DebugColorFlags::empty(),
            Some("Test Recovery"),
        );
        let mut recovery = ErrorRecovery::new(cat);

        // Simulate some recovery attempts
        recovery.recovery_history.push(RecoveryAttempt {
            error: "Test error 1".to_string(),
            action: RecoveryAction::LogAndContinue,
            timestamp: std::time::Instant::now(),
            successful: true,
        });

        recovery.recovery_history.push(RecoveryAttempt {
            error: "Test error 2".to_string(),
            action: RecoveryAction::Retry {
                strategy: RetryStrategy::Linear,
                max_attempts: 3,
                delay: Duration::from_secs(1),
            },
            timestamp: std::time::Instant::now(),
            successful: false,
        });

        let stats = recovery.get_recovery_stats();
        assert_eq!(stats.total_attempts, 2);
        assert_eq!(stats.successful_attempts, 1);
        assert_eq!(stats.success_rate, 50.0);
    }
}
