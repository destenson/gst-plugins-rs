# PRP-RTSP-40: Transport Protocol Enhancements

## Overview  
Enhance transport protocol support to match original rtspsrc capabilities by adding missing URI protocols (`rtsph`, `rtsp-sdp`, `rtsps`, `rtspsu`, `rtspst`, `rtspsh`) and implementing `default-rtsp-version` property for version negotiation.

## Context
The original rtspsrc supports 9 URI protocols while rtspsrc2 currently supports only 3 (`rtsp`, `rtspu`, `rtspt`). Missing protocols include HTTPS tunneling (`rtsph`), SDP-only (`rtsp-sdp`), secure variants (`rtsps`, `rtspsu`, `rtspst`, `rtspsh`), and version negotiation control.

## Research Context  
- Original rtspsrc URI protocol support in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- RTSP over HTTPS (HTTP tunneling) RFC specifications
- RTSP over TLS/SSL secure transport mechanisms
- RTSP version negotiation: 1.0, 1.1, 2.0 support
- SDP-only RTSP sessions for session description retrieval

## Scope
This PRP implements ONLY the property and URI protocol registration:
1. Add missing URI protocol registrations: `rtsph`, `rtsp-sdp`, `rtsps`, `rtspsu`, `rtspst`, `rtspsh`
2. Add `default-rtsp-version` enumeration property (default: "1-0")
3. Update URI handler to recognize all protocol variants
4. Add RTSP version enum with values: invalid(0), 1-0(16), 1-1(17), 2-0(32)

Does NOT implement:
- HTTPS tunneling logic
- TLS/SSL connection establishment  
- Actual RTSP 1.1 or 2.0 protocol differences
- SDP-only session handling

## Implementation Tasks
1. Define RtspVersion enum: Invalid, V1_0, V1_1, V2_0 with original numeric values
2. Add `default-rtsp-version` property with enum type
3. Update URI handler protocol list to include all 9 protocol variants
4. Add URI protocol parsing logic for new variants  
5. Add version property validation and state change restrictions
6. Document protocol variant meanings and usage scenarios
7. Update element URI handling capabilities inspection

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property and enum definitions
- `net/rtsp/src/rtspsrc/mod.rs` - URI handler registration
- URI protocol parsing and validation logic

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_transport_enhancements --all-targets --all-features -- --nocapture

# URI Protocol Test
cargo test test_uri_protocol_variants --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:

**URI Protocols:**
```
Supported URI protocols:
  rtsp
  rtspu  
  rtspt
  rtsph
  rtsp-sdp
  rtsps
  rtspsu
  rtspst
  rtspsh
```

**Property:**
```  
default-rtsp-version: The RTSP version that should be tried first when negotiating version.
                      flags: readable, writable, changeable only in NULL or READY state
                      Enum "RtspVersion" Default: 16, "1-0"
                         (0): invalid          - GST_RTSP_VERSION_INVALID
                         (16): 1-0              - GST_RTSP_VERSION_1_0  
                         (17): 1-1              - GST_RTSP_VERSION_1_1
                         (32): 2-0              - GST_RTSP_VERSION_2_0
```

## Protocol Variant Meanings
- **rtsp**: Standard RTSP over TCP/UDP
- **rtspu**: RTSP explicitly over UDP
- **rtspt**: RTSP explicitly over TCP  
- **rtsph**: RTSP over HTTPS (HTTP tunneling)
- **rtsp-sdp**: SDP-only session (retrieve session description only)
- **rtsps**: RTSP over TLS/SSL
- **rtspsu**: RTSP over TLS/SSL with UDP data  
- **rtspst**: RTSP over TLS/SSL with TCP data
- **rtspsh**: RTSP over TLS/SSL with HTTPS tunneling

## RTSP Version Enum Values
Numeric values must match original GStreamer constants:
- Invalid: 0
- Version 1.0: 16  
- Version 1.1: 17
- Version 2.0: 32

## Dependencies
- **Enum type**: RtspVersion enumeration matching GStreamer values

## Success Criteria
- [ ] All 9 URI protocols registered and recognized
- [ ] gst-inspect shows complete URI protocol list
- [ ] default-rtsp-version property with correct enum values
- [ ] URI protocol parsing accepts all variants
- [ ] Property defaults match original rtspsrc (1-0)  
- [ ] Version enum uses correct numeric values
- [ ] No actual protocol logic implemented (out of scope)

## Risk Assessment
**LOW-MEDIUM RISK** - URI registration and enum property, but requires GStreamer integration.

## Estimated Effort
3-4 hours

## Confidence Score
8/10 - URI registration is straightforward, enum requires attention to numeric values.