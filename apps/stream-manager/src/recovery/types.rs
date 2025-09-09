use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    Network(String),
    Pipeline(String),
    Resource(String),
    System(String),
    Configuration(String),
    Unknown(String),
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorType::Network(msg) => write!(f, "Network error: {}", msg),
            ErrorType::Pipeline(msg) => write!(f, "Pipeline error: {}", msg),
            ErrorType::Resource(msg) => write!(f, "Resource error: {}", msg),
            ErrorType::System(msg) => write!(f, "System error: {}", msg),
            ErrorType::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            ErrorType::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorClass {
    Transient,
    Recoverable,
    Fatal,
    Cascade,
}

#[derive(Debug, Clone, Error)]
pub enum RecoveryError {
    #[error("Maximum retries exceeded")]
    MaxRetriesExceeded,
    
    #[error("No recovery handler registered")]
    NoHandler,
    
    #[error("No snapshot available for recovery")]
    NoSnapshot,
    
    #[error("Recovery timeout")]
    Timeout,
    
    #[error("Circuit breaker open")]
    CircuitBreakerOpen,
    
    #[error("Fatal error: {0}")]
    FatalError(String),
    
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
    
    #[error("Dependency failure: {0}")]
    DependencyFailure(String),
}

#[derive(Debug, Clone)]
pub enum RecoveryResult {
    Recovered,
    PartialRecovery,
    Failed(RecoveryError),
    Deferred,
}

impl RecoveryResult {
    pub fn is_success(&self) -> bool {
        matches!(self, RecoveryResult::Recovered | RecoveryResult::PartialRecovery)
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, RecoveryResult::Failed(_))
    }
}

#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    Immediate,
    ExponentialBackoff,
    LinearBackoff,
    CircuitBreaker,
    Failover,
    Degraded,
}

#[derive(Debug, Clone)]
pub struct RecoveryPolicy {
    pub strategy: RecoveryStrategy,
    pub max_attempts: u32,
    pub timeout_ms: u64,
    pub allow_partial: bool,
    pub cascade_recovery: bool,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            strategy: RecoveryStrategy::ExponentialBackoff,
            max_attempts: 5,
            timeout_ms: 30000,
            allow_partial: true,
            cascade_recovery: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecoveryContext {
    pub component_id: String,
    pub error: ErrorType,
    pub attempt: u32,
    pub policy: RecoveryPolicy,
}

pub trait Recoverable: Send + Sync {
    fn can_recover(&self, error: &ErrorType) -> bool;
    fn recovery_strategy(&self) -> RecoveryStrategy;
    fn create_snapshot(&self) -> Result<Vec<u8>, RecoveryError>;
    fn restore_from_snapshot(&mut self, data: Vec<u8>) -> Result<(), RecoveryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_type_display() {
        let error = ErrorType::Network("Connection timeout".to_string());
        assert_eq!(format!("{}", error), "Network error: Connection timeout");
        
        let error = ErrorType::Pipeline("State change failed".to_string());
        assert_eq!(format!("{}", error), "Pipeline error: State change failed");
    }

    #[test]
    fn test_recovery_result() {
        let result = RecoveryResult::Recovered;
        assert!(result.is_success());
        assert!(!result.is_failure());
        
        let result = RecoveryResult::Failed(RecoveryError::MaxRetriesExceeded);
        assert!(!result.is_success());
        assert!(result.is_failure());
        
        let result = RecoveryResult::PartialRecovery;
        assert!(result.is_success());
        assert!(!result.is_failure());
    }

    #[test]
    fn test_recovery_policy_default() {
        let policy = RecoveryPolicy::default();
        assert!(matches!(policy.strategy, RecoveryStrategy::ExponentialBackoff));
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.timeout_ms, 30000);
        assert!(policy.allow_partial);
        assert!(!policy.cascade_recovery);
    }

    #[test]
    fn test_recovery_error_display() {
        let error = RecoveryError::MaxRetriesExceeded;
        assert_eq!(format!("{}", error), "Maximum retries exceeded");
        
        let error = RecoveryError::FatalError("System crash".to_string());
        assert_eq!(format!("{}", error), "Fatal error: System crash");
    }
}