# PRP-RTSP-09: Basic NAT Hole Punching Support

## Overview
Implement basic NAT traversal for UDP transport using hole punching techniques to enable RTSP streaming through NAT/firewall configurations.

## Current State
- Listed as missing feature: "NAT hole punching"
- UDP transport may fail behind NAT
- No STUN support
- Cannot receive UDP streams through restrictive NATs

## Success Criteria
- [ ] Send UDP packets to punch NAT holes
- [ ] Support symmetric RTP/RTCP
- [ ] Configure client ports properly
- [ ] Handle NAT timeout prevention
- [ ] Tests verify NAT traversal

## Technical Details

### NAT Traversal Techniques
1. Client-initiated UDP hole punching
2. Send dummy RTP/RTCP packets to server
3. Use client_port from Transport header
4. Symmetric RTP (same port for RTP/RTCP)
5. Regular keep-alive packets

### Implementation Components
- UDP socket binding with reuse
- Initial packet burst for hole punching
- Timer for NAT keep-alive packets
- Port allocation strategy
- Fallback to TCP on failure

### Configuration Properties
- nat-method: none, dummy-packets
- prefer-tcp: fallback option
- client-port-range: port allocation

## Implementation Blueprint
1. Add NAT configuration properties
2. Implement UDP hole punching logic
3. Send dummy RTP packets after SETUP
4. Add NAT keep-alive timer
5. Monitor UDP reception success
6. Implement TCP fallback
7. Add NAT traversal tests
8. Document NAT scenarios

## Resources
- RFC 7604 (NAT Traversal for RTSP): https://datatracker.ietf.org/doc/html/rfc7604
- RFC 7825 (ICE for RTSP): https://datatracker.ietf.org/doc/html/rfc7825
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c (NAT methods)

## Validation Gates
```bash
# Test NAT hole punching
cargo test -p gst-plugin-rtsp nat_traversal -- --nocapture

# Test with simulated NAT
cargo test -p gst-plugin-rtsp nat_simulation -- --nocapture

# Verify fallback to TCP
cargo test -p gst-plugin-rtsp nat_fallback -- --nocapture
```

## Dependencies
- None (builds on existing UDP transport)

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - NAT behavior varies
- Challenge: Testing different NAT types

## Success Confidence Score
6/10 - NAT traversal is inherently complex and environment-dependent