# PRP-RTSP-35: RTCP Control Properties Implementation

## Overview
Implement RTCP control properties (`do-rtcp`, `do-retransmission`, `max-rtcp-rtp-time-diff`) to match original rtspsrc RTCP capabilities and provide control over RTP Control Protocol behavior.

## Context
The original rtspsrc provides comprehensive RTCP control through properties like `do-rtcp` (enable/disable RTCP), `do-retransmission` (request retransmissions), and `max-rtcp-rtp-time-diff` (RTCP SR validation). rtspsrc2 currently lacks these critical RTCP controls.

## Research Context
- Original rtspsrc RTCP properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- RFC 3550: RTP: A Transport Protocol for Real-Time Applications (RTCP section)
- RFC 4588: RTP Retransmission Payload Format  
- GStreamer RTCP documentation: https://gstreamer.freedesktop.org/documentation/rtpmanager/
- RTCP SR (Sender Report) timestamp validation requirements

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `do-rtcp` boolean property (default: true)
2. Add `do-retransmission` boolean property (default: true) 
3. Add `max-rtcp-rtp-time-diff` integer property (default: -1, disabled)
4. Add property storage and validation logic

Does NOT implement:
- Actual RTCP packet processing
- RTP retransmission request logic
- RTCP SR/RR generation or validation
- Jitterbuffer RTCP integration

## Implementation Tasks
1. Add RTCP control fields to RtspSrcSettings struct
2. Implement `do-rtcp` boolean property with proper documentation
3. Implement `do-retransmission` boolean property
4. Implement `max-rtcp-rtp-time-diff` with signed integer range (-1 to 2147483647)
5. Add property change restrictions (changeable only in NULL or READY state)
6. Update property registration with correct flags and defaults
7. Implement getter/setter methods for all RTCP properties

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and RtspSrcSettings
- Property registration code for RTCP controls

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_rtcp_properties --all-targets --all-features -- --nocapture

# Property Range Test
cargo test test_rtcp_property_ranges --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
do-rtcp             : Send RTCP packets, disable for old incompatible server.
                      flags: readable, writable, changeable only in NULL or READY state  
                      Boolean. Default: true

do-retransmission   : Ask the server to retransmit lost packets
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: true

max-rtcp-rtp-time-diff: Maximum amount of time in ms that the RTP time in RTCP SRs is allowed to be ahead (-1 disabled)
                        flags: readable, writable, changeable only in NULL or READY state
                        Integer. Range: -1 - 2147483647 Default: -1
```

## Property Behavior Details
- **do-rtcp**: Controls whether RTCP packets are sent/processed (some old servers don't support RTCP)
- **do-retransmission**: When enabled, requests retransmission of lost RTP packets from server
- **max-rtcp-rtp-time-diff**: Validates RTCP SR timestamps to prevent time drift issues (-1 = disabled)

## Dependencies
None - pure property infrastructure.

## Success Criteria
- [ ] All three RTCP properties visible in gst-inspect output
- [ ] Boolean properties accept true/false values correctly
- [ ] max-rtcp-rtp-time-diff accepts -1 (disabled) and positive millisecond values
- [ ] Properties changeable only in NULL/READY states  
- [ ] Property defaults match original rtspsrc (true, true, -1)
- [ ] No actual RTCP logic implemented (out of scope)

## Risk Assessment
**LOW RISK** - Property-only implementation with well-defined ranges.

## Estimated Effort
2-3 hours

## Confidence Score
9/10 - Straightforward property implementation following established patterns.