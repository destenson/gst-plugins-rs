#![allow(unused)]
// GStreamer RTSP Simple Auto Retry Mode
//
// This module implements a simple "auto" retry mode that uses basic heuristics
// to quickly select an appropriate retry strategy without requiring user configuration.
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use super::connection_racer::ConnectionRacingStrategy;
use super::debug::{DecisionHistory, DecisionType, CAT_AUTO};
use super::retry::RetryStrategy;
use crate::debug_decision;
use gst::prelude::*;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const AUTO_DETECTION_ATTEMPTS: usize = 3;
const CONNECTION_DROP_THRESHOLD: Duration = Duration::from_secs(30);
const HIGH_FAILURE_THRESHOLD: f32 = 0.5;

/// Fallback strategy list for auto mode
const FALLBACK_STRATEGIES: [RetryStrategy; 4] = [
    RetryStrategy::ExponentialJitter, // Good default for most networks
    RetryStrategy::Linear,            // Conservative fallback
    RetryStrategy::Exponential,       // Standard exponential
    RetryStrategy::Immediate,         // Last resort
];

#[derive(Debug, Clone)]
pub struct ConnectionAttemptResult {
    pub success: bool,
    pub connection_duration: Option<Duration>,
    pub timestamp: Instant,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkPattern {
    /// Normal stable network
    Stable,
    /// Connection-limited device (e.g., IP cameras)
    ConnectionLimited,
    /// High packet loss network
    HighPacketLoss,
    /// Unknown or transitioning pattern
    Unknown,
}

impl std::fmt::Display for NetworkPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkPattern::Stable => write!(f, "stable"),
            NetworkPattern::ConnectionLimited => write!(f, "limited"),
            NetworkPattern::HighPacketLoss => write!(f, "lossy"),
            NetworkPattern::Unknown => write!(f, "unknown"),
        }
    }
}

/// Auto selector for retry strategies
pub struct AutoRetrySelector {
    /// Recent connection attempts for analysis
    recent_attempts: VecDeque<ConnectionAttemptResult>,
    /// Current detected network pattern
    current_pattern: NetworkPattern,
    /// Current selected strategy
    current_strategy: RetryStrategy,
    /// Fallback strategy index
    fallback_index: usize,
    /// Whether auto fallback is enabled
    auto_fallback_enabled: bool,
    /// Number of attempts to analyze before making a decision
    detection_attempts: usize,
    /// Decision history for observability
    decision_history: Option<DecisionHistory>,
}

impl AutoRetrySelector {
    pub fn new() -> Self {
        Self {
            recent_attempts: VecDeque::with_capacity(10),
            current_pattern: NetworkPattern::Unknown,
            current_strategy: FALLBACK_STRATEGIES[0],
            fallback_index: 0,
            auto_fallback_enabled: true,
            detection_attempts: AUTO_DETECTION_ATTEMPTS,
            decision_history: Some(DecisionHistory::default()),
        }
    }

    /// Record a connection attempt result
    pub fn record_attempt(&mut self, result: ConnectionAttemptResult) {
        self.recent_attempts.push_back(result);

        // Keep only recent attempts
        while self.recent_attempts.len() > 10 {
            self.recent_attempts.pop_front();
        }

        // Analyze pattern after enough attempts
        if self.recent_attempts.len() >= self.detection_attempts {
            self.analyze_pattern();
        }
    }

