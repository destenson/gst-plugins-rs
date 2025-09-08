// Health monitoring system
// TODO: Implement in PRP-10

use std::time::Duration;

pub struct HealthMonitor {
    check_interval: Duration,
}

impl HealthMonitor {
    pub fn new(check_interval: Duration) -> Self {
        Self { check_interval }
    }
}