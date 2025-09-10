# PRP-RTSP-05: TLS/TCP Transport Setup

## Overview
Add TLS support for secure RTSP connections (rtsps://), implementing the foundation for encrypted RTSP communication on port 322.

## Current State
- No TLS support implemented
- Listed as missing feature: "TLS/TCP support"
- Only plain TCP connections supported
- Cannot connect to rtsps:// URLs

## Success Criteria
- [ ] Parse rtsps:// URLs correctly
- [ ] Establish TLS connections on port 322
- [ ] Support configurable TLS versions
- [ ] Handle certificate validation
- [ ] Tests pass with TLS mock server

## Technical Details

### TLS Implementation Components
1. URL scheme detection (rtsp:// vs rtsps://)
2. TLS connector using tokio-native-tls or tokio-rustls
3. Certificate validation options
4. TLS version configuration
5. Upgrade existing TCP code to handle TLS streams

### Reference Patterns
- Check ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c for TLS handling
- Study net/reqwest plugin for TLS patterns in Rust
- Use tokio TLS features (already a dependency)

### Properties to Add
- tls-validation-flags (similar to rtspsrc)
- tls-database (certificate store)
- default to port 322 for rtsps://

## Implementation Blueprint
1. Add TLS dependencies to Cargo.toml (tokio-native-tls)
2. Modify URL parsing to detect rtsps scheme
3. Create tls module in rtspsrc/
4. Abstract TCP stream to support both plain and TLS
5. Implement TLS connector with certificate options
6. Update connection establishment logic
7. Add TLS properties to element
8. Create TLS tests with mock server

## Resources
- RTSPS specification (port 322): RFC 2326 Section 11.1
- tokio-native-tls docs: https://docs.rs/tokio-native-tls/
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c (search for "rtsps")
- GStreamer TLS: https://gstreamer.freedesktop.org/documentation/gio/giostreamsink.html

## Validation Gates
```bash
# Test TLS connection
cargo test -p gst-plugin-rtsp tls -- --nocapture

# Test with self-signed cert
cargo test -p gst-plugin-rtsp tls_self_signed -- --nocapture

# Verify port 322 usage
GST_DEBUG=rtspsrc2:5 gst-launch-1.0 rtspsrc2 location=rtsps://example.com/stream ! fakesink
```

## Dependencies
- PRP-RTSP-02 (Mock Server) - needs TLS support added

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - TLS adds significant complexity
- Challenge: Certificate validation configuration

## Success Confidence Score
7/10 - tokio has good TLS support but integration needs care