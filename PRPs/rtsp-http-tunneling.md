# PRP: Native HTTP Tunneling Support

## Overview
Replace the custom HTTP tunneling implementation with RTSPConnection's native HTTP tunnel support, providing robust RTSP-over-HTTP for firewall traversal.

## Background
Current implementation has a custom http_tunnel.rs module that manually handles HTTP tunneling. RTSPConnection provides built-in HTTP tunneling with proper session management and protocol handling.

## Requirements
- Support RTSP-over-HTTP tunneling 
- Maintain compatibility with existing proxy settings
- Handle both GET and POST tunnel connections
- Support custom HTTP headers for tunneling
- Preserve current http-tunnel property behavior

## Technical Context
RTSPConnection HTTP tunneling:
- `set_http_mode()` - Enable HTTP mode
- `do_tunnel()` - Link two connections for tunneling
- `set_tunneled()` - Mark connection as tunneled
- `get_tunnelid()` - Retrieve tunnel session ID
- `add_extra_http_request_header()` - Custom headers
- `connect_with_response()` - Handle tunnel setup

Current implementation:
- Custom HTTP CONNECT handling in http_tunnel.rs
- Manual session ID management
- Custom header injection

## Implementation Tasks
1. Replace http_tunnel.rs with RTSPConnection methods
2. Update connection setup for HTTP tunnel mode
3. Implement dual connection (GET/POST) setup
4. Use do_tunnel() to link connections
5. Handle tunnel session IDs properly
6. Support extra HTTP headers via RTSPConnection
7. Update proxy integration for tunneling
8. Implement tunnel-specific error handling

## Testing Approach
- Test with HTTP proxy servers
- Verify GET/POST tunnel establishment
- Test header passthrough
- Validate session management

## Validation Gates
```bash
# Build and test
cargo build --package gst-plugin-rtsp --all-features

# HTTP tunnel tests
cargo test --package gst-plugin-rtsp http_tunnel

# Proxy integration tests
cargo test --package gst-plugin-rtsp proxy_tunnel
```

## Success Metrics
- HTTP tunneling works with common proxies
- Session management handles reconnections
- Custom headers properly transmitted
- Performance comparable to direct connection

## Dependencies
- RTSPConnection foundation
- Proxy configuration support
- MainLoop for async tunnel setup

## Risk Mitigation
- Test with various proxy implementations
- Add detailed tunnel debugging logs
- Fallback to direct connection on tunnel failure
- Support both strict and lenient proxy modes

## References
- Apple HTTP tunneling spec (QuickTime)
- RTSPConnection tunnel methods
- Current implementation: `net/rtsp/src/rtspsrc/http_tunnel.rs`

## Confidence Score: 9/10
Native implementation is more robust than custom code. Well-tested in GStreamer ecosystem.