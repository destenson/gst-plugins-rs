# PRP-RTSP-32: Digest Authentication Protocol Implementation

## Overview
Implement HTTP Digest Authentication (RFC 2617) for RTSP requests to match original rtspsrc capabilities. This provides more secure authentication than Basic auth.

## Context
Building on PRP-31's Basic authentication, this PRP adds support for Digest authentication which is more secure and commonly used by RTSP servers. The original rtspsrc supports both authentication methods.

## Research Context
- RFC 2617 Section 3: Digest Access Authentication
- RFC 3310: HTTP Digest Authentication Using Authentication and Key Agreement (AKA)
- Original rtspsrc digest auth in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- Digest authentication algorithm examples: https://tools.ietf.org/html/rfc2617#section-3.5

## Scope  
This PRP implements ONLY HTTP Digest Authentication:
1. WWW-Authenticate header parsing  
2. MD5 hash computation for digest response
3. Authorization header generation with digest response
4. Nonce and opaque value handling
5. Support for auth and auth-int quality-of-protection

Does NOT implement:
- Advanced digest algorithms beyond MD5
- Mutual authentication
- Protection space optimization  
- Stale nonce handling beyond basic retry

## Implementation Tasks
1. Add MD5 hash utility functions
2. Create WWW-Authenticate header parser for digest challenges
3. Implement digest response calculation: `MD5(MD5(A1):nonce:MD5(A2))`
4. Create Authorization header builder for digest responses
5. Add nonce and opaque value storage/tracking
6. Integrate digest auth detection and response logic
7. Add qop (quality of protection) parameter handling
8. Ensure digest auth takes precedence over basic auth when available

## Files to Modify
- `net/rtsp/src/rtspsrc/auth.rs` - Add digest authentication logic
- `net/rtsp/src/rtspsrc/imp.rs` - Integrate digest auth selection
- May need MD5 crypto dependency in `Cargo.toml`

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_digest_auth --all-targets --all-features -- --nocapture

# Integration Test - Mock server with digest auth  
cargo test test_digest_authentication_flow --all-targets --all-features -- --nocapture
```

## Expected Behavior
1. Server sends 401 with WWW-Authenticate: Digest challenge
2. Client parses realm, nonce, opaque, qop parameters  
3. Client computes digest response using MD5
4. Client sends Authorization: Digest header with response
5. Server validates digest and allows access

## Authentication Algorithm
```
A1 = username:realm:password
A2 = method:uri  
response = MD5(MD5(A1):nonce:MD5(A2))
```

For qop=auth:
```
response = MD5(MD5(A1):nonce:nc:cnonce:qop:MD5(A2))
```

## Dependencies
- **Requires**: PRP-31 (basic auth implementation and auth infrastructure)
- **New dependency**: MD5 hashing library (likely `md-5` crate)

## Success Criteria
- [ ] Correctly parse WWW-Authenticate digest challenges
- [ ] Generate proper MD5-based digest responses
- [ ] Handle nonce, opaque, and realm parameters correctly
- [ ] Support both qop=auth and no-qop modes
- [ ] Prefer digest over basic auth when both available
- [ ] Pass digest authentication against real RTSP servers

## Risk Assessment
**MEDIUM-HIGH RISK** - Complex cryptographic calculations and parameter handling.

## Estimated Effort  
4-5 hours

## Confidence Score
7/10 - More complex than basic auth due to cryptographic calculations and parameter parsing.