# PRP-RTSP-31: Basic Authentication Protocol Implementation

## Overview
Implement HTTP Basic Authentication (RFC 2617) for RTSP requests using the authentication properties from PRP-30. This adds authentication header generation and credential handling.

## Context
With authentication properties available from PRP-30, this PRP implements the actual Basic Authentication protocol. The original rtspsrc supports both basic and digest authentication, but this PRP focuses only on the simpler Basic auth.

## Research Context
- RFC 2617 Section 2: Basic Authentication Scheme
- Original rtspsrc authentication handling in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- RTSP RFC 2326 Section 11: Authentication
- GStreamer RTSP library authentication: https://gstreamer.freedesktop.org/documentation/gst-rtsp/

## Scope
This PRP implements ONLY Basic Authentication:
1. Base64 encoding for credentials
2. Authorization header generation
3. Credential injection into RTSP requests
4. 401 Unauthorized response handling

Does NOT implement:
- Digest authentication
- Advanced authentication schemes
- Credential caching beyond request scope
- Authentication challenges beyond 401 response

## Implementation Tasks
1. Add Base64 encoding utility function for credentials
2. Create authentication header builder: `Authorization: Basic <base64(user:pass)>`
3. Modify RTSP request building to include authentication headers
4. Add 401 response detection and retry logic
5. Integrate with existing connection and retry mechanisms
6. Add authentication state tracking (authenticated/pending/failed)
7. Add debug logging for authentication attempts (without exposing credentials)

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Authentication integration
- `net/rtsp/src/rtspsrc/connection_racer.rs` - Request header modification
- Add new module `net/rtsp/src/rtspsrc/auth.rs` - Authentication logic

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests  
cargo test rtspsrc_basic_auth --all-targets --all-features -- --nocapture

# Integration Test - Mock server with auth
cargo test test_basic_authentication_flow --all-targets --all-features -- --nocapture
```

## Expected Behavior
1. When `user-id` and `user-pw` are set, include Authorization header in requests
2. When server responds with 401, retry with credentials if not already included
3. Authentication should work with TCP, UDP unicast, and multicast transports
4. Debug logs should show authentication attempts without revealing passwords

## Dependencies
- **Requires**: PRP-30 (authentication properties)
- **Integrates with**: Existing connection racing and retry logic

## Authentication Flow
```
1. Client sends RTSP request without auth
2. Server responds with 401 Unauthorized  
3. Client detects 401, adds Basic auth header
4. Client retries request with Authorization header
5. Server accepts request and continues RTSP handshake
```

## Success Criteria
- [ ] Basic auth header correctly formatted as per RFC 2617
- [ ] Credentials properly Base64 encoded
- [ ] 401 responses trigger authentication retry
- [ ] Authentication works across all transport protocols
- [ ] No credential information leaked in logs
- [ ] Integration with existing retry/racing mechanisms

## Risk Assessment
**MEDIUM RISK** - Involves credential handling and protocol logic, but Basic auth is well-established.

## Estimated Effort
3-4 hours

## Confidence Score
8/10 - Standard authentication pattern, well-documented protocol.