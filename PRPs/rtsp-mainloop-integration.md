# PRP: GLib MainLoop Integration for Async Operations

## Overview
Replace the Tokio runtime with GLib MainLoop for all async operations, enabling proper integration with GStreamer's event system and the new RTSPConnection async methods.

## Background
The current implementation spawns a Tokio runtime for async operations, which adds complexity and overhead. GStreamer provides MainLoop/MainContext for async operations that integrate better with the GStreamer ecosystem and the new RTSPConnection uses GIO async patterns.

## Requirements
- Replace Tokio runtime with GLib MainLoop
- Convert futures-based async to callback-based async
- Maintain task cancellation capabilities
- Preserve current threading model for RTCP and RTP handling

## Technical Context
GLib MainLoop patterns:
- MainLoop creation: `glib::MainLoop::new(None, false)`
- Source attachment: `glib::idle_add()`, `glib::timeout_add()`
- GIO async callbacks with RTSPConnection
- Cancellation via `gio::Cancellable`

Current Tokio usage:
- Runtime in `imp.rs` for main task loop
- Async TCP/UDP operations
- Future-based timeout handling
- Task spawning for concurrent operations

## Implementation Tasks
1. Replace Tokio runtime creation with MainLoop setup
2. Convert task loop from async fn to MainLoop sources
3. Replace tokio::select! with GSource priorities
4. Implement cancellation using gio::Cancellable
5. Convert timeout handling to g_timeout_add
6. Update RTCP timer to use MainLoop timeouts
7. Replace async mutexes with GLib thread-safe types
8. Handle element state changes with MainLoop

## Testing Approach
- Verify MainLoop starts and stops correctly
- Test cancellation during operations
- Ensure no deadlocks in state transitions
- Performance testing vs Tokio implementation

## Validation Gates
```bash
# Build and test
cargo build --package gst-plugin-rtsp --all-features
cargo test --package gst-plugin-rtsp mainloop

# State transition tests
cargo test --package gst-plugin-rtsp state_changes

# Stress test with multiple instances
cargo test --package gst-plugin-rtsp stress_mainloop
```

## Success Metrics
- Element state changes work correctly
- No resource leaks on pipeline stop
- CPU usage comparable to Tokio version
- Proper cancellation of pending operations

## Dependencies
- RTSPConnection foundation (using GIO async)
- GLib/GIO bindings

## Risk Mitigation
- Implement gradual migration with compatibility layer
- Use extensive logging for MainLoop events
- Add deadlock detection in tests
- Keep Tokio as fallback initially

## References
- GLib MainLoop docs: https://docs.gtk.org/glib/main-loop.html
- GIO async patterns: https://docs.gtk.org/gio/async-programming.html
- Current task loop: `net/rtsp/src/rtspsrc/imp.rs:4500-5000`

## Confidence Score: 7/10
Significant architectural change but well-established patterns in GStreamer. Complexity in converting async/await to callbacks.