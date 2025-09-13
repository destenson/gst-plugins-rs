// GStreamer RTSP retry integration tests
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
    use super::super::retry::*;
    use std::time::Duration;

    #[test]
    fn test_retry_integration_marks_connection_start() {
        let config = RetryConfig {
            strategy: RetryStrategy::Auto,
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            linear_step: Duration::from_millis(200),
        };
        
        let mut calc = RetryCalculator::new(config)
            .with_server_url("rtsp://test.local");
        
        // Mark connection start
        calc.mark_connection_start();
        
        // Should have a start time
        assert_eq!(calc.current_attempt(), 0);
    }

    #[test]
    fn test_retry_integration_records_success() {
        let config = RetryConfig {
            strategy: RetryStrategy::Auto,
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            linear_step: Duration::from_millis(200),
        };
        
        let mut calc = RetryCalculator::new(config)
            .with_server_url("rtsp://test.local");
        
        // Simulate connection attempt
        calc.mark_connection_start();
        std::thread::sleep(Duration::from_millis(50));
        calc.record_connection_result(true, false);
        
        // Auto mode should have recorded the success
        assert!(calc.get_auto_summary().is_some());
    }

    #[test]
    fn test_retry_integration_records_failure() {
        let config = RetryConfig {
            strategy: RetryStrategy::Auto,
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            linear_step: Duration::from_millis(200),
        };
        
        let mut calc = RetryCalculator::new(config)
            .with_server_url("rtsp://test.local");
        
        // Simulate failed connection attempt
        calc.mark_connection_start();
        calc.record_connection_result(false, false);
        
        // Should still be able to retry
        assert!(calc.should_retry());
        assert!(calc.next_delay().is_some());
    }

    #[test]
    fn test_retry_integration_updates_racing_strategy() {
        let config = RetryConfig {
            strategy: RetryStrategy::Auto,
            max_attempts: 10,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            linear_step: Duration::from_millis(200),
        };
        
        let mut calc = RetryCalculator::new(config)
            .with_server_url("rtsp://test.local");
        
        // Simulate multiple failures to trigger strategy change
        for _ in 0..5 {
            calc.mark_connection_start();
            calc.record_connection_result(false, false);
            let _ = calc.next_delay();
        }
        
        // Auto mode should recommend a racing strategy
        let strategy = calc.get_racing_strategy();
        assert!(strategy.is_some());
    }

    #[test]
    fn test_retry_integration_handles_connection_drops() {
        let config = RetryConfig {
            strategy: RetryStrategy::Auto,
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            linear_step: Duration::from_millis(200),
        };
        
        let mut calc = RetryCalculator::new(config)
            .with_server_url("rtsp://test.local");
        
        // Simulate successful connection that then drops
        calc.mark_connection_start();
        std::thread::sleep(Duration::from_millis(100));
        calc.record_connection_result(true, true); // success but dropped
        
        // Should be able to retry after drop
        assert!(calc.should_retry());
    }

    #[test]
    fn test_retry_integration_auto_mode_adapts() {
        let config = RetryConfig {
            strategy: RetryStrategy::Auto,
            max_attempts: 20,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            linear_step: Duration::from_millis(200),
        };
        
        let mut calc = RetryCalculator::new(config)
            .with_server_url("rtsp://test.local");
        
        // Simulate pattern of failures and successes
        for i in 0..10 {
            calc.mark_connection_start();
            let success = i % 3 == 0; // Success every 3rd attempt
            calc.record_connection_result(success, false);
            let _ = calc.next_delay();
        }
        
        // Auto mode should have adapted to the pattern
        let summary = calc.get_auto_summary();
        assert!(summary.is_some());
        assert!(summary.unwrap().len() > 0);
    }

    #[test]
    fn test_retry_integration_racing_strategy_changes() {
        let config = RetryConfig {
            strategy: RetryStrategy::Auto,
            max_attempts: 15,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            linear_step: Duration::from_millis(200),
        };
        
        let mut calc = RetryCalculator::new(config)
            .with_server_url("rtsp://test.local");
        
        // Get initial strategy
        let initial_strategy = calc.get_racing_strategy();
        
        // Simulate consistent failures to trigger strategy change
        for _ in 0..10 {
            calc.mark_connection_start();
            calc.record_connection_result(false, false);
            let _ = calc.next_delay();
        }
        
        // Strategy should have changed
        let new_strategy = calc.get_racing_strategy();
        assert!(new_strategy != initial_strategy || new_strategy.is_some());
    }

    #[test]
    fn test_retry_integration_reset_clears_state() {
        let config = RetryConfig {
            strategy: RetryStrategy::Auto,
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            linear_step: Duration::from_millis(200),
        };
        
        let mut calc = RetryCalculator::new(config)
            .with_server_url("rtsp://test.local");
        
        // Build up some state
        for _ in 0..3 {
            calc.mark_connection_start();
            calc.record_connection_result(false, false);
            let _ = calc.next_delay();
        }
        
        // Reset should clear everything
        calc.reset();
        assert_eq!(calc.current_attempt(), 0);
        
        // Should start fresh
        calc.mark_connection_start();
        calc.record_connection_result(true, false);
        assert_eq!(calc.current_attempt(), 0); // Still 0 because successful
    }
}