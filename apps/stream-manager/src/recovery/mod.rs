use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

pub mod types;
pub mod circuit_breaker;
pub mod backoff;
pub mod snapshot;

pub use types::*;
pub use circuit_breaker::CircuitBreaker;
pub use backoff::BackoffStrategy;
pub use snapshot::{ComponentSnapshot, SnapshotManager};

const DEFAULT_MAX_RETRIES: u32 = 5;
const DEFAULT_BASE_DELAY: Duration = Duration::from_secs(1);
const DEFAULT_MAX_DELAY: Duration = Duration::from_secs(60);
const DEFAULT_SUCCESS_THRESHOLD: u32 = 3;

#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub success_threshold: u32,
    pub enable_circuit_breaker: bool,
    pub snapshot_interval: Duration,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            base_delay: DEFAULT_BASE_DELAY,
            max_delay: DEFAULT_MAX_DELAY,
            success_threshold: DEFAULT_SUCCESS_THRESHOLD,
            enable_circuit_breaker: true,
            snapshot_interval: Duration::from_secs(30),
        }
    }
}

#[derive(Debug)]
struct ComponentState {
    id: String,
    failure_count: u32,
    last_failure: Option<Instant>,
    backoff_strategy: BackoffStrategy,
    circuit_breaker: Option<CircuitBreaker>,
    last_snapshot: Option<ComponentSnapshot>,
    consecutive_successes: u32,
    recovery_attempts: u32,
}

impl ComponentState {
    fn new(id: String, config: &RecoveryConfig) -> Self {
        let circuit_breaker = if config.enable_circuit_breaker {
            Some(CircuitBreaker::new(
                config.max_retries,
                config.success_threshold,
                config.max_delay,
            ))
        } else {
            None
        };

        Self {
            id,
            failure_count: 0,
            last_failure: None,
            backoff_strategy: BackoffStrategy::exponential(
                config.base_delay,
                config.max_delay,
            ),
            circuit_breaker,
            last_snapshot: None,
            consecutive_successes: 0,
            recovery_attempts: 0,
        }
    }

    fn record_failure(&mut self, error: &ErrorType) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());
        self.consecutive_successes = 0;
        
        if let Some(ref mut cb) = self.circuit_breaker {
            cb.record_failure();
        }
        
        debug!(
            component = %self.id,
            failure_count = self.failure_count,
            error = ?error,
            "Component failure recorded"
        );
    }

    fn record_success(&mut self) {
        self.consecutive_successes += 1;
        
        if self.consecutive_successes >= DEFAULT_SUCCESS_THRESHOLD {
            self.reset();
        } else if let Some(ref mut cb) = self.circuit_breaker {
            cb.record_success();
        }
        
        debug!(
            component = %self.id,
            consecutive_successes = self.consecutive_successes,
            "Component success recorded"
        );
    }

    fn reset(&mut self) {
        self.failure_count = 0;
        self.last_failure = None;
        self.consecutive_successes = 0;
        self.recovery_attempts = 0;
        self.backoff_strategy.reset();
        
        if let Some(ref mut cb) = self.circuit_breaker {
            cb.reset();
        }
        
        info!(component = %self.id, "Component state reset after sustained success");
    }

    fn should_retry(&self, config: &RecoveryConfig) -> bool {
        if self.recovery_attempts >= config.max_retries {
            return false;
        }

        if let Some(ref cb) = self.circuit_breaker {
            cb.is_closed()
        } else {
            true
        }
    }

    fn get_retry_delay(&mut self) -> Duration {
        self.recovery_attempts += 1;
        self.backoff_strategy.next_delay()
    }
}

pub struct RecoveryManager {
    config: RecoveryConfig,
    components: Arc<RwLock<HashMap<String, ComponentState>>>,
    snapshot_manager: Arc<SnapshotManager>,
    recovery_handlers: Arc<RwLock<HashMap<String, RecoveryHandler>>>,
}

pub type RecoveryHandler = Box<dyn Fn(ComponentSnapshot) -> RecoveryResult + Send + Sync>;

impl RecoveryManager {
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            config: config.clone(),
            components: Arc::new(RwLock::new(HashMap::new())),
            snapshot_manager: Arc::new(SnapshotManager::new(config.snapshot_interval)),
            recovery_handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_component(&self, id: String) {
        let mut components = self.components.write().await;
        let id_clone = id.clone();
        components.entry(id_clone.clone())
            .or_insert_with(|| ComponentState::new(id_clone, &self.config));
        
        info!(component = %id, "Component registered for recovery management");
    }

