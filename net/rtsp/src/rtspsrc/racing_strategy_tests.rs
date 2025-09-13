// GStreamer RTSP dynamic racing strategy tests
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
    use super::super::auto_selector::*;
    use super::super::connection_racer::*;
    use super::super::retry::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_racing_strategy_updates_from_auto_mode() {
        let config = RetryConfig {
            strategy: RetryStrategy::Auto,
            max_attempts: 20,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            linear_step: Duration::from_millis(200),
        };
        
        let mut calc = RetryCalculator::new(config)
            .with_server_url("rtsp://test.local");
        
        // Initial strategy should be None
        let initial = calc.get_racing_strategy();
        assert!(initial.is_none() || initial == Some(ConnectionRacingStrategy::None));
        
        // Simulate multiple failures to trigger pattern detection
        for _ in 0..10 {
            calc.mark_connection_start();
            calc.record_connection_result(false, false);
            let _ = calc.next_delay();
        }
        
        // Should recommend a racing strategy after failures
        let recommended = calc.get_racing_strategy();
        assert!(recommended.is_some());
    }

    #[test]
    fn test_racing_strategy_change_on_pattern() {
        let mut selector = AutoRetrySelector::new();
        
        // Simulate connection-limited pattern (connections succeed but drop quickly)
        // The selector only looks at the last 3 attempts, so ensure those show the pattern
        for i in 0..5 {
            selector.record_attempt(ConnectionAttemptResult {
                success: true,
                connection_duration: Some(Duration::from_secs(2)), // Short connections
                timestamp: Instant::now(),
                retry_count: i,
            });
        }
        
        // Should recommend LastWins for connection-limited
        let pattern = selector.get_pattern();
        assert_eq!(pattern, NetworkPattern::ConnectionLimited);
        let strategy = selector.get_racing_strategy();
        assert_eq!(strategy, ConnectionRacingStrategy::LastWins);
    }

    #[test]
    fn test_racing_strategy_transitions() {
        let mut selector = AutoRetrySelector::new();
        
        // Start with stable pattern
        for _ in 0..5 {
            selector.record_attempt(ConnectionAttemptResult {
                success: true,
                connection_duration: Some(Duration::from_secs(60)),
                timestamp: Instant::now(),
                retry_count: 0,
            });
        }
        
        let initial = selector.get_racing_strategy();
        assert_eq!(initial, ConnectionRacingStrategy::None);
        
        // Transition to lossy pattern
        for i in 0..10 {
            selector.record_attempt(ConnectionAttemptResult {
                success: i % 3 == 0, // 33% success rate
                connection_duration: if i % 3 == 0 { 
                    Some(Duration::from_secs(5)) 
                } else { 
                    None 
                },
                timestamp: Instant::now(),
                retry_count: i,
            });
        }
        
        let after_loss = selector.get_racing_strategy();
        assert_eq!(after_loss, ConnectionRacingStrategy::FirstWins);
    }

    #[test]
    fn test_connection_racer_strategy_update() {
        let config = ConnectionRacingConfig {
            strategy: ConnectionRacingStrategy::None,
            max_parallel_connections: 3,
            racing_delay_ms: 250,
            racing_timeout: Duration::from_secs(5),
            proxy_config: None,
        };
        
        let mut racer = ConnectionRacer::new(config);
        
        // Initial strategy
        assert_eq!(racer.current_strategy(), ConnectionRacingStrategy::None);
        
        // Update to FirstWins
        racer.update_strategy(ConnectionRacingStrategy::FirstWins);
        assert_eq!(racer.current_strategy(), ConnectionRacingStrategy::FirstWins);
        
        // Update to LastWins
        racer.update_strategy(ConnectionRacingStrategy::LastWins);
        assert_eq!(racer.current_strategy(), ConnectionRacingStrategy::LastWins);
        
        // Update to Hybrid
        racer.update_strategy(ConnectionRacingStrategy::Hybrid);
        assert_eq!(racer.current_strategy(), ConnectionRacingStrategy::Hybrid);
    }

    #[test]
    fn test_strategy_mapping_patterns() {
        let mut selector = AutoRetrySelector::new();
        
        // Test ConnectionLimited → LastWins
        // Need successful connections that drop quickly
        for i in 0..10 {
            selector.record_attempt(ConnectionAttemptResult {
                success: true, // Connections succeed
                connection_duration: Some(Duration::from_secs(2)), // But drop quickly
                timestamp: Instant::now(),
                retry_count: i,
            });
        }
        assert_eq!(selector.get_pattern(), NetworkPattern::ConnectionLimited);
        assert_eq!(selector.get_racing_strategy(), ConnectionRacingStrategy::LastWins);
        
        selector.reset();
        
        // Test HighPacketLoss → FirstWins
        for i in 0..12 {
            selector.record_attempt(ConnectionAttemptResult {
                success: i % 4 == 0, // 25% success
                connection_duration: if i % 4 == 0 {
                    Some(Duration::from_secs(3))
                } else {
                    None
                },
                timestamp: Instant::now(),
                retry_count: i % 3,
            });
        }
        assert_eq!(selector.get_pattern(), NetworkPattern::HighPacketLoss);
        assert_eq!(selector.get_racing_strategy(), ConnectionRacingStrategy::FirstWins);
        
        selector.reset();
        
        // Test Stable → None
        for _ in 0..5 {
            selector.record_attempt(ConnectionAttemptResult {
                success: true,
                connection_duration: Some(Duration::from_secs(60)),
                timestamp: Instant::now(),
                retry_count: 0,
            });
        }
        assert_eq!(selector.get_pattern(), NetworkPattern::Stable);
        assert_eq!(selector.get_racing_strategy(), ConnectionRacingStrategy::None);
    }

    #[test]
    fn test_strategy_persistence_across_retries() {
        let config = RetryConfig {
            strategy: RetryStrategy::Auto,
            max_attempts: 30,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            linear_step: Duration::from_millis(200),
        };
        
        let mut calc = RetryCalculator::new(config)
            .with_server_url("rtsp://test.local");
        
        // Build up a pattern
        for _ in 0..10 {
            calc.mark_connection_start();
            calc.record_connection_result(false, false);
            let _ = calc.next_delay();
        }
        
        let strategy1 = calc.get_racing_strategy();
        
        // Continue with more attempts
        for _ in 0..5 {
            calc.mark_connection_start();
            calc.record_connection_result(false, false);
            let _ = calc.next_delay();
        }
        
        let strategy2 = calc.get_racing_strategy();
        
        // Strategy should be consistent or evolve predictably
        assert!(strategy1.is_some());
        assert!(strategy2.is_some());
    }

    #[test]
    fn test_smooth_strategy_transitions() {
        let mut selector = AutoRetrySelector::new();
        
        // Gradual transition from stable to lossy
        for i in 0..20 {
            let success = if i < 5 {
                true // Start stable
            } else if i < 10 {
                i % 2 == 0 // Transition to 50% success
            } else {
                i % 4 == 0 // End with 25% success
            };
            
            selector.record_attempt(ConnectionAttemptResult {
                success,
                connection_duration: if success {
                    Some(Duration::from_secs(10))
                } else {
                    None
                },
                timestamp: Instant::now(),
                retry_count: if success { 0 } else { 1 },
            });
        }
        
        // Should detect high packet loss pattern
        let pattern = selector.get_pattern();
        assert!(pattern == NetworkPattern::HighPacketLoss || 
                pattern == NetworkPattern::ConnectionLimited);
        
        // Should recommend appropriate racing strategy
        let strategy = selector.get_racing_strategy();
        assert!(strategy == ConnectionRacingStrategy::FirstWins || 
                strategy == ConnectionRacingStrategy::LastWins);
    }
}