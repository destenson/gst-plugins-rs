# PRP: GIO Socket Management for UDP/TCP Operations

## Overview
Replace all Tokio socket operations (TCP and UDP) with GIO sockets, providing native GStreamer-compatible networking that integrates with RTSPConnection.

## Background
Currently using tokio::net::TcpStream and UdpSocket for network operations. GIO provides GSocket with full async support, proper cancellation, and integration with GStreamer's networking stack.

## Requirements
- Replace all Tokio TCP sockets with gio::Socket
- Replace all Tokio UDP sockets with gio::Socket
- Implement UDP multicast using GIO
- Support both IPv4 and IPv6 with GIO
- Maintain socket pooling and reuse patterns

## Technical Context
GIO socket features:
- `gio::Socket::new()` - Create TCP/UDP sockets
- `socket.connect()` - TCP connection
- `socket.bind()` - Bind to address
- `socket.send_to()`, `socket.receive_from()` - UDP operations
- `socket.join_multicast_group()` - Multicast support
- `socket.create_source()` - MainLoop integration

Current Tokio usage:
- TCP streams for RTSP control connection
- UDP sockets for RTP/RTCP data
- Multicast socket configuration
- Socket pool in connection_pool.rs

## Implementation Tasks
1. Create GIO socket wrapper for common operations
2. Replace TCP connection with gio::Socket
3. Implement UDP socket creation and binding
4. Add multicast group management with GIO
5. Create socket source for MainLoop integration
6. Implement socket pooling with GIO sockets
7. Add proper socket cleanup and shutdown
8. Handle platform-specific socket options
9. Implement non-blocking mode configuration
10. Create socket address resolution with GIO

## Testing Approach
- TCP connection establishment tests
- UDP send/receive verification
- Multicast join/leave testing
- Socket reuse and pooling tests
- IPv4/IPv6 dual-stack testing

## Validation Gates
```bash
# Build without Tokio
cargo build --package gst-plugin-rtsp --no-default-features

# Socket operation tests
cargo test --package gst-plugin-rtsp socket

# Network stress tests
cargo test --package gst-plugin-rtsp network_stress

# Multicast functionality
cargo test --package gst-plugin-rtsp multicast
```

## Success Metrics
- All socket operations work correctly
- Multicast reception functions properly
- No socket descriptor leaks
- Performance matches Tokio sockets

## Dependencies
- GIO socket bindings
- GLib MainContext for event handling
- Platform-specific socket headers

## Risk Mitigation
- Comprehensive socket wrapper abstraction
- Platform-specific testing (Linux/Windows/macOS)
- Socket leak detection in tests
- Fallback to blocking operations if needed

## References
- GIO Socket API: https://docs.gtk.org/gio/class.Socket.html
- GStreamer udpsrc/udpsink socket usage
- Current implementation: net/rtsp/src/rtspsrc/transport.rs

## Confidence Score: 8/10
Well-documented GIO API. Main complexity in preserving all socket options and platform compatibility.