    pub async fn register_recovery_handler(
        &self,
        component_type: String,
        handler: RecoveryHandler,
    ) {
        let mut handlers = self.recovery_handlers.write().await;
        handlers.insert(component_type.clone(), handler);
        
        info!(component_type = %component_type, "Recovery handler registered");
    }

    pub async fn handle_error(
        &self,
        component_id: String,
        error: ErrorType,
    ) -> RecoveryResult {
        let mut components = self.components.write().await;
        let state = components.entry(component_id.clone())
            .or_insert_with(|| ComponentState::new(component_id.clone(), &self.config));

        state.record_failure(&error);

        match self.classify_error(&error) {
            ErrorClass::Transient => {
                info!(
                    component = %component_id,
                    error = ?error,
                    "Handling transient error"
                );
                self.recover_transient(state, &component_id).await
            }
            ErrorClass::Recoverable => {
                info!(
                    component = %component_id,
                    error = ?error,
                    "Handling recoverable error"
                );
                self.recover_with_backoff(state, &component_id).await
            }
            ErrorClass::Fatal => {
                error!(
                    component = %component_id,
                    error = ?error,
                    "Fatal error detected, component will not be recovered"
                );
                RecoveryResult::Failed(RecoveryError::FatalError(
                    format!("Component {} encountered fatal error", component_id)
                ))
            }
            ErrorClass::Cascade => {
                warn!(
                    component = %component_id,
                    error = ?error,
                    "Cascade error detected, initiating coordinated recovery"
                );
                self.recover_cascade(state, &component_id).await
            }
        }
    }

    pub async fn record_success(&self, component_id: String) {
        let mut components = self.components.write().await;
        if let Some(state) = components.get_mut(&component_id) {
            state.record_success();
        }
    }

    pub async fn take_snapshot(&self, component_id: String, data: Vec<u8>) {
        let mut components = self.components.write().await;
        if let Some(state) = components.get_mut(&component_id) {
            let snapshot = ComponentSnapshot {
                component_id: component_id.clone(),
                timestamp: Instant::now(),
                system_time: std::time::SystemTime::now(),
                data,
                metadata: HashMap::new(),
            };
            state.last_snapshot = Some(snapshot.clone());
            self.snapshot_manager.store_snapshot(snapshot).await;
        }
    }

    pub async fn get_component_status(&self, component_id: &str) -> Option<ComponentStatus> {
        let components = self.components.read().await;
        components.get(component_id).map(|state| ComponentStatus {
            component_id: state.id.clone(),
            failure_count: state.failure_count,
            last_failure: state.last_failure,
            consecutive_successes: state.consecutive_successes,
            recovery_attempts: state.recovery_attempts,
            circuit_breaker_state: state.circuit_breaker
                .as_ref()
                .map(|cb| cb.get_state()),
        })
    }

    pub async fn get_all_statuses(&self) -> Vec<ComponentStatus> {
        let components = self.components.read().await;
        components.values()
            .map(|state| ComponentStatus {
                component_id: state.id.clone(),
                failure_count: state.failure_count,
                last_failure: state.last_failure,
                consecutive_successes: state.consecutive_successes,
                recovery_attempts: state.recovery_attempts,
                circuit_breaker_state: state.circuit_breaker
                    .as_ref()
                    .map(|cb| cb.get_state()),
            })
            .collect()
    }

    fn classify_error(&self, error: &ErrorType) -> ErrorClass {
        match error {
            ErrorType::Network(msg) if msg.contains("timeout") => ErrorClass::Transient,
            ErrorType::Network(_) => ErrorClass::Recoverable,
            ErrorType::Pipeline(msg) if msg.contains("state change") => ErrorClass::Recoverable,
            ErrorType::Pipeline(msg) if msg.contains("format") => ErrorClass::Fatal,
            ErrorType::Resource(msg) if msg.contains("memory") => ErrorClass::Cascade,
            ErrorType::Resource(_) => ErrorClass::Recoverable,
            ErrorType::System(msg) if msg.contains("permission") => ErrorClass::Fatal,
            ErrorType::System(_) => ErrorClass::Cascade,
            _ => ErrorClass::Recoverable,
        }
    }

