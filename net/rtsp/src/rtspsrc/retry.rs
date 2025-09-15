#![allow(unused)]
// GStreamer RTSP plugin retry logic implementation
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use super::debug::{DecisionHistory, DecisionType, CAT_RETRY};
use crate::debug_decision;
use gst::prelude::*;
use rand::Rng;
use std::time::{Duration, Instant};

#[cfg(feature = "adaptive")]
use super::adaptive_retry::{AdaptiveRetryConfig, AdaptiveRetryManager};
use super::auto_selector::{AutoRetrySelector, ConnectionAttemptResult};
use super::connection_racer::ConnectionRacingStrategy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    Auto,
    #[cfg(feature = "adaptive")]
    Adaptive,
    None,
    Immediate,
    Linear,
    Exponential,
    ExponentialJitter,
    FirstWins, // Connection racing strategy integrated with retry
    LastWins,  // Connection racing strategy integrated with retry
}

impl Default for RetryStrategy {
    fn default() -> Self {
        RetryStrategy::Auto
    }
}

impl RetryStrategy {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "auto" => RetryStrategy::Auto,
            #[cfg(feature = "adaptive")]
            "adaptive" => RetryStrategy::Adaptive,
            "none" => RetryStrategy::None,
            "immediate" => RetryStrategy::Immediate,
            "linear" => RetryStrategy::Linear,
            "exponential" => RetryStrategy::Exponential,
            "exponential-jitter" => RetryStrategy::ExponentialJitter,
            "first-wins" => RetryStrategy::FirstWins,
            "last-wins" => RetryStrategy::LastWins,
            _ => RetryStrategy::Auto,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RetryStrategy::Auto => "auto",
            #[cfg(feature = "adaptive")]
            RetryStrategy::Adaptive => "adaptive",
            RetryStrategy::None => "none",
            RetryStrategy::Immediate => "immediate",
            RetryStrategy::Linear => "linear",
            RetryStrategy::Exponential => "exponential",
            RetryStrategy::ExponentialJitter => "exponential-jitter",
            RetryStrategy::FirstWins => "first-wins",
            RetryStrategy::LastWins => "last-wins",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub strategy: RetryStrategy,
    pub max_attempts: i32, // -1 for infinite
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub linear_step: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            strategy: RetryStrategy::Auto,
            max_attempts: 5,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            linear_step: Duration::from_secs(2),
        }
    }
}

pub struct RetryCalculator {
    config: RetryConfig,
    attempt: u32,
    #[cfg(feature = "adaptive")]
    adaptive_manager: Option<AdaptiveRetryManager>,
    auto_selector: Option<AutoRetrySelector>,
    last_attempt_time: Option<Instant>,
    last_connection_start: Option<Instant>,
    server_url: Option<String>,
    #[cfg(feature = "telemetry")]
    telemetry: Option<super::telemetry::RtspMetrics>,
    first_failure_time: Option<Instant>,
    decision_history: Option<DecisionHistory>,
}

impl RetryCalculator {
    pub fn new(config: RetryConfig) -> Self {
        let auto_selector = if config.strategy == RetryStrategy::Auto {
            Some(AutoRetrySelector::new())
        } else {
            None
        };

        Self {
            config,
            attempt: 0,
            #[cfg(feature = "adaptive")]
            adaptive_manager: None,
            auto_selector,
            last_attempt_time: None,
            last_connection_start: None,
            server_url: None,
            #[cfg(feature = "telemetry")]
            telemetry: None,
            first_failure_time: None,
            decision_history: Some(DecisionHistory::default()),
        }
    }

    pub fn with_server_url(mut self, url: &str) -> Self {
        self.server_url = Some(url.to_string());
        #[cfg(feature = "adaptive")]
        if self.config.strategy == RetryStrategy::Adaptive {
            let adaptive_config = AdaptiveRetryConfig::default();
            self.adaptive_manager = Some(AdaptiveRetryManager::new(url, adaptive_config));
        }
        self
    }

    #[cfg(feature = "telemetry")]
    pub fn with_telemetry(mut self, telemetry: super::telemetry::RtspMetrics) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    pub fn should_retry(&self) -> bool {
        if self.config.strategy == RetryStrategy::None {
            gst::debug!(CAT_RETRY, "Retry disabled by strategy=None");
            return false;
        }

        let should_retry = if self.config.max_attempts < 0 {
            // Infinite retries
            true
        } else {
            self.attempt < self.config.max_attempts as u32
        };

        gst::debug!(
            CAT_RETRY,
            "Should retry check: attempt={}/{}, will_retry={}",
            self.attempt,
            if self.config.max_attempts < 0 {
                "infinite".to_string()
            } else {
                self.config.max_attempts.to_string()
            },
            should_retry
        );

        should_retry
    }