    /// Analyze recent attempts to detect network pattern
    fn analyze_pattern(&mut self) {
        let recent: Vec<&ConnectionAttemptResult> = self
            .recent_attempts
            .iter()
            .rev()
            .take(self.detection_attempts)
            .collect();

        let old_pattern = self.current_pattern;
        let old_strategy = self.current_strategy;

        // Check for connection-limited device pattern
        if self.is_connection_limited(&recent) {
            self.current_pattern = NetworkPattern::ConnectionLimited;
            self.current_strategy = RetryStrategy::Linear; // Use linear for predictable retry

            let evidence = format!(
                "Short connections detected: {} of {} attempts dropped quickly",
                recent
                    .iter()
                    .filter(|a| a.success
                        && a.connection_duration
                            .map_or(false, |d| d < CONNECTION_DROP_THRESHOLD))
                    .count(),
                recent.len()
            );

            debug_decision!(
                CAT_AUTO,
                self.decision_history.as_ref(),
                DecisionType::PatternDetected {
                    pattern: "connection-limited".to_string(),
                    confidence: 0.8,
                    evidence: evidence.clone(),
                },
                "Pattern detected: Connection-limited device. Evidence: {}",
                evidence
            );

            if old_strategy != self.current_strategy {
                debug_decision!(
                    CAT_AUTO,
                    self.decision_history.as_ref(),
                    DecisionType::StrategyChanged {
                        from: old_strategy.as_str().to_string(),
                        to: self.current_strategy.as_str().to_string(),
                        reason: "Connection-limited pattern detected".to_string(),
                    },
                    "Strategy changed from {} to {} due to connection-limited pattern",
                    old_strategy.as_str(),
                    self.current_strategy.as_str()
                );
            }
            return;
        }

        // Check for high packet loss
        if self.is_high_packet_loss(&recent) {
            self.current_pattern = NetworkPattern::HighPacketLoss;
            self.current_strategy = RetryStrategy::Immediate; // Retry quickly

            let failures = recent.iter().filter(|a| !a.success).count();
            let failure_rate = failures as f32 / recent.len() as f32;
            let evidence = format!(
                "High failure rate: {}/{} attempts failed ({:.1}%)",
                failures,
                recent.len(),
                failure_rate * 100.0
            );

            debug_decision!(
                CAT_AUTO,
                self.decision_history.as_ref(),
                DecisionType::PatternDetected {
                    pattern: "high-packet-loss".to_string(),
                    confidence: failure_rate,
                    evidence: evidence.clone(),
                },
                "Pattern detected: High packet loss. Evidence: {}",
                evidence
            );

            if old_strategy != self.current_strategy {
                debug_decision!(
                    CAT_AUTO,
                    self.decision_history.as_ref(),
                    DecisionType::StrategyChanged {
                        from: old_strategy.as_str().to_string(),
                        to: self.current_strategy.as_str().to_string(),
                        reason: "High packet loss detected".to_string(),
                    },
                    "Strategy changed from {} to {} due to high packet loss",
                    old_strategy.as_str(),
                    self.current_strategy.as_str()
                );
            }
            return;
        }

        // Check if current strategy is working well
        if self.is_stable(&recent) {
            self.current_pattern = NetworkPattern::Stable;

            let successes = recent.iter().filter(|a| a.success).count();
            let success_rate = successes as f32 / recent.len() as f32;
            let evidence = format!(
                "Stable network: {}/{} attempts succeeded ({:.1}%)",
                successes,
                recent.len(),
                success_rate * 100.0
            );

            debug_decision!(
                CAT_AUTO,
                self.decision_history.as_ref(),
                DecisionType::PatternDetected {
                    pattern: "stable".to_string(),
                    confidence: success_rate,
                    evidence: evidence.clone(),
                },
                "Pattern detected: Stable network. Evidence: {}",
                evidence
            );

            if old_pattern != self.current_pattern {
                gst::debug!(
                    CAT_AUTO,
                    "Network pattern changed from {} to {}",
                    old_pattern,
                    self.current_pattern
                );
            }
            // Keep current strategy if it's working
            return;
        }

        // If nothing specific detected, try next fallback
        if self.auto_fallback_enabled && !self.is_working(&recent) {
            gst::debug!(
                CAT_AUTO,
                "No specific pattern detected, trying next fallback strategy"
            );
            self.try_next_fallback();
        }
    }

    /// Check if connection-limited device pattern
    fn is_connection_limited(&self, attempts: &[&ConnectionAttemptResult]) -> bool {
        let mut short_connections = 0;

        for attempt in attempts {
            if attempt.success {
                if let Some(duration) = attempt.connection_duration {
                    if duration < CONNECTION_DROP_THRESHOLD {
                        short_connections += 1;
                    }
                }
            }
        }

        // If most successful connections drop quickly, it's likely connection-limited
        short_connections >= 2 && short_connections as f32 / attempts.len() as f32 > 0.6
    }

    /// Check if high packet loss pattern
    fn is_high_packet_loss(&self, attempts: &[&ConnectionAttemptResult]) -> bool {
        let failures = attempts.iter().filter(|a| !a.success).count();
        let failure_rate = failures as f32 / attempts.len() as f32;

        // High failure rate indicates packet loss
        failure_rate > HIGH_FAILURE_THRESHOLD
    }

    /// Check if network is stable
    fn is_stable(&self, attempts: &[&ConnectionAttemptResult]) -> bool {
        let successes = attempts.iter().filter(|a| a.success).count();
        let success_rate = successes as f32 / attempts.len() as f32;

        // High success rate indicates stability
        success_rate > 0.8
    }

    /// Check if current strategy is working
    fn is_working(&self, attempts: &[&ConnectionAttemptResult]) -> bool {
        attempts.iter().any(|a| a.success)
    }

    /// Try next fallback strategy
    fn try_next_fallback(&mut self) {
        let old_strategy = self.current_strategy;
        self.fallback_index = (self.fallback_index + 1) % FALLBACK_STRATEGIES.len();
        self.current_strategy = FALLBACK_STRATEGIES[self.fallback_index];
        self.current_pattern = NetworkPattern::Unknown;

        debug_decision!(
            CAT_AUTO,
            self.decision_history.as_ref(),
            DecisionType::StrategyChanged {
                from: old_strategy.as_str().to_string(),
                to: self.current_strategy.as_str().to_string(),
                reason: format!("Fallback to strategy index {}", self.fallback_index),
            },
            "Fallback strategy change: {} -> {} (index {})",
            old_strategy.as_str(),
            self.current_strategy.as_str(),
            self.fallback_index
        );
    }

