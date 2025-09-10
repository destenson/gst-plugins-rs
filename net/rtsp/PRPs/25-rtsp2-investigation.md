# PRP-RTSP-25: RTSP 2.0 Investigation and Preparation

## Overview
Investigate RTSP 2.0 (RFC 7826) requirements and prepare groundwork for future implementation, understanding the major protocol changes and migration path.

## Current State
- Listed as missing: "RTSP 2 support (no servers exist at present)"
- Only RTSP 1.0 implemented
- Not backward compatible with 1.0
- Few servers support 2.0

## Success Criteria
- [ ] Document RTSP 2.0 differences
- [ ] Identify breaking changes
- [ ] Design version negotiation
- [ ] Create migration plan
- [ ] Tests for version detection

## Technical Details

### Major RTSP 2.0 Changes
1. **Mandatory Features**
   - TCP and TLS required
   - Version negotiation mechanism
   - New header syntax
   - Changed PLAY semantics

2. **Protocol Changes**
   - Pipelined requests
   - Request queuing
   - Media properties
   - New status codes

3. **Removed Features**
   - UDP transport for RTSP messages
   - RECORD method changes
   - Several headers deprecated

### Version Negotiation
- RTSP/2.0 in request line
- Require header for features
- Proxy-Require for proxies
- Unsupported response handling

## Implementation Blueprint
1. Research RFC 7826 thoroughly
2. Document all breaking changes
3. Design version detection
4. Create version abstraction layer
5. Add RTSP/2.0 parser stubs
6. Implement version negotiation
7. Add 2.0 detection tests
8. Create migration guide

## Resources
- RFC 7826 (RTSP 2.0): https://datatracker.ietf.org/doc/html/rfc7826
- RTSP 2.0 vs 1.0 comparison: Section 18 of RFC 7826
- rtsp-types crate 2.0 support status
- Check if any test servers exist now

## Validation Gates
```bash
# Test version detection
cargo test -p gst-plugin-rtsp version_detect -- --nocapture

# Test version negotiation
cargo test -p gst-plugin-rtsp version_negotiate -- --nocapture

# Verify 1.0 compatibility maintained
cargo test -p gst-plugin-rtsp rtsp1_compat -- --nocapture
```

## Dependencies
- None (investigation phase)

## Estimated Effort
3 hours (investigation only)

## Risk Assessment
- Low risk - research and preparation only
- Challenge: Limited server availability
- Future benefit: Ready when servers appear

## Success Confidence Score
8/10 - Investigation phase with clear RFC documentation