    pub fn next_delay(&mut self) -> Option<Duration> {
        if !self.should_retry() {
            return None;
        }

        let current_strategy = if self.config.strategy == RetryStrategy::Auto {
            let strategy = self
                .auto_selector
                .as_ref()
                .map(|s| s.get_strategy())
                .unwrap_or(self.config.strategy);
            gst::debug!(
                CAT_RETRY,
                "Auto mode selected strategy: {}",
                strategy.as_str()
            );
            strategy
        } else {
            self.config.strategy
        };

        let (delay, reason) = match current_strategy {
            RetryStrategy::None => return None,
            RetryStrategy::Immediate => (Duration::ZERO, "immediate retry"),
            RetryStrategy::Linear => (self.calculate_linear_delay(), "linear backoff"),
            RetryStrategy::Exponential => (
                self.calculate_exponential_delay(false),
                "exponential backoff",
            ),
            RetryStrategy::ExponentialJitter => (
                self.calculate_exponential_delay(true),
                "exponential with jitter",
            ),
            RetryStrategy::FirstWins | RetryStrategy::LastWins => {
                (Duration::from_millis(250), "racing mode")
            }
            RetryStrategy::Auto => (self.calculate_auto_delay(), "auto-calculated delay"),
            #[cfg(feature = "adaptive")]
            RetryStrategy::Adaptive => (self.calculate_adaptive_delay(), "adaptive learning"),
        };

        // Cap at max_delay
        let final_delay = delay.min(self.config.max_delay);

        // Log the retry decision
        debug_decision!(
            CAT_RETRY,
            self.decision_history.as_ref(),
            DecisionType::RetryDelay {
                attempt: self.attempt,
                strategy: current_strategy.as_str().to_string(),
                delay_ms: final_delay.as_millis() as u64,
                reason: reason.to_string(),
            },
            "Retry attempt {} using {} strategy: delay={}ms ({})",
            self.attempt + 1,
            current_strategy.as_str(),
            final_delay.as_millis(),
            reason
        );

        self.attempt += 1;
        self.last_attempt_time = Some(Instant::now());

        // Record retry in telemetry
        #[cfg(feature = "telemetry")]
        if let Some(ref telemetry) = self.telemetry {
            telemetry.record_retry(current_strategy.as_str());
        }

        Some(final_delay)
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
        self.last_attempt_time = None;
        self.last_connection_start = None;
        self.first_failure_time = None;
        if let Some(ref mut selector) = self.auto_selector {
            selector.reset();
        }
    }

    pub fn current_attempt(&self) -> u32 {
        self.attempt
    }

    /// Mark the start of a connection attempt
    pub fn mark_connection_start(&mut self) {
        self.last_connection_start = Some(Instant::now());
    }

    /// Record the result of a connection attempt for auto mode
    pub fn record_connection_result(&mut self, success: bool, connection_dropped: bool) {
        // Track first failure time for recovery metrics
        if !success && self.first_failure_time.is_none() {
            self.first_failure_time = Some(Instant::now());
        }

        // If successful, record recovery time
        if success {
            #[cfg(feature = "telemetry")]
            if let Some(ref telemetry) = self.telemetry {
                if let Some(first_failure) = self.first_failure_time {
                    let recovery_time_ms = first_failure.elapsed().as_millis() as u64;
                    telemetry.record_connection_recovery(recovery_time_ms);
                }
            }
            self.first_failure_time = None;
        }

        // Log connection result
        let duration_ms = self
            .last_connection_start
            .map(|start| start.elapsed().as_millis() as u64);
        debug_decision!(
            CAT_RETRY,
            self.decision_history.as_ref(),
            DecisionType::ConnectionResult {
                success,
                duration_ms,
                retry_count: self.attempt,
            },
            "Connection result: success={}, dropped={}, duration={}ms, retry_count={}",
            success,
            connection_dropped,
            duration_ms.map_or("N/A".to_string(), |d| d.to_string()),
            self.attempt
        );

        if let Some(ref mut selector) = self.auto_selector {
            let connection_duration = if success && !connection_dropped {
                // Connection is still alive
                self.last_connection_start.map(|start| start.elapsed())
            } else if success && connection_dropped {
                // Connection succeeded but then dropped
                self.last_connection_start.map(|start| start.elapsed())
            } else {
                // Connection failed
                None
            };

            let old_strategy = selector.get_strategy();
            selector.record_attempt(ConnectionAttemptResult {
                success,
                connection_duration,
                timestamp: Instant::now(),
                retry_count: self.attempt,
            });
            let new_strategy = selector.get_strategy();

            // Record strategy change in telemetry
            #[cfg(feature = "telemetry")]
            if old_strategy != new_strategy {
                if let Some(ref telemetry) = self.telemetry {
                    telemetry.record_retry_strategy_change(
                        old_strategy.as_str(),
                        new_strategy.as_str(),
                        Some("auto-mode"),
                    );
                }
            }

            // Record auto mode pattern
            #[cfg(feature = "telemetry")]
            if let Some(ref telemetry) = self.telemetry {
                let pattern = selector.get_network_pattern();
                telemetry.record_auto_mode_pattern(&pattern.to_string());
            }
        }

        // Record adaptive confidence if using adaptive strategy
        #[cfg(all(feature = "adaptive", feature = "telemetry"))]
        if let Some(ref manager) = self.adaptive_manager {
            if let Some(ref telemetry) = self.telemetry {
                let confidence = manager.get_confidence_score();
                telemetry.record_adaptive_confidence(confidence);
            }
        }
    }

