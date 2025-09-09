use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    state: Arc<RwLock<State>>,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    half_open_requests: Arc<AtomicU32>,
    max_half_open_requests: u32,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(State::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            failure_threshold,
            success_threshold,
            timeout,
            half_open_requests: Arc::new(AtomicU32::new(0)),
            max_half_open_requests: 3,
        }
    }

    pub fn record_success(&mut self) {
        let mut state = self.state.write();
        let success_count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;

        match *state {
            State::HalfOpen => {
                if success_count >= self.success_threshold {
                    *state = State::Closed;
                    self.reset_counters();
                }
            }
            State::Open => {
                // Success during open state shouldn't happen, but if it does, ignore it
            }
            State::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::SeqCst);
            }
        }
    }

    pub fn record_failure(&mut self) {
        let mut state = self.state.write();
        let failure_count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        
        *self.last_failure_time.write() = Some(Instant::now());

        match *state {
            State::Closed => {
                if failure_count >= self.failure_threshold {
                    *state = State::Open;
                }
            }
            State::HalfOpen => {
                *state = State::Open;
                self.reset_counters();
            }
            State::Open => {
                // Already open, just update the failure time
            }
        }
    }

    pub fn is_closed(&self) -> bool {
        self.check_state();
        matches!(*self.state.read(), State::Closed)
    }

    pub fn is_open(&self) -> bool {
        self.check_state();
        matches!(*self.state.read(), State::Open)
    }

    pub fn is_half_open(&self) -> bool {
        self.check_state();
        matches!(*self.state.read(), State::HalfOpen)
    }

    pub fn can_attempt(&mut self) -> bool {
        self.check_state();
        
        let state = self.state.read();
        match *state {
            State::Closed => true,
            State::Open => false,
            State::HalfOpen => {
                let current = self.half_open_requests.load(Ordering::SeqCst);
                if current < self.max_half_open_requests {
                    self.half_open_requests.fetch_add(1, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn get_state(&self) -> State {
        self.check_state();
        *self.state.read()
    }

    pub fn reset(&mut self) {
        *self.state.write() = State::Closed;
        self.reset_counters();
        *self.last_failure_time.write() = None;
    }

    fn check_state(&self) {
        let mut state = self.state.write();
        
        if *state == State::Open {
            if let Some(last_failure) = *self.last_failure_time.read() {
                if last_failure.elapsed() >= self.timeout {
                    *state = State::HalfOpen;
                    self.half_open_requests.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                }
            }
        }
    }

    fn reset_counters(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        self.half_open_requests.store(0, Ordering::SeqCst);
    }
}

impl Clone for CircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            failure_count: Arc::clone(&self.failure_count),
            success_count: Arc::clone(&self.success_count),
            last_failure_time: Arc::clone(&self.last_failure_time),
            failure_threshold: self.failure_threshold,
            success_threshold: self.success_threshold,
            timeout: self.timeout,
            half_open_requests: Arc::clone(&self.half_open_requests),
            max_half_open_requests: self.max_half_open_requests,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(5));
        assert_eq!(cb.get_state(), State::Closed);
        assert!(cb.is_closed());
        assert!(!cb.is_open());
        assert!(!cb.is_half_open());
    }

    #[test]
    fn test_circuit_breaker_opens_on_threshold() {
        let mut cb = CircuitBreaker::new(3, 2, Duration::from_secs(5));
        
        assert!(cb.can_attempt());
        cb.record_failure();
        assert_eq!(cb.get_state(), State::Closed);
        
        cb.record_failure();
        assert_eq!(cb.get_state(), State::Closed);
        
        cb.record_failure();
        assert_eq!(cb.get_state(), State::Open);
        assert!(!cb.can_attempt());
    }

    #[test]
    fn test_circuit_breaker_half_open_after_timeout() {
        let mut cb = CircuitBreaker::new(1, 2, Duration::from_millis(100));
        
        cb.record_failure();
        assert_eq!(cb.get_state(), State::Open);
        
        thread::sleep(Duration::from_millis(150));
        
        assert_eq!(cb.get_state(), State::HalfOpen);
        assert!(cb.can_attempt());
    }

    #[test]
    fn test_circuit_breaker_closes_on_success_threshold() {
        let mut cb = CircuitBreaker::new(1, 2, Duration::from_millis(100));
        
        cb.record_failure();
        assert_eq!(cb.get_state(), State::Open);
        
        thread::sleep(Duration::from_millis(150));
        assert_eq!(cb.get_state(), State::HalfOpen);
        
        cb.record_success();
        assert_eq!(cb.get_state(), State::HalfOpen);
        
        cb.record_success();
        assert_eq!(cb.get_state(), State::Closed);
    }

    #[test]
    fn test_circuit_breaker_reopens_on_half_open_failure() {
        let mut cb = CircuitBreaker::new(1, 2, Duration::from_millis(100));
        
        cb.record_failure();
        assert_eq!(cb.get_state(), State::Open);
        
        thread::sleep(Duration::from_millis(150));
        assert_eq!(cb.get_state(), State::HalfOpen);
        
        cb.record_failure();
        assert_eq!(cb.get_state(), State::Open);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let mut cb = CircuitBreaker::new(1, 2, Duration::from_secs(5));
        
        cb.record_failure();
        assert_eq!(cb.get_state(), State::Open);
        
        cb.reset();
        assert_eq!(cb.get_state(), State::Closed);
        assert!(cb.can_attempt());
    }

    #[test]
    fn test_half_open_request_limiting() {
        let mut cb = CircuitBreaker::new(1, 2, Duration::from_millis(100));
        
        cb.record_failure();
        thread::sleep(Duration::from_millis(150));
        
        assert_eq!(cb.get_state(), State::HalfOpen);
        
        // Should allow up to max_half_open_requests
        for _ in 0..3 {
            assert!(cb.can_attempt());
        }
        
        // Should deny further requests
        assert!(!cb.can_attempt());
    }
}