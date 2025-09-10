# PRP-RTSP-04: Digest Authentication Support

## Overview
Implement HTTP Digest Authentication (RFC 7616) for RTSP, providing more secure authentication than Basic auth by avoiding plaintext password transmission.

## Current State
- No digest authentication support
- Many RTSP cameras prefer digest over basic auth
- Cannot connect to digest-only secured streams

## Success Criteria
- [ ] Parse WWW-Authenticate digest challenges
- [ ] Generate correct digest response with MD5
- [ ] Support qop="auth" and qop="auth-int"
- [ ] Handle nonce updates between requests
- [ ] Tests pass with digest-authenticated mock server

## Technical Details

### Digest Authentication Flow
1. Receive 401 with WWW-Authenticate: Digest realm="...", nonce="...", qop="..."
2. Calculate response hash: MD5(MD5(user:realm:pass):nonce:MD5(method:uri))
3. Send Authorization: Digest username="...", response="...", ...
4. Track nonce for session reuse
5. Handle stale nonce challenges

### Reference Implementation
- Check ~/repos/gstreamer/subprojects/gst-rtsp-server/examples/test-auth-digest.c
- Study gst-plugins-good rtspsrc digest implementation
- Use md5 or ring crate for hashing

### Components to Implement
- Digest challenge parser
- Response hash calculator
- Nonce management per session
- Client nonce (cnonce) generation
- Request counter (nc) tracking

## Implementation Blueprint
1. Extend auth module from PRP-RTSP-03
2. Add DigestAuth struct for challenge parsing
3. Implement MD5 hash calculation functions
4. Add digest_auth_header() function
5. Modify 401 handler to detect auth type
6. Store nonce and increment nc counter
7. Add digest auth tests
8. Handle auth-int qop (optional)

## Resources
- RFC 7616 (HTTP Digest): https://datatracker.ietf.org/doc/html/rfc7616
- Local ref: ~/repos/gstreamer/subprojects/gst-rtsp-server/gst/rtsp-server/rtsp-auth.c
- Example: ~/repos/gstreamer/subprojects/gst-rtsp-server/examples/test-auth-digest.c
- MD5 implementation: md5 crate or ring crate

## Validation Gates
```bash
# Run digest auth tests
cargo test -p gst-plugin-rtsp digest -- --nocapture

# Test against mock server with digest
cargo test -p gst-plugin-rtsp mock_digest -- --nocapture

# Verify with GStreamer test server
cd ~/repos/gstreamer/subprojects/gst-rtsp-server/examples
./test-auth-digest
# Then test with rtspsrc2
```

## Dependencies
- PRP-RTSP-03 (Basic Authentication) - builds on auth infrastructure

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - digest algorithm has multiple variations
- Challenge: Correctly implementing all qop modes

## Success Confidence Score
7/10 - More complex than basic auth but well-documented