# PRP: Enhanced Proxy Support with Native RTSPConnection

## Overview
Upgrade proxy support using RTSPConnection's native proxy capabilities, replacing the custom proxy implementation with GStreamer's battle-tested proxy handling.

## Background
Current implementation has a proxy.rs module with custom proxy handling. RTSPConnection provides comprehensive proxy support including HTTP, SOCKS, and authentication that's been tested across many environments.

## Requirements
- Support HTTP and SOCKS proxies
- Handle proxy authentication properly
- Support proxy for both RTSP and HTTP tunneling
- Maintain current proxy property interface
- Add proxy auto-detection capabilities

## Technical Context
RTSPConnection proxy features:
- `set_proxy()` - Configure proxy server
- HTTP CONNECT method support
- Proxy authentication integration
- Works with HTTP tunneling
- Automatic proxy protocol detection

Current limitations:
- Basic HTTP proxy support only
- Limited authentication methods
- No SOCKS support
- Manual proxy protocol handling

## Implementation Tasks
1. Replace proxy.rs with RTSPConnection proxy methods
2. Map proxy properties to RTSPConnection settings
3. Implement proxy authentication callback
4. Add SOCKS proxy support detection
5. Handle proxy with HTTP tunneling
6. Implement proxy auto-detection from environment
7. Add proxy connection timeout handling
8. Create proxy-specific error messages
9. Support proxy exclusion lists
10. Add proxy performance metrics

## Testing Approach
- Test with HTTP/HTTPS proxies
- SOCKS4/SOCKS5 proxy testing
- Proxy authentication scenarios
- Proxy with tunneling combinations

## Validation Gates
```bash
# Build and test
cargo build --package gst-plugin-rtsp --no-default-features

# Proxy-specific tests
cargo test --package gst-plugin-rtsp proxy

# Integration with real proxy
HTTP_PROXY=http://localhost:8080 cargo test --package gst-plugin-rtsp proxy_integration
```

## Success Metrics
- Works with common proxy servers (Squid, nginx)
- Supports authentication methods (Basic, Digest)
- SOCKS proxy support functional
- No regression in current proxy users

## Dependencies
- RTSPConnection with proxy support
- GIO proxy resolver for auto-detection

## Risk Mitigation
- Test with various proxy implementations
- Maintain backward compatibility
- Add proxy debugging/logging
- Support proxy bypass for local addresses

## References
- RTSPConnection proxy methods
- GIO proxy support: https://docs.gtk.org/gio/iface.Proxy.html
- Current implementation: net/rtsp/src/rtspsrc/proxy.rs

## Confidence Score: 8/10
Native implementation more robust than custom code. Adds missing SOCKS support.