#![allow(unused)]
// Telemetry module for RTSP source element
// Provides structured logging, metrics collection, and performance tracking

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{event, info_span, span, Level, Span};

#[cfg(feature = "prometheus")]
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, GaugeVec,
    HistogramVec,
};

/// Prometheus metrics for RTSP
#[cfg(feature = "prometheus")]
#[derive(Debug)]
pub struct PrometheusMetrics {
    pub connection_attempts: CounterVec,
    pub connection_successes: CounterVec,
    pub connection_failures: CounterVec,
    pub retry_count: CounterVec,
    pub retry_strategy_changes: CounterVec,
    pub connection_recovery_time: HistogramVec,
    pub auto_mode_pattern: GaugeVec,
    pub adaptive_confidence: GaugeVec,
    pub packets_received: CounterVec,
    pub packets_lost: CounterVec,
    pub bytes_received: CounterVec,
    pub jitter_gauge: GaugeVec,
    pub connection_duration: HistogramVec,
    pub rtcp_packets: CounterVec,
    pub errors: CounterVec,
}

#[cfg(feature = "prometheus")]
impl PrometheusMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        Ok(Self {
            connection_attempts: register_counter_vec!(
                "rtsp_connection_attempts_total",
                "Total number of RTSP connection attempts",
                &["url", "element_id"]
            )?,
            connection_successes: register_counter_vec!(
                "rtsp_connection_successes_total",
                "Total number of successful RTSP connections",
                &["url", "element_id"]
            )?,
            connection_failures: register_counter_vec!(
                "rtsp_connection_failures_total",
                "Total number of failed RTSP connections",
                &["url", "element_id", "reason"]
            )?,
            retry_count: register_counter_vec!(
                "rtsp_retry_attempts_total",
                "Total number of retry attempts",
                &["strategy", "element_id"]
            )?,
            retry_strategy_changes: register_counter_vec!(
                "rtsp_strategy_changes_total",
                "Total number of retry strategy changes",
                &["element_id", "reason"]
            )?,
            connection_recovery_time: register_histogram_vec!(
                "rtsp_connection_recovery_seconds",
                "Time to recover connection after failure",
                &["element_id"]
            )?,
            auto_mode_pattern: register_gauge_vec!(
                "rtsp_auto_mode_pattern",
                "Current auto mode network pattern (0=unknown, 1=stable, 2=limited, 3=lossy)",
                &["element_id"]
            )?,
            adaptive_confidence: register_gauge_vec!(
                "rtsp_adaptive_confidence_score",
                "Adaptive learning confidence score (0.0-1.0)",
                &["element_id"]
            )?,
            packets_received: register_counter_vec!(
                "rtsp_packets_received_total",
                "Total number of packets received",
                &["stream_id", "element_id"]
            )?,
            packets_lost: register_counter_vec!(
                "rtsp_packets_lost_total",
                "Total number of packets lost",
                &["stream_id", "element_id"]
            )?,
            bytes_received: register_counter_vec!(
                "rtsp_bytes_received_total",
                "Total number of bytes received",
                &["stream_id", "element_id"]
            )?,
            jitter_gauge: register_gauge_vec!(
                "rtsp_jitter_milliseconds",
                "Current jitter in milliseconds",
                &["stream_id", "element_id"]
            )?,
            connection_duration: register_histogram_vec!(
                "rtsp_connection_duration_seconds",
                "Connection establishment duration in seconds",
                &["url", "element_id"]
            )?,
            rtcp_packets: register_counter_vec!(
                "rtsp_rtcp_packets_total",
                "Total number of RTCP packets",
                &["direction", "element_id"]
            )?,
            errors: register_counter_vec!(
                "rtsp_errors_total",
                "Total number of errors",
                &["type", "element_id"]
            )?,
        })
    }
}

/// Metrics collector for RTSP connection statistics
#[derive(Debug, Clone)]
pub struct RtspMetrics {
    inner: Arc<MetricsInner>,
    #[cfg(feature = "prometheus")]
    prometheus: Option<Arc<PrometheusMetrics>>,
}

#[derive(Debug)]
struct MetricsInner {
    // Connection metrics
    connection_attempts: AtomicU64,
    connection_successes: AtomicU64,
    connection_failures: AtomicU64,

    // Retry metrics
    retry_count: AtomicU64,
    retry_strategy_changes: AtomicU64,
    retry_attempts_by_strategy: Arc<parking_lot::RwLock<std::collections::HashMap<String, u64>>>,
    connection_recovery_time_ms: AtomicU64,
    auto_mode_pattern: AtomicU64, // 0=unknown, 1=stable, 2=limited, 3=lossy
    adaptive_confidence_score: AtomicU64, // Stored as percentage * 100 (0-10000 for 0.0-100.0)

