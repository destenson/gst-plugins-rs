pub mod monitor;

pub use monitor::{HealthMonitor, HealthState, HealthMetrics, HealthConfig};

use std::time::Duration;

// Re-export for backward compatibility
#[deprecated(note = "Use HealthMonitor from monitor module instead")]
#[allow(non_camel_case_types)]
pub struct HealthMonitor_Old {
    #[allow(dead_code)]
    check_interval: Duration,
}

#[allow(deprecated)]
impl HealthMonitor_Old {
    pub fn new(check_interval: Duration) -> Self {
        Self { check_interval }
    }
}