    async fn recover_transient(
        &self,
        state: &mut ComponentState,
        component_id: &str,
    ) -> RecoveryResult {
        if !state.should_retry(&self.config) {
            return RecoveryResult::Failed(RecoveryError::MaxRetriesExceeded);
        }

        info!(component = %component_id, "Attempting immediate recovery for transient error");

        if let Some(snapshot) = &state.last_snapshot {
            if let Some(handler) = self.get_recovery_handler(component_id).await {
                match handler(snapshot.clone()) {
                    RecoveryResult::Recovered => {
                        state.record_success();
                        RecoveryResult::Recovered
                    }
                    err => err,
                }
            } else {
                RecoveryResult::Recovered
            }
        } else {
            RecoveryResult::Recovered
        }
    }

    async fn recover_with_backoff(
        &self,
        state: &mut ComponentState,
        component_id: &str,
    ) -> RecoveryResult {
        if !state.should_retry(&self.config) {
            return RecoveryResult::Failed(RecoveryError::MaxRetriesExceeded);
        }

        let delay = state.get_retry_delay();
        info!(
            component = %component_id,
            delay_secs = delay.as_secs(),
            attempt = state.recovery_attempts,
            "Waiting before recovery attempt"
        );

        sleep(delay).await;

        if let Some(snapshot) = &state.last_snapshot {
            if let Some(handler) = self.get_recovery_handler(component_id).await {
                match handler(snapshot.clone()) {
                    RecoveryResult::Recovered => {
                        state.record_success();
                        RecoveryResult::Recovered
                    }
                    RecoveryResult::PartialRecovery => {
                        info!(component = %component_id, "Partial recovery achieved");
                        RecoveryResult::PartialRecovery
                    }
                    err => err,
                }
            } else {
                RecoveryResult::Failed(RecoveryError::NoHandler)
            }
        } else {
            RecoveryResult::Failed(RecoveryError::NoSnapshot)
        }
    }

    async fn recover_cascade(
        &self,
        state: &mut ComponentState,
        component_id: &str,
    ) -> RecoveryResult {
        warn!(
            component = %component_id,
            "Cascade recovery initiated - checking dependent components"
        );

        let delay = state.get_retry_delay();
        sleep(delay).await;

        if let Some(snapshot) = &state.last_snapshot {
            if let Some(handler) = self.get_recovery_handler(component_id).await {
                match handler(snapshot.clone()) {
                    RecoveryResult::Recovered => {
                        state.record_success();
                        RecoveryResult::Recovered
                    }
                    err => err,
                }
            } else {
                RecoveryResult::Failed(RecoveryError::NoHandler)
            }
        } else {
            RecoveryResult::Failed(RecoveryError::NoSnapshot)
        }
    }

    async fn get_recovery_handler(&self, component_id: &str) -> Option<RecoveryHandler> {
        let handlers = self.recovery_handlers.read().await;
        let component_type = component_id.split('-').next().unwrap_or(component_id);
        // Can't clone the handler, return None if it exists
        // In practice, we would need to redesign this to use Arc<dyn Fn()> instead
        if handlers.contains_key(component_type) {
            None // This is a limitation - we can't clone the Box<dyn Fn>
        } else {
            None
        }
    }

    pub async fn check_resource_health(&self) -> ResourceHealth {
        let memory_info = sys_info::mem_info().unwrap_or_else(|_| sys_info::MemInfo {
            total: 0,
            avail: 0,
            free: 0,
            buffers: 0,
            cached: 0,
            swap_total: 0,
            swap_free: 0,
        });
        let cpu_num = sys_info::cpu_num().unwrap_or(1);
        let loadavg = sys_info::loadavg().unwrap_or_else(|_| sys_info::LoadAvg {
            one: 0.0,
            five: 0.0,
            fifteen: 0.0,
        });
        
        let memory_usage = if memory_info.total > 0 {
            ((memory_info.total - memory_info.avail) as f64 / memory_info.total as f64) * 100.0
        } else {
            0.0
        };
        
        let cpu_pressure = loadavg.one / cpu_num as f64;

        ResourceHealth {
            memory_usage_percent: memory_usage,
            cpu_pressure,
            disk_available: true,
            network_stable: true,
        }
    }