    // Performance metrics
    total_packets_received: AtomicU64,
    total_packets_lost: AtomicU64,
    total_bytes_received: AtomicU64,

    // Timing metrics
    last_connection_time_ms: AtomicU64,
    average_jitter_ms: AtomicU64,

    // RTCP statistics
    rtcp_packets_sent: AtomicU64,
    rtcp_packets_received: AtomicU64,

    // Error tracking
    network_errors: AtomicU64,
    protocol_errors: AtomicU64,
    timeout_errors: AtomicU64,

    // Element identification
    element_id: String,
    url: String,
}

impl Default for RtspMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl RtspMetrics {
    pub fn new() -> Self {
        Self::with_id("unknown", "")
    }

    pub fn with_id(element_id: &str, url: &str) -> Self {
        #[cfg(feature = "prometheus")]
        let prometheus = PrometheusMetrics::new().ok().map(Arc::new);

        Self {
            inner: Arc::new(MetricsInner {
                connection_attempts: AtomicU64::new(0),
                connection_successes: AtomicU64::new(0),
                connection_failures: AtomicU64::new(0),
                retry_count: AtomicU64::new(0),
                retry_strategy_changes: AtomicU64::new(0),
                retry_attempts_by_strategy: Arc::new(RwLock::new(HashMap::new())),
                connection_recovery_time_ms: AtomicU64::new(0),
                auto_mode_pattern: AtomicU64::new(0),
                adaptive_confidence_score: AtomicU64::new(0),
                total_packets_received: AtomicU64::new(0),
                total_packets_lost: AtomicU64::new(0),
                total_bytes_received: AtomicU64::new(0),
                last_connection_time_ms: AtomicU64::new(0),
                average_jitter_ms: AtomicU64::new(0),
                rtcp_packets_sent: AtomicU64::new(0),
                rtcp_packets_received: AtomicU64::new(0),
                network_errors: AtomicU64::new(0),
                protocol_errors: AtomicU64::new(0),
                timeout_errors: AtomicU64::new(0),
                element_id: element_id.to_string(),
                url: url.to_string(),
            }),
            #[cfg(feature = "prometheus")]
            prometheus,
        }
    }

