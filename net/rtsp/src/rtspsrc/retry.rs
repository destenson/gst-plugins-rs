// GStreamer RTSP plugin retry logic implementation
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use std::time::Duration;
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    Auto,
    Adaptive,
    None,
    Immediate,
    Linear,
    Exponential,
    ExponentialJitter,
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
            "adaptive" => RetryStrategy::Adaptive,
            "none" => RetryStrategy::None,
            "immediate" => RetryStrategy::Immediate,
            "linear" => RetryStrategy::Linear,
            "exponential" => RetryStrategy::Exponential,
            "exponential-jitter" => RetryStrategy::ExponentialJitter,
            _ => RetryStrategy::Auto,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RetryStrategy::Auto => "auto",
            RetryStrategy::Adaptive => "adaptive",
            RetryStrategy::None => "none",
            RetryStrategy::Immediate => "immediate",
            RetryStrategy::Linear => "linear",
            RetryStrategy::Exponential => "exponential",
            RetryStrategy::ExponentialJitter => "exponential-jitter",
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
}

impl RetryCalculator {
    pub fn new(config: RetryConfig) -> Self {
        Self { config, attempt: 0 }
    }

    pub fn should_retry(&self) -> bool {
        if self.config.strategy == RetryStrategy::None {
            return false;
        }
        
        if self.config.max_attempts < 0 {
            // Infinite retries
            true
        } else {
            self.attempt < self.config.max_attempts as u32
        }
    }

    pub fn next_delay(&mut self) -> Option<Duration> {
        if !self.should_retry() {
            return None;
        }

        let delay = match self.config.strategy {
            RetryStrategy::None => return None,
            RetryStrategy::Immediate => Duration::ZERO,
            RetryStrategy::Linear => self.calculate_linear_delay(),
            RetryStrategy::Exponential => self.calculate_exponential_delay(false),
            RetryStrategy::ExponentialJitter => self.calculate_exponential_delay(true),
            RetryStrategy::Auto => self.calculate_auto_delay(),
            RetryStrategy::Adaptive => self.calculate_adaptive_delay(),
        };

        self.attempt += 1;
        
        // Cap at max_delay
        Some(delay.min(self.config.max_delay))
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    pub fn current_attempt(&self) -> u32 {
        self.attempt
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

    fn calculate_auto_delay(&self) -> Duration {
        // Simple heuristic: use exponential with jitter for network issues
        // This provides good balance between quick recovery and avoiding overload
        self.calculate_exponential_delay(true)
    }

    fn calculate_adaptive_delay(&self) -> Duration {
        // For now, use exponential with jitter
        // In PRP-28, this will be enhanced with learning-based optimization
        self.calculate_exponential_delay(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_from_string() {
        assert_eq!(RetryStrategy::from_string("none"), RetryStrategy::None);
        assert_eq!(RetryStrategy::from_string("immediate"), RetryStrategy::Immediate);
        assert_eq!(RetryStrategy::from_string("linear"), RetryStrategy::Linear);
        assert_eq!(RetryStrategy::from_string("exponential"), RetryStrategy::Exponential);
        assert_eq!(RetryStrategy::from_string("exponential-jitter"), RetryStrategy::ExponentialJitter);
        assert_eq!(RetryStrategy::from_string("auto"), RetryStrategy::Auto);
        assert_eq!(RetryStrategy::from_string("adaptive"), RetryStrategy::Adaptive);
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
        
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(1)));  // 1s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(3)));  // 1s + 2s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(5)));  // 1s + 4s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(7)));  // 1s + 6s
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
        
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(1)));   // 1s * 2^0
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(2)));   // 1s * 2^1
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(4)));   // 1s * 2^2
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(8)));   // 1s * 2^3
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(16)));  // 1s * 2^4
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
        
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(1)));  // 1s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(2)));  // 2s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(4)));  // 4s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(5)));  // Capped at 5s
        assert_eq!(calc.next_delay(), Some(Duration::from_secs(5)));  // Still capped
    }

    #[test]
    fn test_infinite_retries() {
        let config = RetryConfig {
            strategy: RetryStrategy::Immediate,
            max_attempts: -1,  // Infinite
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