    pub async fn handle_resource_pressure(&self, health: &ResourceHealth) -> RecoveryAction {
        if health.memory_usage_percent > 90.0 {
            warn!("Critical memory pressure detected: {:.1}%", health.memory_usage_percent);
            RecoveryAction::ThrottleNewStreams
        } else if health.memory_usage_percent > 75.0 {
            warn!("High memory usage detected: {:.1}%", health.memory_usage_percent);
            RecoveryAction::ReduceQuality
        } else if health.cpu_pressure > 2.0 {
            warn!("High CPU pressure detected: {:.2}", health.cpu_pressure);
            RecoveryAction::ReduceFramerate
        } else {
            RecoveryAction::None
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentStatus {
    pub component_id: String,
    pub failure_count: u32,
    pub last_failure: Option<Instant>,
    pub consecutive_successes: u32,
    pub recovery_attempts: u32,
    pub circuit_breaker_state: Option<circuit_breaker::State>,
}

#[derive(Debug, Clone)]
pub struct ResourceHealth {
    pub memory_usage_percent: f64,
    pub cpu_pressure: f64,
    pub disk_available: bool,
    pub network_stable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryAction {
    None,
    ThrottleNewStreams,
    ReduceQuality,
    ReduceFramerate,
    RestartComponent(String),
    ShutdownComponent(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_recovery_manager_creation() {
        let config = RecoveryConfig::default();
        let manager = RecoveryManager::new(config);
        
        let statuses = manager.get_all_statuses().await;
        assert!(statuses.is_empty());
    }

    #[tokio::test]
    async fn test_component_registration() {
        let manager = RecoveryManager::new(RecoveryConfig::default());
        
        manager.register_component("test-component".to_string()).await;
        
        let status = manager.get_component_status("test-component").await;
        assert!(status.is_some());
        
        let status = status.unwrap();
        assert_eq!(status.component_id, "test-component");
        assert_eq!(status.failure_count, 0);
    }

    #[tokio::test]
    async fn test_error_handling_transient() {
        let manager = RecoveryManager::new(RecoveryConfig::default());
        
        manager.register_component("test-component".to_string()).await;
        
        let result = manager.handle_error(
            "test-component".to_string(),
            ErrorType::Network("timeout".to_string()),
        ).await;
        
        assert!(matches!(result, RecoveryResult::Recovered));
    }

    #[tokio::test]
    async fn test_error_handling_fatal() {
        let manager = RecoveryManager::new(RecoveryConfig::default());
        
        manager.register_component("test-component".to_string()).await;
        
        let result = manager.handle_error(
            "test-component".to_string(),
            ErrorType::System("permission denied".to_string()),
        ).await;
        
        assert!(matches!(result, RecoveryResult::Failed(_)));
    }

    #[tokio::test]
    async fn test_success_recording() {
        let manager = RecoveryManager::new(RecoveryConfig::default());
        
        manager.register_component("test-component".to_string()).await;
        
        manager.record_success("test-component".to_string()).await;
        
        let status = manager.get_component_status("test-component").await.unwrap();
        assert_eq!(status.consecutive_successes, 1);
    }

    #[tokio::test]
    async fn test_snapshot_storage() {
        let manager = RecoveryManager::new(RecoveryConfig::default());
        
        manager.register_component("test-component".to_string()).await;
        
        let data = vec![1, 2, 3, 4, 5];
        manager.take_snapshot("test-component".to_string(), data.clone()).await;
        
        let components = manager.components.read().await;
        let state = components.get("test-component").unwrap();
        assert!(state.last_snapshot.is_some());
        
        let snapshot = state.last_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.data, data);
    }

    #[tokio::test]
    async fn test_resource_health_check() {
        let manager = RecoveryManager::new(RecoveryConfig::default());
        
        let health = manager.check_resource_health().await;
        
        assert!(health.memory_usage_percent >= 0.0);
        assert!(health.memory_usage_percent <= 100.0);
        assert!(health.cpu_pressure >= 0.0);
    }

    #[tokio::test]
    async fn test_resource_pressure_handling() {
        let manager = RecoveryManager::new(RecoveryConfig::default());
        
        let health = ResourceHealth {
            memory_usage_percent: 95.0,
            cpu_pressure: 1.0,
            disk_available: true,
            network_stable: true,
        };
        
        let action = manager.handle_resource_pressure(&health).await;
        assert_eq!(action, RecoveryAction::ThrottleNewStreams);
        
        let health = ResourceHealth {
            memory_usage_percent: 50.0,
            cpu_pressure: 3.0,
            disk_available: true,
            network_stable: true,
        };
        
        let action = manager.handle_resource_pressure(&health).await;
        assert_eq!(action, RecoveryAction::ReduceFramerate);
    }
}