    // Connection tracking
    pub fn record_connection_attempt(&self) {
        self.inner
            .connection_attempts
            .fetch_add(1, Ordering::Relaxed);
        event!(Level::DEBUG, metric = "connection_attempt", count = 1);

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.connection_attempts
                .with_label_values(&[self.inner.url.as_str(), self.inner.element_id.as_str()])
                .inc();
        }
    }

    pub fn record_connection_success(&self, duration_ms: u64) {
        self.inner
            .connection_successes
            .fetch_add(1, Ordering::Relaxed);
        self.inner
            .last_connection_time_ms
            .store(duration_ms, Ordering::Relaxed);
        event!(
            Level::INFO,
            metric = "connection_success",
            duration_ms = duration_ms,
            total_successes = self.inner.connection_successes.load(Ordering::Relaxed)
        );

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.connection_successes
                .with_label_values(&[self.inner.url.as_str(), self.inner.element_id.as_str()])
                .inc();
            prom.connection_duration
                .with_label_values(&[self.inner.url.as_str(), self.inner.element_id.as_str()])
                .observe(duration_ms as f64 / 1000.0);
        }
    }

    pub fn record_connection_failure(&self, reason: &str) {
        self.inner
            .connection_failures
            .fetch_add(1, Ordering::Relaxed);
        event!(
            Level::WARN,
            metric = "connection_failure",
            reason = reason,
            total_failures = self.inner.connection_failures.load(Ordering::Relaxed)
        );

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.connection_failures
                .with_label_values(&[
                    self.inner.url.as_str(),
                    self.inner.element_id.as_str(),
                    reason,
                ])
                .inc();
        }
    }

    // Retry tracking
    pub fn record_retry(&self, strategy: &str) {
        self.inner.retry_count.fetch_add(1, Ordering::Relaxed);
        event!(
            Level::DEBUG,
            metric = "retry_attempt",
            strategy = strategy,
            retry_count = self.inner.retry_count.load(Ordering::Relaxed)
        );

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.retry_count
                .with_label_values(&[strategy, self.inner.element_id.as_str()])
                .inc();
        }
    }

    pub fn record_retry_strategy_change(
        &self,
        old_strategy: &str,
        new_strategy: &str,
        reason: Option<&str>,
    ) {
        self.inner
            .retry_strategy_changes
            .fetch_add(1, Ordering::Relaxed);
        let reason_str = reason.unwrap_or("manual");
        event!(
            Level::INFO,
            metric = "retry_strategy_change",
            old_strategy = old_strategy,
            new_strategy = new_strategy,
            reason = reason_str
        );

        // Track attempts per strategy
        {
            let mut attempts = self.inner.retry_attempts_by_strategy.write();
            *attempts.entry(new_strategy.to_string()).or_insert(0) += 1;
        }

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.retry_strategy_changes
                .with_label_values(&[self.inner.element_id.as_str(), reason_str])
                .inc();
        }
    }

    /// Record connection recovery time (time from first failure to successful reconnection)
    pub fn record_connection_recovery(&self, recovery_time_ms: u64) {
        self.inner
            .connection_recovery_time_ms
            .store(recovery_time_ms, Ordering::Relaxed);
        event!(
            Level::INFO,
            metric = "connection_recovery",
            recovery_time_ms = recovery_time_ms
        );

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.connection_recovery_time
                .with_label_values(&[self.inner.element_id.as_str()])
                .observe(recovery_time_ms as f64 / 1000.0);
        }
    }

    /// Record auto mode pattern detection
    pub fn record_auto_mode_pattern(&self, pattern: &str) {
        let pattern_value = match pattern {
            "stable" => 1,
            "limited" => 2,
            "lossy" => 3,
            _ => 0, // unknown
        };
        self.inner
            .auto_mode_pattern
            .store(pattern_value, Ordering::Relaxed);
        event!(
            Level::DEBUG,
            metric = "auto_mode_pattern",
            pattern = pattern
        );

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.auto_mode_pattern
                .with_label_values(&[self.inner.element_id.as_str()])
                .set(pattern_value as f64);
        }
    }

    /// Record adaptive learning confidence score
    pub fn record_adaptive_confidence(&self, confidence: f64) {
        // Store as integer (confidence * 100)
        let confidence_int = (confidence * 100.0) as u64;
        self.inner
            .adaptive_confidence_score
            .store(confidence_int, Ordering::Relaxed);
        event!(
            Level::DEBUG,
            metric = "adaptive_confidence",
            confidence = confidence
        );

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.adaptive_confidence
                .with_label_values(&[self.inner.element_id.as_str()])
                .set(confidence);
        }
    }

    /// Get retry attempts by strategy
    pub fn get_retry_attempts_by_strategy(&self) -> HashMap<String, u64> {
        self.inner.retry_attempts_by_strategy.read().clone()
    }

    /// Get current auto mode pattern
    pub fn get_auto_mode_pattern(&self) -> String {
        match self.inner.auto_mode_pattern.load(Ordering::Relaxed) {
            1 => "stable".to_string(),
            2 => "limited".to_string(),
            3 => "lossy".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Get adaptive confidence score as float
    pub fn get_adaptive_confidence(&self) -> f64 {
        self.inner.adaptive_confidence_score.load(Ordering::Relaxed) as f64 / 100.0
    }

    // Packet tracking
    pub fn record_packets_received(&self, count: u64, bytes: u64) {
        self.inner
            .total_packets_received
            .fetch_add(count, Ordering::Relaxed);
        self.inner
            .total_bytes_received
            .fetch_add(bytes, Ordering::Relaxed);

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.packets_received
                .with_label_values(&["0", self.inner.element_id.as_str()])
                .inc_by(count as f64);
            prom.bytes_received
                .with_label_values(&["0", self.inner.element_id.as_str()])
                .inc_by(bytes as f64);
        }
    }

    pub fn record_packet_loss(&self, count: u64) {
        self.inner
            .total_packets_lost
            .fetch_add(count, Ordering::Relaxed);
        let total_received = self.inner.total_packets_received.load(Ordering::Relaxed);
        let total_lost = self.inner.total_packets_lost.load(Ordering::Relaxed);

        if total_received > 0 {
            let loss_rate = (total_lost as f64 / (total_received + total_lost) as f64) * 100.0;
            event!(
                Level::WARN,
                metric = "packet_loss",
                lost_packets = count,
                total_lost = total_lost,
                loss_rate_percent = loss_rate
            );
        }

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.packets_lost
                .with_label_values(&["0", self.inner.element_id.as_str()])
                .inc_by(count as f64);
        }
    }

    // Jitter tracking
    pub fn update_jitter(&self, jitter_ms: u64) {
        let current = self.inner.average_jitter_ms.load(Ordering::Relaxed);
        // Simple exponential moving average
        let new_avg = if current == 0 {
            jitter_ms
        } else {
            (current * 7 + jitter_ms * 3) / 10
        };
        self.inner
            .average_jitter_ms
            .store(new_avg, Ordering::Relaxed);

        if jitter_ms > 100 {
            event!(
                Level::WARN,
                metric = "high_jitter",
                jitter_ms = jitter_ms,
                average_jitter_ms = new_avg
            );
        }
    }

    // RTCP tracking
    pub fn record_rtcp_sent(&self) {
        self.inner.rtcp_packets_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_rtcp_received(&self) {
        self.inner
            .rtcp_packets_received
            .fetch_add(1, Ordering::Relaxed);
    }

    // Error tracking
    pub fn record_network_error(&self) {
        self.inner.network_errors.fetch_add(1, Ordering::Relaxed);
        event!(
            Level::ERROR,
            metric = "network_error",
            total_network_errors = self.inner.network_errors.load(Ordering::Relaxed)
        );

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.errors
                .with_label_values(&["network", self.inner.element_id.as_str()])
                .inc();
        }
    }

    pub fn record_protocol_error(&self) {
        self.inner.protocol_errors.fetch_add(1, Ordering::Relaxed);
        event!(
            Level::ERROR,
            metric = "protocol_error",
            total_protocol_errors = self.inner.protocol_errors.load(Ordering::Relaxed)
        );

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.errors
                .with_label_values(&["protocol", self.inner.element_id.as_str()])
                .inc();
        }
    }

    pub fn record_timeout_error(&self) {
        self.inner.timeout_errors.fetch_add(1, Ordering::Relaxed);
        event!(
            Level::WARN,
            metric = "timeout_error",
            total_timeout_errors = self.inner.timeout_errors.load(Ordering::Relaxed)
        );

        #[cfg(feature = "prometheus")]
        if let Some(ref prom) = self.prometheus {
            prom.errors
                .with_label_values(&["timeout", self.inner.element_id.as_str()])
                .inc();
        }
    }

    // Get current metrics as properties
    pub fn get_metrics_summary(&self) -> MetricsSummary {
        MetricsSummary {
            connection_attempts: self.inner.connection_attempts.load(Ordering::Relaxed),
            connection_successes: self.inner.connection_successes.load(Ordering::Relaxed),
            connection_failures: self.inner.connection_failures.load(Ordering::Relaxed),
            retry_count: self.inner.retry_count.load(Ordering::Relaxed),
            total_packets_received: self.inner.total_packets_received.load(Ordering::Relaxed),
            total_packets_lost: self.inner.total_packets_lost.load(Ordering::Relaxed),
            total_bytes_received: self.inner.total_bytes_received.load(Ordering::Relaxed),
            average_jitter_ms: self.inner.average_jitter_ms.load(Ordering::Relaxed),
            rtcp_packets_sent: self.inner.rtcp_packets_sent.load(Ordering::Relaxed),
            rtcp_packets_received: self.inner.rtcp_packets_received.load(Ordering::Relaxed),
            network_errors: self.inner.network_errors.load(Ordering::Relaxed),
            protocol_errors: self.inner.protocol_errors.load(Ordering::Relaxed),
            timeout_errors: self.inner.timeout_errors.load(Ordering::Relaxed),
        }
    }

    pub fn reset(&self) {
        self.inner.connection_attempts.store(0, Ordering::Relaxed);
        self.inner.connection_successes.store(0, Ordering::Relaxed);
        self.inner.connection_failures.store(0, Ordering::Relaxed);
        self.inner.retry_count.store(0, Ordering::Relaxed);
        self.inner
            .retry_strategy_changes
            .store(0, Ordering::Relaxed);
        self.inner
            .total_packets_received
            .store(0, Ordering::Relaxed);
        self.inner.total_packets_lost.store(0, Ordering::Relaxed);
        self.inner.total_bytes_received.store(0, Ordering::Relaxed);
        self.inner
            .last_connection_time_ms
            .store(0, Ordering::Relaxed);
        self.inner.average_jitter_ms.store(0, Ordering::Relaxed);
        self.inner.rtcp_packets_sent.store(0, Ordering::Relaxed);
        self.inner.rtcp_packets_received.store(0, Ordering::Relaxed);
        self.inner.network_errors.store(0, Ordering::Relaxed);
        self.inner.protocol_errors.store(0, Ordering::Relaxed);
        self.inner.timeout_errors.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub connection_attempts: u64,
    pub connection_successes: u64,
    pub connection_failures: u64,
    pub retry_count: u64,
    pub total_packets_received: u64,
    pub total_packets_lost: u64,
    pub total_bytes_received: u64,
    pub average_jitter_ms: u64,
    pub rtcp_packets_sent: u64,
    pub rtcp_packets_received: u64,
    pub network_errors: u64,
    pub protocol_errors: u64,
    pub timeout_errors: u64,
}

