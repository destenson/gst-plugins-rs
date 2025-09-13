# PRP: Enhanced Stability with Native Keep-Alive and Recovery

## Overview
Leverage RTSPConnection's built-in keep-alive, timeout management, and connection recovery mechanisms to improve stability and reduce custom error handling code.

## Background
Current implementation has extensive custom error recovery, retry logic, and keep-alive management spread across multiple modules. RTSPConnection provides battle-tested implementations of these features that are used throughout the GStreamer ecosystem.

## Requirements
- Use native keep-alive instead of custom implementation
- Leverage built-in timeout and reconnection handling
- Simplify error recovery using RTSPResult patterns
- Maintain current retry and backoff behavior
- Improve connection stability metrics

## Technical Context
RTSPConnection stability features:
- `next_timeout()` - Get next keep-alive timeout
- `reset_timeout()` - Reset keep-alive timer
- `set_remember_session_id()` - Session persistence
- `flush()` - Clear pending data on error
- `poll()` - Event monitoring with timeout
- Built-in RTCP-based keep-alive

Current implementation:
- Custom retry logic in retry.rs
- Error recovery in error_recovery.rs
- Manual keep-alive via RTCP or OPTIONS
- Complex timeout management

## Implementation Tasks
1. Replace custom keep-alive with RTSPConnection timeouts
2. Use next_timeout() for scheduling keep-alive
3. Implement reset_timeout() on message activity
4. Simplify retry logic using RTSPResult errors
5. Use flush() for connection recovery
6. Enable session ID persistence
7. Implement poll() for connection monitoring
8. Update error mapping to preserve context
9. Remove redundant error recovery code

## Testing Approach
- Long-running stability tests
- Network interruption recovery tests
- Keep-alive timeout validation
- Session persistence tests

## Validation Gates
```bash
# Build and test
cargo build --package gst-plugin-rtsp --all-features
cargo clippy --package gst-plugin-rtsp -- -D warnings

# Stability tests
cargo test --package gst-plugin-rtsp stability
cargo test --package gst-plugin-rtsp keep_alive

# Long-running stress test
cargo test --package gst-plugin-rtsp --features integration stress_recovery
```

## Success Metrics
- Improved connection uptime (>99% for stable networks)
- Faster recovery from network interruptions
- Reduced code complexity in error handling
- Lower CPU usage from keep-alive management

## Dependencies
- RTSPConnection foundation
- MainLoop integration for timeouts
- Existing telemetry/metrics infrastructure

## Risk Mitigation
- Gradual migration of error handling
- Extensive logging of recovery events
- Maintain metrics for before/after comparison
- Keep custom recovery as fallback option

## References
- RTSPConnection timeout methods
- RFC 2326 Section 10.1 (Keep-Alive)
- Current implementation: `net/rtsp/src/rtspsrc/error_recovery.rs`

## Confidence Score: 9/10
Native implementation is battle-tested. Significant code simplification and reliability improvement expected.