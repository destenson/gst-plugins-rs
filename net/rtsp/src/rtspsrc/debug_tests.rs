#![allow(unused)]
// GStreamer RTSP Debug Observability Tests
//
// Tests for verifying debug logging and decision history functionality
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
    use super::super::auto_selector::{AutoRetrySelector, ConnectionAttemptResult};
    use super::super::debug::*;
    use super::super::retry::{RetryCalculator, RetryConfig, RetryStrategy};
    use gst::prelude::*;
    use std::time::{Duration, Instant};

    // Initialize GStreamer for tests
    fn init() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            gst::init().unwrap();
        });
    }

    #[test]
    fn test_debug_categories_initialization() {
        init();

        // Verify all debug categories are properly initialized
        assert_eq!(CAT_RETRY.name(), "rtspsrc2-retry");
        assert_eq!(CAT_AUTO.name(), "rtspsrc2-auto");
        assert_eq!(CAT_ADAPTIVE.name(), "rtspsrc2-adaptive");
        assert_eq!(CAT_RACING.name(), "rtspsrc2-racing");
    }

    #[test]
    fn test_decision_history_buffer() {
        let history = DecisionHistory::new(3);

        // Add multiple decisions
        history.record(
            DecisionType::RetryDelay {
                attempt: 1,
                strategy: "exponential".to_string(),
                delay_ms: 1000,
                reason: "Initial failure".to_string(),
            },
            Some("Test context 1".to_string()),
        );

        history.record(
            DecisionType::PatternDetected {
                pattern: "lossy".to_string(),
                confidence: 0.85,
                evidence: "High packet loss detected".to_string(),
            },
            Some("Test context 2".to_string()),
        );

        history.record(
            DecisionType::StrategyChanged {
                from: "exponential".to_string(),
                to: "immediate".to_string(),
                reason: "Lossy network detected".to_string(),
            },
            None,
        );

        // Add a fourth decision to test buffer overflow
        history.record(
            DecisionType::RacingModeUpdate {
                mode: "first-wins".to_string(),
                reason: "Multiple failures detected".to_string(),
            },
            None,
        );

        // Should only keep last 3 decisions
        let decisions = history.get_history();
        assert_eq!(decisions.len(), 3);

        // Verify JSON serialization
        let json = history.get_history_json();
        assert!(json.contains("\"pattern\": \"lossy\""));
        assert!(json.contains("\"strategy_changed\"") || json.contains("\"StrategyChanged\""));
        assert!(json.contains("\"racing_mode_update\"") || json.contains("\"RacingModeUpdate\""));

        // Verify the oldest decision was dropped
        assert!(!json.contains("\"attempt\": 1"));
    }

    #[test]
    fn test_retry_calculator_with_debug_logging() {
        init();

        let config = RetryConfig {
            strategy: RetryStrategy::Exponential,
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
            linear_step: Duration::from_secs(2),
        };

        let mut calculator = RetryCalculator::new(config);

        // Test that decision history is created
        let history_json = calculator.get_decision_history_json();
        assert_eq!(history_json, "[]");

        // Calculate delays and verify logging happens
        let delay1 = calculator.next_delay();
        assert!(delay1.is_some());

        let delay2 = calculator.next_delay();
        assert!(delay2.is_some());

        // Get history after calculations
        let history_json = calculator.get_decision_history_json();
        assert!(
            history_json.contains("\"retry_delay\"") || history_json.contains("\"RetryDelay\"")
        );
        assert!(history_json.contains("exponential"));
    }

    #[test]
    fn test_auto_selector_debug_logging() {
        init();

        let mut selector = AutoRetrySelector::new();

        // Record some connection attempts
        selector.record_attempt(ConnectionAttemptResult {
            success: false,
            connection_duration: None,
            timestamp: Instant::now(),
            retry_count: 1,
        });

        selector.record_attempt(ConnectionAttemptResult {
            success: false,
            connection_duration: None,
            timestamp: Instant::now(),
            retry_count: 2,
        });

        selector.record_attempt(ConnectionAttemptResult {
            success: false,
            connection_duration: None,
            timestamp: Instant::now(),
            retry_count: 3,
        });

        // After 3 attempts, pattern detection should trigger
        let pattern = selector.get_pattern();
        assert_ne!(format!("{}", pattern), "unknown");
    }

    #[test]
    fn test_verbose_retry_logging_env() {
        // Test environment variable detection
        std::env::set_var("GST_RTSP_VERBOSE_RETRY", "1");
        assert!(is_verbose_retry_logging());

        std::env::set_var("GST_RTSP_VERBOSE_RETRY", "true");
        assert!(is_verbose_retry_logging());

        std::env::set_var("GST_RTSP_VERBOSE_RETRY", "0");
        assert!(!is_verbose_retry_logging());

        std::env::remove_var("GST_RTSP_VERBOSE_RETRY");
        assert!(!is_verbose_retry_logging());
    }

    #[test]
    fn test_format_helpers() {
        let retry_msg = format_retry_decision(
            1,
            "exponential",
            Duration::from_millis(1500),
            "Connection timeout",
        );
        assert!(retry_msg.contains("attempt=1"));
        assert!(retry_msg.contains("strategy=exponential"));
        assert!(retry_msg.contains("1500ms"));
        assert!(retry_msg.contains("Connection timeout"));

        let pattern_msg = format_pattern_detection("lossy", 0.75, "3/4 attempts failed");
        assert!(pattern_msg.contains("type=lossy"));
        assert!(pattern_msg.contains("confidence=0.75"));
        assert!(pattern_msg.contains("3/4 attempts failed"));

        let strategy_msg = format_strategy_change("exponential", "immediate", "High packet loss");
        assert!(strategy_msg.contains("from=exponential"));
        assert!(strategy_msg.contains("to=immediate"));
        assert!(strategy_msg.contains("High packet loss"));

        let racing_msg = format_racing_update("first-wins", "Lossy network detected");
        assert!(racing_msg.contains("mode=first-wins"));
        assert!(racing_msg.contains("Lossy network detected"));

        let adaptive_msg = format_adaptive_learning("linear", 0.92, "exploitation");
        assert!(adaptive_msg.contains("strategy=linear"));
        assert!(adaptive_msg.contains("confidence=0.92"));
        assert!(adaptive_msg.contains("phase=exploitation"));
    }

    #[test]
    fn test_decision_history_clear() {
        let history = DecisionHistory::new(5);

        // Add some decisions
        for i in 0..3 {
            history.record(
                DecisionType::RetryDelay {
                    attempt: i,
                    strategy: "test".to_string(),
                    delay_ms: 100 * (i as u64 + 1),
                    reason: format!("Test {}", i),
                },
                None,
            );
        }

        assert_eq!(history.get_history().len(), 3);

        // Clear history
        history.clear();
        assert_eq!(history.get_history().len(), 0);
        assert_eq!(history.get_history_json(), "[]");
    }

    #[test]
    fn test_connection_result_decision_type() {
        let history = DecisionHistory::new(10);

        history.record(
            DecisionType::ConnectionResult {
                success: true,
                duration_ms: Some(5000),
                retry_count: 2,
            },
            Some("Successful connection after 2 retries".to_string()),
        );

        let json = history.get_history_json();
        assert!(json.contains("\"success\": true"));
        assert!(json.contains("\"duration_ms\": 5000"));
        assert!(json.contains("\"retry_count\": 2"));
    }

    #[test]
    fn test_adaptive_learning_decision_type() {
        let history = DecisionHistory::new(10);

        history.record(
            DecisionType::AdaptiveLearning {
                strategy: "exponential-jitter".to_string(),
                confidence: 0.87,
                phase: "discovery".to_string(),
            },
            Some("Learning phase".to_string()),
        );

        let json = history.get_history_json();
        assert!(json.contains("exponential-jitter"));
        assert!(json.contains("0.87"));
        assert!(json.contains("discovery"));
    }
}