/// Helper for creating tracing spans
pub struct SpanHelper;

impl SpanHelper {
    pub fn connection_span(url: &str) -> Span {
        info_span!(
            "rtsp_connection",
            url = %url,
            otel.kind = "client",
            otel.status_code = tracing::field::Empty,
        )
    }

    pub fn retry_span(strategy: &str, attempt: u32) -> Span {
        info_span!(
            "retry_attempt",
            strategy = %strategy,
            attempt = attempt,
        )
    }

    pub fn setup_span() -> Span {
        info_span!("rtsp_setup")
    }

    pub fn play_span() -> Span {
        info_span!("rtsp_play")
    }

    pub fn teardown_span() -> Span {
        info_span!("rtsp_teardown")
    }

    pub fn packet_processing_span(stream_id: u32) -> Span {
        span!(Level::TRACE, "packet_processing", stream_id = stream_id,)
    }
}

/// Timer for measuring operation durations
pub struct OperationTimer {
    name: String,
    start: Instant,
    threshold_ms: u64,
}

impl OperationTimer {
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_threshold(name, 100)
    }

    pub fn with_threshold(name: impl Into<String>, threshold_ms: u64) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
            threshold_ms,
        }
    }
}

impl Drop for OperationTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        let duration_ms = duration.as_millis() as u64;

        if duration_ms > self.threshold_ms {
            event!(
                Level::WARN,
                operation = %self.name,
                duration_ms = duration_ms,
                threshold_ms = self.threshold_ms,
                "Operation exceeded threshold"
            );
        } else {
            event!(
                Level::TRACE,
                operation = %self.name,
                duration_ms = duration_ms,
                "Operation completed"
            );
        }
    }
}

