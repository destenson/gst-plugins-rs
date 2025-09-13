# PRP-RTSP-03: Basic Authentication Support

## Overview
Implement HTTP Basic Authentication (RFC 7617) for RTSP connections, enabling rtspsrc2 to connect to password-protected RTSP streams.

## Current State
- No authentication support implemented
- Listed as missing feature in README.md: "Credentials support"
- Cannot connect to secured RTSP cameras/servers

## Success Criteria
- [ ] Parse and store username/password from RTSP URLs
- [ ] Generate correct Authorization headers
- [ ] Handle 401 Unauthorized responses
- [ ] Retry with credentials on authentication challenge
- [ ] Tests pass with mock authenticated server

## Technical Details

### Authentication Flow
1. Parse credentials from URL (rtsp://user:pass@host/path)
2. Store credentials securely in element state
3. On 401 response with WWW-Authenticate header
4. Generate Authorization: Basic base64(user:pass)
5. Retry request with authorization header

### Implementation Components
- URL credential parsing in transport.rs
- Authorization header generation
- Response code handling in imp.rs
- Credential storage in element properties
- Property additions: "user-id" and "user-pw" (matching rtspsrc)

### Security Considerations
- Clear credentials from memory after use
- Don't log credentials
- Support credentials via properties (not just URL)
- Handle special characters in passwords

## Implementation Blueprint
1. Add user-id and user-pw properties to element
2. Parse credentials from URL in handle_connection
3. Create auth module in rtspsrc/
4. Implement basic_auth_header() function
5. Modify request sending to include auth header
6. Handle 401 responses and retry logic
7. Add tests with authenticated mock server
8. Update documentation

## Resources
- RFC 7617 (HTTP Basic Auth): https://datatracker.ietf.org/doc/html/rfc7617
- gst-rtsp-server test-auth.c: https://github.com/GStreamer/gst-rtsp-server/blob/master/examples/test-auth.c
- Base64 encoding in Rust: data-encoding crate (already a dependency)

## Validation Gates
```bash
# Run authentication tests
cargo test -p gst-plugin-rtsp auth -- --nocapture

# Test with real camera (if available)
GST_DEBUG=rtspsrc2:5 gst-launch-1.0 rtspsrc2 location=rtsp://admin:password@camera.local/stream ! fakesink

# Verify no credential leaks in logs
cargo test -p gst-plugin-rtsp 2>&1 | grep -i password
```

## Dependencies
- PRP-RTSP-02 (Mock RTSP Server) - for testing

## Estimated Effort
3 hours

## Risk Assessment
- Low risk - additive feature
- Main concern: Secure credential handling

## Success Confidence Score
9/10 - Straightforward implementation with clear RFC specification