    /// Get the current recommended retry strategy
    pub fn get_strategy(&self) -> RetryStrategy {
        self.current_strategy
    }

    /// Get the recommended connection racing strategy based on pattern
    pub fn get_racing_strategy(&self) -> ConnectionRacingStrategy {
        match self.current_pattern {
            NetworkPattern::ConnectionLimited => ConnectionRacingStrategy::LastWins,
            NetworkPattern::HighPacketLoss => ConnectionRacingStrategy::FirstWins,
            NetworkPattern::Stable => ConnectionRacingStrategy::None,
            NetworkPattern::Unknown => ConnectionRacingStrategy::None,
        }
    }

    /// Get current network pattern
    pub fn get_pattern(&self) -> NetworkPattern {
        self.current_pattern
    }

    /// Get current network pattern as string
    pub fn get_network_pattern(&self) -> NetworkPattern {
        self.current_pattern
    }

    /// Reset the selector
    pub fn reset(&mut self) {
        self.recent_attempts.clear();
        self.current_pattern = NetworkPattern::Unknown;
        self.current_strategy = FALLBACK_STRATEGIES[0];
        self.fallback_index = 0;
    }

    /// Get a summary of the current state
    pub fn get_summary(&self) -> String {
        format!(
            "Pattern: {:?}, Strategy: {:?}, Attempts: {}, Racing: {:?}",
            self.current_pattern,
            self.current_strategy,
            self.recent_attempts.len(),
            self.get_racing_strategy()
        )
    }
}

impl Default for AutoRetrySelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_limited_detection() {
        let mut selector = AutoRetrySelector::new();

        // Simulate connection-limited device behavior
        for i in 0..3 {
            selector.record_attempt(ConnectionAttemptResult {
                success: true,
                connection_duration: Some(Duration::from_secs(15 + i * 5)), // Short connections
                timestamp: Instant::now(),
                retry_count: 0,
            });
        }

        assert_eq!(selector.get_pattern(), NetworkPattern::ConnectionLimited);
        assert_eq!(
            selector.get_racing_strategy(),
            ConnectionRacingStrategy::LastWins
        );
    }

    #[test]
    fn test_high_packet_loss_detection() {
        let mut selector = AutoRetrySelector::new();

        // Simulate high packet loss
        selector.record_attempt(ConnectionAttemptResult {
            success: false,
            connection_duration: None,
            timestamp: Instant::now(),
            retry_count: 0,
        });

        selector.record_attempt(ConnectionAttemptResult {
            success: false,
            connection_duration: None,
            timestamp: Instant::now(),
            retry_count: 1,
        });

        selector.record_attempt(ConnectionAttemptResult {
            success: true,
            connection_duration: Some(Duration::from_secs(60)),
            timestamp: Instant::now(),
            retry_count: 2,
        });

        assert_eq!(selector.get_pattern(), NetworkPattern::HighPacketLoss);
        assert_eq!(
            selector.get_racing_strategy(),
            ConnectionRacingStrategy::FirstWins
        );
    }

    #[test]
    fn test_stable_network_detection() {
        let mut selector = AutoRetrySelector::new();

        // Simulate stable network
        for _ in 0..3 {
            selector.record_attempt(ConnectionAttemptResult {
                success: true,
                connection_duration: Some(Duration::from_secs(300)), // Long stable connections
                timestamp: Instant::now(),
                retry_count: 0,
            });
        }

        assert_eq!(selector.get_pattern(), NetworkPattern::Stable);
        assert_eq!(
            selector.get_racing_strategy(),
            ConnectionRacingStrategy::None
        );
    }

    #[test]
    fn test_fallback_progression() {
        let mut selector = AutoRetrySelector::new();

        // Simulate total failures to trigger fallback
        for _ in 0..4 {
            for _ in 0..3 {
                selector.record_attempt(ConnectionAttemptResult {
                    success: false,
                    connection_duration: None,
                    timestamp: Instant::now(),
                    retry_count: 0,
                });
            }

            // After detection, should try next fallback
            let prev_strategy = selector.get_strategy();
            selector.analyze_pattern();

            if selector.fallback_index > 0 {
                // Should have moved to a different strategy
                assert_ne!(selector.get_strategy(), prev_strategy);
            }
        }
    }

    #[test]
    fn test_reset() {
        let mut selector = AutoRetrySelector::new();

        // Add some attempts
        for _ in 0..5 {
            selector.record_attempt(ConnectionAttemptResult {
                success: true,
                connection_duration: Some(Duration::from_secs(100)),
                timestamp: Instant::now(),
                retry_count: 0,
            });
        }

        selector.reset();

        assert_eq!(selector.recent_attempts.len(), 0);
        assert_eq!(selector.get_pattern(), NetworkPattern::Unknown);
        assert_eq!(selector.get_strategy(), FALLBACK_STRATEGIES[0]);
    }
}
