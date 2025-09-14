# PRP-RTSP-31: Integrate Telemetry with Retry Metrics

## Overview
Telemetry system exists but doesn't track retry-specific metrics. This PRP adds comprehensive retry pattern tracking to enable monitoring and debugging of connection issues.

## Current State
- Basic telemetry tracks connections, packets, bytes
- No tracking of retry attempts, strategies, or patterns
- No visibility into auto mode decisions
- Can't correlate failures with network conditions

## Success Criteria
- [ ] Track retry attempts per strategy
- [ ] Record strategy changes and reasons
- [ ] Measure time-to-successful-connection
- [ ] Export metrics for auto mode pattern detection
- [ ] Prometheus metrics for retry patterns
- [ ] Tests verify metrics are collected correctly

## Technical Details

### New Metrics to Add
- retry_attempts_total (counter per strategy)
- strategy_changes_total (counter)
- connection_recovery_time (histogram)
- auto_mode_pattern (gauge: 0=unknown, 1=stable, 2=limited, 3=lossy)
- adaptive_confidence_score (gauge 0.0-1.0)

### Integration Points
1. RetryCalculator records to telemetry
2. Auto selector reports pattern changes
3. Adaptive manager exports confidence
4. Connection success/failure with retry context

## Implementation Blueprint
1. Extend RtspMetrics struct with retry fields
2. Add recording methods for retry events
3. Wire retry calculator to call telemetry
4. Export Prometheus metrics for retry data
5. Add trace spans for retry decisions
6. Create dashboard template for Grafana
7. Add property to query retry metrics

## Resources
- Prometheus Rust client: https://docs.rs/prometheus/latest/prometheus/
- OpenTelemetry tracing: https://docs.rs/tracing/latest/tracing/
- Grafana dashboard examples: https://grafana.com/grafana/dashboards/
- GStreamer statistics: https://gstreamer.freedesktop.org/documentation/design/statistics.html

## Validation Gates
```bash
# Test telemetry collection
cargo test -p gst-plugin-rtsp --features telemetry telemetry -- --nocapture

# Verify Prometheus metrics
curl http://localhost:9090/metrics | grep retry_

# Check trace spans
RUST_LOG=trace cargo test retry_with_telemetry
```

## Dependencies
- telemetry.rs module
- retry.rs and auto_selector.rs
- Feature flag: telemetry

## Estimated Effort
2 hours

## Risk Assessment
- Low risk - additive changes only
- Performance impact minimal (metrics are cheap)
- Must handle feature flag properly

## Success Confidence Score
8/10 - Clear integration with existing telemetry system