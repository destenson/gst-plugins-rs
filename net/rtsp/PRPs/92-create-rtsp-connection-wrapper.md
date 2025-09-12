# PRP-RTSP-92: Create RTSPConnection Wrapper Module

## Overview
Create a new module that wraps GStreamer's RTSP connection functionality, providing a clean interface that can replace the current Tokio-based transport layer.

## Context
Based on research from PRPs 90-91, create a wrapper around GStreamer's RTSP client library that:
- Uses GIO for async I/O (no Tokio)
- Integrates with GStreamer's MainLoop
- Provides connection management
- Handles TCP/UDP transports
- Supports TLS configuration

## Research Context
- GstRTSPConnection API: https://gstreamer.freedesktop.org/documentation/gstreamer-rtsp/gstrtsconnection.html
- GIO async in Rust: https://gtk-rs.org/gtk-rs-core/stable/latest/docs/gio/
- Reference: `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c` lines 5800-6000

## Scope
This PRP ONLY covers:
1. Create new module `rtsp_connection.rs`
2. Define connection wrapper struct
3. Implement connection establishment
4. Add message send/receive methods
5. Integrate with GIO MainContext

Does NOT include:
- Removing existing Tokio code
- Modifying imp.rs
- Full RTSP protocol implementation

## Implementation Tasks
1. Create `net/rtsp/src/rtspsrc/rtsp_connection.rs`
2. Define `RtspConnection` wrapper struct
3. Implement connection methods:
   - new() - create connection
   - connect() - establish connection
   - send() - send RTSP message
   - receive() - receive RTSP message
   - close() - cleanup
4. Add GIO MainContext integration
5. Implement timeout handling
6. Add TLS configuration support
7. Create unit tests

## Module Structure
```
rtsp_connection.rs
  - RtspConnection struct
  - Connection state enum
  - Message handling
  - GIO callbacks
  - Error types
```

## Integration Points
- Uses gstreamer-rtsp crate types
- Integrates with GIO MainLoop
- Provides futures-compatible API if needed
- Supports existing Settings struct

## Validation Gates
```bash
# Build check
cargo build -p gst-plugin-rtsp

# Module tests
cargo test -p gst-plugin-rtsp rtsp_connection

# Check no Tokio usage
! grep -q "use tokio" net/rtsp/src/rtspsrc/rtsp_connection.rs
```

## Expected Behavior
- Connection establishment without Tokio
- Message exchange using GIO async
- Proper error handling
- TLS support through properties

## Success Criteria
- [ ] Module compiles without Tokio
- [ ] Basic connection test passes
- [ ] GIO MainLoop integration works
- [ ] TLS properties configurable
- [ ] No thread safety issues

## Risk Assessment
**MEDIUM RISK** - New module creation with GIO integration complexity.

## Estimated Effort
3-4 hours

## Confidence Score
7/10 - GIO async patterns need careful implementation