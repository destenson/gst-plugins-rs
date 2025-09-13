# PRP: RTSP Connection Foundation Migration

## Overview
Replace the current Tokio-based TCP connection handling in rtspsrc2 with the new GStreamer RTSPConnection bindings. This forms the foundation for all subsequent RTSP improvements.

## Background
The current rtspsrc2 implementation uses Tokio for all network operations, including TCP connections, socket management, and async I/O. The new RTSPConnection bindings provide native GStreamer RTSP protocol support with built-in connection management, automatic keep-alive, and better error recovery.

## Requirements
- Replace Tokio TCP streams with RTSPConnection for RTSP communication
- Maintain backward compatibility with existing properties and signals  
- Preserve current connection racing and retry mechanisms
- Integrate with existing buffer pool and session management

## Technical Context
The RTSPConnection API provides:
- Connection creation from URL: `RTSPConnection::create(&url)`
- Connect with timeout: `conn.connect(timeout_secs)`
- Send/receive RTSP messages: `conn.send(&msg)`, `conn.receive(&msg)`
- Built-in keep-alive: `conn.next_timeout()`, `conn.reset_timeout()`
- TLS support: `conn.set_tls_database()`, `conn.set_tls_validation_flags()`

Current implementation to replace is in:
- `net/rtsp/src/rtspsrc/imp.rs` - RtspManager and connection handling
- `net/rtsp/src/rtspsrc/connection_pool.rs` - Connection pooling logic
- `net/rtsp/src/rtspsrc/tcp_message.rs` - TCP message framing

## Implementation Tasks
1. Add gstreamer-rtsp dependency pointing to local bindings
2. Create RTSPConnection wrapper that matches current connection interface
3. Replace TCP stream creation with RTSPConnection::create()
4. Migrate send_request/receive_response to use RTSPConnection methods
5. Update connection pool to manage RTSPConnection instances
6. Implement connection state tracking using RTSPConnection
7. Update error types to handle RTSPResult errors
8. Preserve connection racing logic with RTSPConnection

## Testing Approach
- Unit tests for RTSPConnection wrapper
- Integration tests with mock RTSP server
- Compatibility tests with existing test suite
- Performance comparison with Tokio version

## Validation Gates
```bash
# Build and format check
cargo build --package gst-plugin-rtsp --all-features
cargo fmt --package gst-plugin-rtsp -- --check
cargo clippy --package gst-plugin-rtsp --all-features -- -D warnings

# Run existing tests to ensure compatibility
cargo test --package gst-plugin-rtsp connection

# Integration test with real server
cargo test --package gst-plugin-rtsp gst_server_integration
```

## Success Metrics
- All existing connection tests pass
- Connection establishment time <= current implementation
- Memory usage comparable to Tokio version
- Successfully connects to test RTSP servers

## Dependencies
- Local gstreamer-rs with RTSPConnection bindings
- Existing session_manager and buffer_pool modules

## Risk Mitigation
- Keep Tokio connection code initially, use feature flag to switch
- Implement comprehensive error mapping from RTSPResult
- Add detailed logging for connection state transitions

## References
- RTSPConnection documentation: https://gstreamer.freedesktop.org/documentation/rtsplib/gstrtspconnection.html
- Current connection implementation: `net/rtsp/src/rtspsrc/imp.rs:4313-4500`
- Connection pool pattern: `net/rtsp/src/rtspsrc/connection_pool.rs`

## Confidence Score: 8/10
Strong foundation with clear API mapping. Main complexity is preserving existing behavior while switching underlying implementation.