/// Initialize telemetry subsystem
pub fn init_telemetry() {
    // Initialize tracing subscriber if not already initialized
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rtspsrc2=debug".parse().unwrap()),
        )
        .try_init();

    // Initialize tokio-console if feature is enabled
    #[cfg(feature = "tokio-console")]
    {
        console_subscriber::init();
        event!(Level::INFO, "tokio-console subscriber initialized");
    }

    event!(Level::INFO, "RTSP telemetry initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = RtspMetrics::new();

        // Test connection metrics
        metrics.record_connection_attempt();
        metrics.record_connection_success(150);
        assert_eq!(metrics.inner.connection_attempts.load(Ordering::Relaxed), 1);
        assert_eq!(
            metrics.inner.connection_successes.load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            metrics
                .inner
                .last_connection_time_ms
                .load(Ordering::Relaxed),
            150
        );

        // Test retry metrics
        metrics.record_retry("exponential");
        assert_eq!(metrics.inner.retry_count.load(Ordering::Relaxed), 1);

        // Test packet metrics
        metrics.record_packets_received(100, 50000);
        assert_eq!(
            metrics.inner.total_packets_received.load(Ordering::Relaxed),
            100
        );
        assert_eq!(
            metrics.inner.total_bytes_received.load(Ordering::Relaxed),
            50000
        );

        // Test jitter
        metrics.update_jitter(50);
        assert_eq!(metrics.inner.average_jitter_ms.load(Ordering::Relaxed), 50);
        metrics.update_jitter(70);
        // Should be (50 * 7 + 70 * 3) / 10 = 56
        assert_eq!(metrics.inner.average_jitter_ms.load(Ordering::Relaxed), 56);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = RtspMetrics::new();

        metrics.record_connection_attempt();
        metrics.record_packets_received(100, 50000);
        metrics.record_network_error();

        metrics.reset();

        assert_eq!(metrics.inner.connection_attempts.load(Ordering::Relaxed), 0);
        assert_eq!(
            metrics.inner.total_packets_received.load(Ordering::Relaxed),
            0
        );
        assert_eq!(metrics.inner.network_errors.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_metrics_summary() {
        let metrics = RtspMetrics::new();

        metrics.record_connection_attempt();
        metrics.record_connection_success(100);
        metrics.record_packets_received(50, 25000);

        let summary = metrics.get_metrics_summary();
        assert_eq!(summary.connection_attempts, 1);
        assert_eq!(summary.connection_successes, 1);
        assert_eq!(summary.total_packets_received, 50);
        assert_eq!(summary.total_bytes_received, 25000);
    }
}
