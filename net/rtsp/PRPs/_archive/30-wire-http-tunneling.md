# PRP-RTSP-30: Wire HTTP Tunneling Implementation

## Overview
HTTP tunneling is fully implemented in `http_tunnel.rs` but never instantiated or used. This PRP connects the tunneling logic to the connection establishment flow.

## Current State
- HttpTunnel struct exists with complete implementation
- Properties for tunnel mode and port are defined
- No code creates HttpTunnel or uses it for connections
- Should automatically detect when tunneling is needed

## Success Criteria
- [ ] HTTP tunneling used when mode is "always"
- [ ] Auto-detection of tunneling need based on firewall/proxy
- [ ] Dual GET/POST connections established correctly
- [ ] Base64 encoding/decoding working for RTSP messages
- [ ] Tests verify tunneling through mock HTTP proxy

## Technical Details

### Integration Points
1. Before TCP connection in connection logic
2. Check `http_tunnel_mode` setting
3. If tunneling needed, create HttpTunnel instead of direct TCP
4. Route RTSP messages through tunnel encode/decode
5. Handle tunnel-specific errors

### Detection Logic
- Port 554 blocked → use tunneling
- Behind HTTP proxy → use tunneling
- Firewall detection via initial probe

## Implementation Blueprint
1. Add tunnel detection before connection attempt
2. Create HttpTunnel when needed in connection flow
3. Wrap TCP stream with tunnel abstraction
4. Modify RTSP message sending to use tunnel methods
5. Handle tunnel-specific keep-alive
6. Add debug logging for tunnel decisions
7. Create integration test with mock HTTP server

## Resources
- HTTP tunneling spec: https://www.rfc-editor.org/rfc/rfc2817
- RTSP over HTTP: https://www.rfc-editor.org/rfc/rfc2326#appendix-C.2
- Base64 in Rust: https://docs.rs/base64/latest/base64/
- Example implementations in VLC and FFmpeg

## Validation Gates
```bash
# Unit tests for tunneling
cargo test -p gst-plugin-rtsp http_tunnel -- --nocapture

# Test with forced tunneling
GST_DEBUG=rtspsrc2:7 gst-launch-1.0 rtspsrc2 location=rtsp://server.com http-tunnel-mode=always

# Verify base64 encoding
cargo test -p gst-plugin-rtsp tunnel_encoding -- --nocapture
```

## Dependencies
- http_tunnel.rs implementation
- TCP connection logic in imp.rs
- Proxy support (if behind HTTP proxy)

## Estimated Effort
3 hours

## Risk Assessment
- High complexity - dual connection management
- Need careful error handling for tunnel failures
- Must maintain compatibility with direct connections

## Success Confidence Score
6/10 - Complex integration with multiple connection paths