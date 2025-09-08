// OpenTelemetry implementation
// TODO: Implement in PRP-23

#[cfg(feature = "telemetry")]
pub struct TelemetryManager {}

#[cfg(feature = "telemetry")]
impl TelemetryManager {
    pub fn new() -> Self {
        Self {}
    }
}