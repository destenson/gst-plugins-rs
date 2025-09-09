use rand::Rng;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum BackoffType {
    Fixed,
    Linear,
    Exponential,
    Fibonacci,
}

#[derive(Debug, Clone)]
pub struct BackoffStrategy {
    backoff_type: BackoffType,
    base_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
    jitter: bool,
    attempt: u32,
    fibonacci_prev: u64,
    fibonacci_curr: u64,
}

impl BackoffStrategy {
    pub fn new(backoff_type: BackoffType, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            backoff_type,
            base_delay,
            max_delay,
            multiplier: 2.0,
            jitter: true,
            attempt: 0,
            fibonacci_prev: 0,
            fibonacci_curr: 1,
        }
    }

    pub fn fixed(delay: Duration) -> Self {
        Self::new(BackoffType::Fixed, delay, delay).with_jitter(false)
    }

    pub fn linear(base_delay: Duration, max_delay: Duration) -> Self {
        Self::new(BackoffType::Linear, base_delay, max_delay)
    }

    pub fn exponential(base_delay: Duration, max_delay: Duration) -> Self {
        Self::new(BackoffType::Exponential, base_delay, max_delay)
    }

    pub fn fibonacci(base_delay: Duration, max_delay: Duration) -> Self {
        Self::new(BackoffType::Fibonacci, base_delay, max_delay)
    }

    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    pub fn next_delay(&mut self) -> Duration {
        self.attempt += 1;
        
        let delay = match self.backoff_type {
            BackoffType::Fixed => self.base_delay,
            BackoffType::Linear => self.calculate_linear_delay(),
            BackoffType::Exponential => self.calculate_exponential_delay(),
            BackoffType::Fibonacci => self.calculate_fibonacci_delay(),
        };

        let delay = self.apply_max_limit(delay);
        
        if self.jitter {
            self.apply_jitter(delay)
        } else {
            delay
        }
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
        self.fibonacci_prev = 0;
        self.fibonacci_curr = 1;
    }

    pub fn get_attempt(&self) -> u32 {
        self.attempt
    }

    fn calculate_linear_delay(&self) -> Duration {
        self.base_delay * self.attempt
    }

    fn calculate_exponential_delay(&self) -> Duration {
        let multiplier = self.multiplier.powi(self.attempt.saturating_sub(1) as i32);
        
        let millis = (self.base_delay.as_millis() as f64 * multiplier) as u64;
        Duration::from_millis(millis)
    }

    fn calculate_fibonacci_delay(&mut self) -> Duration {
        let fib_value = if self.attempt == 1 {
            1
        } else {
            let next = self.fibonacci_prev + self.fibonacci_curr;
            self.fibonacci_prev = self.fibonacci_curr;
            self.fibonacci_curr = next;
            next
        };
        
        self.base_delay * fib_value as u32
    }

    fn apply_max_limit(&self, delay: Duration) -> Duration {
        if delay > self.max_delay {
            self.max_delay
        } else {
            delay
        }
    }

    fn apply_jitter(&self, delay: Duration) -> Duration {
        let mut rng = rand::thread_rng();
        let jitter_factor = rng.gen_range(0.5..=1.5);
        
        let millis = (delay.as_millis() as f64 * jitter_factor) as u64;
        let jittered = Duration::from_millis(millis);
        
        self.apply_max_limit(jittered)
    }
}

pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff_strategy: BackoffStrategy,
    pub retry_on: Vec<String>,
    pub give_up_after: Option<Duration>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            backoff_strategy: BackoffStrategy::exponential(
                Duration::from_secs(1),
                Duration::from_secs(60),
            ),
            retry_on: vec![],
            give_up_after: Some(Duration::from_secs(300)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_backoff() {
        let mut strategy = BackoffStrategy::fixed(Duration::from_secs(5));
        
        assert_eq!(strategy.next_delay(), Duration::from_secs(5));
        assert_eq!(strategy.next_delay(), Duration::from_secs(5));
        assert_eq!(strategy.next_delay(), Duration::from_secs(5));
    }

    #[test]
    fn test_linear_backoff() {
        let mut strategy = BackoffStrategy::linear(
            Duration::from_secs(1),
            Duration::from_secs(10),
        ).with_jitter(false);
        
        assert_eq!(strategy.next_delay(), Duration::from_secs(1));
        assert_eq!(strategy.next_delay(), Duration::from_secs(2));
        assert_eq!(strategy.next_delay(), Duration::from_secs(3));
    }

    #[test]
    fn test_exponential_backoff() {
        let mut strategy = BackoffStrategy::exponential(
            Duration::from_secs(1),
            Duration::from_secs(100),
        ).with_jitter(false);
        
        assert_eq!(strategy.next_delay(), Duration::from_secs(1));
        assert_eq!(strategy.next_delay(), Duration::from_secs(2));
        assert_eq!(strategy.next_delay(), Duration::from_secs(4));
        assert_eq!(strategy.next_delay(), Duration::from_secs(8));
    }

    #[test]
    fn test_fibonacci_backoff() {
        let mut strategy = BackoffStrategy::fibonacci(
            Duration::from_secs(1),
            Duration::from_secs(100),
        ).with_jitter(false);
        
        assert_eq!(strategy.next_delay(), Duration::from_secs(1));
        assert_eq!(strategy.next_delay(), Duration::from_secs(1));
        assert_eq!(strategy.next_delay(), Duration::from_secs(2));
        assert_eq!(strategy.next_delay(), Duration::from_secs(3));
        assert_eq!(strategy.next_delay(), Duration::from_secs(5));
    }

    #[test]
    fn test_max_delay_limit() {
        let mut strategy = BackoffStrategy::exponential(
            Duration::from_secs(1),
            Duration::from_secs(5),
        ).with_jitter(false);
        
        assert_eq!(strategy.next_delay(), Duration::from_secs(1));
        assert_eq!(strategy.next_delay(), Duration::from_secs(2));
        assert_eq!(strategy.next_delay(), Duration::from_secs(4));
        assert_eq!(strategy.next_delay(), Duration::from_secs(5)); // Limited by max
        assert_eq!(strategy.next_delay(), Duration::from_secs(5)); // Still limited
    }

    #[test]
    fn test_backoff_reset() {
        let mut strategy = BackoffStrategy::exponential(
            Duration::from_secs(1),
            Duration::from_secs(100),
        ).with_jitter(false);
        
        assert_eq!(strategy.next_delay(), Duration::from_secs(1));
        assert_eq!(strategy.next_delay(), Duration::from_secs(2));
        assert_eq!(strategy.get_attempt(), 2);
        
        strategy.reset();
        assert_eq!(strategy.get_attempt(), 0);
        assert_eq!(strategy.next_delay(), Duration::from_secs(1));
    }

    #[test]
    fn test_jitter_application() {
        let mut strategy = BackoffStrategy::fixed(Duration::from_secs(10))
            .with_jitter(true);
        
        let delay = strategy.next_delay();
        
        // Jitter should keep delay between 5 and 15 seconds
        assert!(delay >= Duration::from_secs(5));
        assert!(delay <= Duration::from_secs(15));
    }

    #[test]
    fn test_custom_multiplier() {
        let mut strategy = BackoffStrategy::exponential(
            Duration::from_secs(1),
            Duration::from_secs(100),
        )
        .with_multiplier(3.0)
        .with_jitter(false);
        
        assert_eq!(strategy.next_delay(), Duration::from_secs(1));
        assert_eq!(strategy.next_delay(), Duration::from_secs(3));
        assert_eq!(strategy.next_delay(), Duration::from_secs(9));
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        
        assert_eq!(config.max_attempts, 5);
        assert!(config.give_up_after.is_some());
        assert_eq!(config.give_up_after.unwrap(), Duration::from_secs(300));
    }
}