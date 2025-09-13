# PRP-RTSP-11: HTTP Tunneling Support

## Overview
Implement RTSP-over-HTTP tunneling to bypass firewalls and proxies that block RTSP traffic, enabling streaming in restrictive network environments.

## Current State
- Listed as missing feature: "HTTP tunnelling"
- Cannot stream through HTTP-only firewalls
- No proxy support
- Required for many corporate networks

## Success Criteria
- [ ] Establish HTTP tunnel for RTSP
- [ ] Support both GET and POST connections
- [ ] Handle base64 encoding of RTSP messages
- [ ] Maintain session across two HTTP connections
- [ ] Tests verify tunneling works

## Technical Details

### HTTP Tunneling Protocol
1. Initial HTTP GET with x-sessioncookie
2. Parallel HTTP POST with same cookie
3. GET receives RTSP responses (base64)
4. POST sends RTSP requests (base64)
5. Use port 80/443 instead of 554

### Implementation Components
- HTTP client using existing tokio/hyper
- Base64 encoding/decoding layer
- Session cookie generation
- Dual connection management
- Tunnel detection from URL or property

### Properties to Add
- protocols: rtsp, http, auto (default: auto)
- proxy: HTTP proxy URL
- tunnel-port: override port (default: 80)

## Implementation Blueprint
1. Add HTTP tunneling properties
2. Detect tunneling need (firewall/property)
3. Create http_tunnel module
4. Implement GET connection handler
5. Implement POST connection handler
6. Add base64 encoding layer
7. Coordinate dual connections
8. Test with mock HTTP server

## Resources
- Apple HTTP Live Streaming spec (Appendix on RTSP tunneling)
- QuickTime RTSP tunneling: https://developer.apple.com/library/archive/documentation/QuickTime/QTSS/Concepts/QTSSConcepts.html
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c (tunneling)

## Validation Gates
```bash
# Test HTTP tunneling
cargo test -p gst-plugin-rtsp http_tunnel -- --nocapture

# Test with proxy
cargo test -p gst-plugin-rtsp http_proxy -- --nocapture

# Verify base64 encoding
cargo test -p gst-plugin-rtsp tunnel_encoding -- --nocapture
```

## Dependencies
- PRP-RTSP-05 (TLS) - for HTTPS tunneling

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - dual connection coordination
- Challenge: Managing two HTTP connections

## Success Confidence Score
6/10 - Complex protocol with limited documentation