# PRP-RTSP-41: RTP-Specific Properties Implementation

## Overview
Implement RTP-specific properties (`rtp-blocksize`, `tcp-timestamp`, `sdes`) to match original rtspsrc RTP protocol capabilities and control RTP packet characteristics.

## Context
The original rtspsrc provides RTP-specific controls through properties like `rtp-blocksize` (packet size hints), `tcp-timestamp` (TCP timestamping), and `sdes` (session description elements). These properties are essential for RTP optimization and session management.

## Research Context
- Original rtspsrc RTP properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- RFC 3550: RTP specification and packet size considerations  
- SDES (Source Description) items in RTP sessions
- TCP timestamping for RTP-over-TCP scenarios
- RTP packet size negotiation with RTSP servers

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `rtp-blocksize` unsigned integer property (default: 0, disabled)
2. Add `tcp-timestamp` boolean property (default: false)
3. Add `sdes` GstStructure property for session description elements
4. Add property validation and state change restrictions

Does NOT implement:
- Actual RTP packet size suggestion to server
- TCP timestamp injection into RTP packets
- SDES item generation or processing
- RTP session manager integration

## Implementation Tasks
1. Add RTP-specific fields to RtspSrcSettings struct
2. Implement `rtp-blocksize` with range 0-65536 (0 = disabled)
3. Implement `tcp-timestamp` boolean property
4. Implement `sdes` property with GstStructure type (boxed pointer)
5. Add property change restrictions (changeable only in NULL or READY state)
6. Add RTP blocksize validation (must be reasonable packet size)
7. Document SDES structure format and common fields

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and RtspSrcSettings
- May need GstStructure handling for SDES property

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_rtp_properties --all-targets --all-features -- --nocapture

# SDES Structure Test
cargo test test_sdes_property_structure --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
rtp-blocksize       : RTP package size to suggest to server (0 = disabled)
                      flags: readable, writable, changeable only in NULL or READY state
                      Unsigned Integer. Range: 0 - 65536 Default: 0

tcp-timestamp       : Timestamp RTP packets with receive times in TCP/HTTP mode
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: false

sdes                : The SDES items of this session
                      flags: readable, writable, changeable only in NULL or READY state
                      Boxed pointer of type "GstStructure"
```

## Property Behavior Details
- **rtp-blocksize**: Suggests RTP packet size to server (0 = no suggestion, let server decide)
- **tcp-timestamp**: In TCP/HTTP mode, timestamp RTP packets with local receive time
- **sdes**: Session Description Elements structure containing participant information

## RTP Block Size Guidelines
- Range: 0 to 65536 bytes
- 0 = disabled (server chooses packet size)
- Typical values: 1024, 1400 (MTU consideration), 8192
- Should not exceed network MTU to avoid fragmentation

## SDES Structure Format  
Common SDES fields in GstStructure:
- `cname`: Canonical name (string)
- `name`: Participant name (string)
- `email`: Email address (string)
- `phone`: Phone number (string)
- `location`: Geographic location (string)
- `tool`: Application/tool name (string)
- `note`: Miscellaneous note (string)

## Dependencies
- **GStreamer**: GstStructure type for SDES property
- **May need**: Structure serialization/deserialization utilities

## Success Criteria
- [ ] All three properties visible in gst-inspect output
- [ ] rtp-blocksize accepts range 0-65536 with proper validation
- [ ] tcp-timestamp boolean property works correctly
- [ ] sdes property accepts GstStructure values
- [ ] Properties changeable only in NULL/READY states
- [ ] Property defaults match original rtspsrc exactly
- [ ] No actual RTP processing logic implemented (out of scope)

## Risk Assessment  
**MEDIUM RISK** - GstStructure property handling adds complexity.

## Estimated Effort
3-4 hours (GstStructure property handling)

## Confidence Score
7/10 - Straightforward except for GstStructure property integration.