    /// Get the recommended connection racing strategy from auto mode
    pub fn get_racing_strategy(&self) -> Option<ConnectionRacingStrategy> {
        self.auto_selector.as_ref().map(|s| s.get_racing_strategy())
    }

    /// Get auto mode status summary
    pub fn get_auto_summary(&self) -> Option<String> {
        self.auto_selector.as_ref().map(|s| s.get_summary())
    }

    fn calculate_linear_delay(&self) -> Duration {
        self.config.initial_delay + self.config.linear_step * self.attempt
    }

    fn calculate_exponential_delay(&self, with_jitter: bool) -> Duration {
        let base_delay = self.config.initial_delay * 2u32.pow(self.attempt);

        if with_jitter {
            // Add ±25% jitter
            let mut rng = rand::rng();
            let jitter_factor = rng.random_range(0.75..1.25);
            Duration::from_secs_f64(base_delay.as_secs_f64() * jitter_factor)
        } else {
            base_delay
        }
    }

    fn calculate_auto_delay(&mut self) -> Duration {
        if let Some(ref selector) = self.auto_selector {
            // Use the auto-selected strategy
            match selector.get_strategy() {
                RetryStrategy::Immediate => Duration::ZERO,
                RetryStrategy::Linear => self.calculate_linear_delay(),
                RetryStrategy::Exponential => self.calculate_exponential_delay(false),
                RetryStrategy::ExponentialJitter => self.calculate_exponential_delay(true),
                RetryStrategy::FirstWins | RetryStrategy::LastWins => {
                    // For racing strategies, use minimal delay
                    Duration::from_millis(250)
                }
                _ => self.calculate_exponential_delay(true),
            }
        } else {
            // Fallback to exponential with jitter
            self.calculate_exponential_delay(true)
        }
    }

    #[cfg(feature = "adaptive")]
    fn calculate_adaptive_delay(&mut self) -> Duration {
        if let Some(ref mut manager) = self.adaptive_manager {
            let strategy = manager.select_strategy();

            // Convert adaptive strategy to retry strategy and calculate delay
            match strategy {
                super::adaptive_retry::Strategy::Immediate => Duration::ZERO,
                super::adaptive_retry::Strategy::Linear => self.calculate_linear_delay(),
                super::adaptive_retry::Strategy::Exponential => {
                    self.calculate_exponential_delay(false)
                }
                super::adaptive_retry::Strategy::ExponentialJitter => {
                    self.calculate_exponential_delay(true)
                }
            }
        } else {
            // Fall back to exponential with jitter if adaptive manager not initialized
            self.calculate_exponential_delay(true)
        }
    }

    #[cfg(feature = "adaptive")]
    pub fn record_attempt_result(&mut self, success: bool) {
        if let Some(ref manager) = self.adaptive_manager {
            if let Some(start_time) = self.last_attempt_time {
                let recovery_time = start_time.elapsed();

                // Get the last selected strategy from the manager
                let strategy = manager.get_best_strategy();
                manager.record_attempt(strategy, success, recovery_time);
            }
        }
    }

    #[cfg(feature = "adaptive")]
    pub fn get_adaptive_stats(&self) -> Option<String> {
        self.adaptive_manager
            .as_ref()
            .map(|m| m.get_stats_summary())
    }

    /// Get the decision history as JSON
    pub fn get_decision_history_json(&self) -> String {
        self.decision_history
            .as_ref()
            .map(|h| h.get_history_json())
            .unwrap_or_else(|| "[]".to_string())
    }

