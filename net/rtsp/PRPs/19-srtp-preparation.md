# PRP-RTSP-19: SRTP Support Preparation

## Overview
Prepare infrastructure for Secure RTP (SRTP) support, enabling encrypted media streams for enhanced security in RTSP communications.

## Current State
- Listed as missing: "SRTP support"
- No encryption for media streams
- Only RTSP control channel can use TLS
- Media vulnerable to interception

## Success Criteria
- [ ] Parse SRTP SDP attributes
- [ ] Detect SRTP requirement
- [ ] Prepare SRTP element integration
- [ ] Handle key management attributes
- [ ] Tests verify SRTP detection

## Technical Details

### SRTP Components
1. SDP crypto attribute parsing
2. Key management (MIKEY, SDES)
3. SRTP/SRTCP profile detection
4. Cipher suite negotiation
5. Integration with gst srtpdec

### SDP Attributes
- a=crypto: for SDES key exchange
- a=key-mgmt: for MIKEY
- RTP/SAVP or RTP/SAVPF profiles
- Supported ciphers (AES_CM_128, etc.)

### Integration Points
- Detect SRTP in SDP
- Create srtpdec elements
- Pass keys to decoder
- Signal SRTP status

## Implementation Blueprint
1. Extend SDP parser for crypto attributes
2. Add SRTP detection logic
3. Parse crypto parameters
4. Create srtp module
5. Prepare element factory for srtpdec
6. Add srtp properties
7. Store keys securely
8. Test SRTP SDP parsing

## Resources
- RFC 3711 (SRTP): https://datatracker.ietf.org/doc/html/rfc3711
- RFC 4568 (SDP Security): https://datatracker.ietf.org/doc/html/rfc4568
- GStreamer SRTP: https://gstreamer.freedesktop.org/documentation/srtp/
- Local ref: Check gst-plugins-bad for SRTP elements

## Validation Gates
```bash
# Test SRTP SDP parsing
cargo test -p gst-plugin-rtsp srtp_sdp -- --nocapture

# Test crypto attribute handling
cargo test -p gst-plugin-rtsp srtp_crypto -- --nocapture

# Verify SRTP detection
cargo test -p gst-plugin-rtsp srtp_detect -- --nocapture
```

## Dependencies
- Requires gst-plugins-bad for srtpdec element

## Estimated Effort
3 hours

## Risk Assessment
- Medium complexity - security-critical code
- Challenge: Secure key handling

## Success Confidence Score
6/10 - SRTP adds significant complexity