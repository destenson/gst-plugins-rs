# PRP-25: RTSP Server and Proxy Implementation

## Overview
Implement RTSP server functionality to proxy streams and serve processed/recorded content to RTSP clients.

## Context
- Need to replace MediaMTX RTSP proxy functionality
- Must serve both live and recorded streams
- Should support multiple client connections
- Need authentication and access control

## Requirements
1. Create RTSP server using gst-rtsp-server
2. Implement stream proxying
3. Add authentication mechanism
4. Support multiple client sessions
5. Handle RTSP commands properly

## Implementation Tasks
1. Create src/rtsp/server.rs module
2. Define RtspServer struct:
   - GstRTSPServer instance
   - Mount points registry
   - Session pool
   - Auth manager
3. Setup RTSP server:
   - Configure bind address/port
   - Create server mainloop
   - Setup mount point factory
   - Initialize auth backend
4. Implement stream mounting:
   - Mount live streams at /live/{stream_id}
   - Mount recordings at /playback/{stream_id}
   - Dynamic mount/unmount
   - Generate SDP from caps
5. Add proxy functionality:
   - Accept incoming RTSP sources
   - Create pipeline from source
   - Serve to multiple clients
   - Handle source disconnection
6. Implement authentication:
   - Basic auth support
   - Token-based auth
   - Per-stream permissions
   - Client IP filtering
7. Handle RTSP methods:
   - OPTIONS, DESCRIBE
   - SETUP, PLAY, PAUSE
   - TEARDOWN
   - GET_PARAMETER, SET_PARAMETER

## Validation Gates
```bash
# Test RTSP server
cargo test --package stream-manager rtsp::server::tests

# Verify stream mounting
cargo test rtsp_mount_points

# Check client connections
ffplay rtsp://localhost:8554/live/test
```

## Dependencies
- PRP-09: StreamManager for stream access
- PRP-06: Tee branch for RTSP output

## References
- gst-rtsp-server: https://gstreamer.freedesktop.org/documentation/gst-rtsp-server/
- RTSP spec: RFC 7826
- Example code: gst-rtsp-server examples

## Success Metrics
- RTSP server accepts connections
- Streams accessible via RTSP
- Multiple clients supported
- Authentication works

**Confidence Score: 6/10**