    /// Clear the decision history
    pub fn clear_decision_history(&mut self) {
        if let Some(ref history) = self.decision_history {
            history.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_from_string() {
        assert_eq!(RetryStrategy::from_string("none"), RetryStrategy::None);
        assert_eq!(
            RetryStrategy::from_string("immediate"),
            RetryStrategy::Immediate
        );
        assert_eq!(RetryStrategy::from_string("linear"), RetryStrategy::Linear);
        assert_eq!(
            RetryStrategy::from_string("exponential"),
            RetryStrategy::Exponential
        );
        assert_eq!(
            RetryStrategy::from_string("exponential-jitter"),
            RetryStrategy::ExponentialJitter
        );
        assert_eq!(RetryStrategy::from_string("auto"), RetryStrategy::Auto);
        assert_eq!(
            RetryStrategy::from_string("first-wins"),
            RetryStrategy::FirstWins
        );
        assert_eq!(
            RetryStrategy::from_string("last-wins"),
            RetryStrategy::LastWins
        );
        #[cfg(feature = "adaptive")]
        assert_eq!(
            RetryStrategy::from_string("adaptive"),
            RetryStrategy::Adaptive
        );
        assert_eq!(RetryStrategy::from_string("invalid"), RetryStrategy::Auto);
    }

    #[test]
    fn test_no_retry_strategy() {
        let config = RetryConfig {
            strategy: RetryStrategy::None,
            ..Default::default()
        };

        let mut calc = RetryCalculator::new(config);
        assert!(!calc.should_retry());
        assert_eq!(calc.next_delay(), None);
    }

    #[test]
    fn test_immediate_retry() {
        let config = RetryConfig {
            strategy: RetryStrategy::Immediate,
            max_attempts: 3,
            ..Default::default()
        };

        let mut calc = RetryCalculator::new(config);

        for _ in 0..3 {
            assert!(calc.should_retry());
            assert_eq!(calc.next_delay(), Some(Duration::ZERO));
        }

        assert!(!calc.should_retry());
        assert_eq!(calc.next_delay(), None);
    }

    #[test]
    fn test_linear_backoff() {
        let config = RetryConfig {
            strategy: RetryStrategy::Linear,
            max_attempts: 4,
            initial_delay: Duration::from_secs(1),
            linear_step: Duration::from_secs(2),
            max_delay: Duration::from_secs(10),
        };

        let mut calc = RetryCalculator::new(config);

        assert_eq!(calc.next_delay(), Some(Duration::from_secs(1))); // 1s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(3))); // 1s + 2s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(5))); // 1s + 4s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(7))); // 1s + 6s
        assert_eq!(calc.next_delay(), None);
    }

    #[test]
    fn test_exponential_backoff() {
        let config = RetryConfig {
            strategy: RetryStrategy::Exponential,
            max_attempts: 5,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(20),
            ..Default::default()
        };

        let mut calc = RetryCalculator::new(config);

        assert_eq!(calc.next_delay(), Some(Duration::from_secs(1))); // 1s * 2^0
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(2))); // 1s * 2^1
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(4))); // 1s * 2^2
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(8))); // 1s * 2^3
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(16))); // 1s * 2^4
        assert_eq!(calc.next_delay(), None);
    }

    #[test]
    fn test_max_delay_cap() {
        let config = RetryConfig {
            strategy: RetryStrategy::Exponential,
            max_attempts: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            ..Default::default()
        };

        let mut calc = RetryCalculator::new(config);

        assert_eq!(calc.next_delay(), Some(Duration::from_secs(1))); // 1s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(2))); // 2s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(4))); // 4s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(5))); // Capped at 5s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(5))); // Still capped
    }

    #[test]
    fn test_infinite_retries() {
        let config = RetryConfig {
            strategy: RetryStrategy::Immediate,
            max_attempts: -1, // Infinite
            ..Default::default()
        };

        let mut calc = RetryCalculator::new(config);

        // Should always return true for should_retry
        for _ in 0..100 {
            assert!(calc.should_retry());
            assert_eq!(calc.next_delay(), Some(Duration::ZERO));
        }
    }

    #[test]
    fn test_reset() {
        let config = RetryConfig {
            strategy: RetryStrategy::Linear,
            max_attempts: 5,
            ..Default::default()
        };

        let mut calc = RetryCalculator::new(config);

        calc.next_delay();
        calc.next_delay();
        assert_eq!(calc.current_attempt(), 2);

        calc.reset();
        assert_eq!(calc.current_attempt(), 0);
        assert!(calc.should_retry());
    }

    #[test]
    fn test_exponential_jitter() {
        let config = RetryConfig {
            strategy: RetryStrategy::ExponentialJitter,
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(100),
            ..Default::default()
        };

        let mut calc = RetryCalculator::new(config);

        // First delay should be around 1s ± 25%
        let delay1 = calc.next_delay().unwrap();
        assert!(delay1 >= Duration::from_millis(750));
        assert!(delay1 <= Duration::from_millis(1250));

        // Second delay should be around 2s ± 25%
        let delay2 = calc.next_delay().unwrap();
        assert!(delay2 >= Duration::from_millis(1500));
        assert!(delay2 <= Duration::from_millis(2500));
    }
}
