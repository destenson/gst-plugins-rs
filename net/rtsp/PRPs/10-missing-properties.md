# PRP-RTSP-10: Missing Configuration Properties Implementation

## Overview
Add essential configuration properties that exist in rtspsrc but are missing in rtspsrc2, improving feature parity and usability.

## Current State
- README lists several missing properties
- Properties needed: latency, do-rtx, do-rtcp, iface, user-agent
- Users cannot configure important behaviors

## Success Criteria
- [ ] All listed properties implemented
- [ ] Properties affect behavior correctly
- [ ] Default values match rtspsrc
- [ ] Property documentation added
- [ ] Tests verify property effects

## Technical Details

### Properties to Implement

1. **latency** (uint, ms)
   - Default: 2000ms
   - Controls jitterbuffer latency
   - Affects rtpjitterbuffer element

2. **do-rtx** (boolean)
   - Default: false
   - Enable RTCP retransmission requests
   - Requires RTCP feedback support

3. **do-rtcp** (boolean)
   - Default: true
   - Enable/disable RTCP
   - Affects sync and stats

4. **iface** (string)
   - Default: NULL (any interface)
   - Bind to specific network interface
   - For multi-homed systems

5. **user-agent** (string)
   - Default: "GStreamer/{version}"
   - Custom User-Agent header
   - For server compatibility

## Implementation Blueprint
1. Add properties in element registration
2. Store property values in element state
3. Apply latency to jitterbuffer configuration
4. Implement do-rtx in RTCP handling
5. Control RTCP with do-rtcp flag
6. Bind sockets to iface if specified
7. Add User-Agent to RTSP headers
8. Test each property's effect

## Resources
- Property definitions: ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c
- GObject property docs: https://docs.gtk.org/gobject/concepts.html#properties
- Network interface binding: https://docs.rs/socket2/latest/socket2/

## Validation Gates
```bash
# Test property getters/setters
cargo test -p gst-plugin-rtsp properties -- --nocapture

# Verify property effects
cargo test -p gst-plugin-rtsp property_behavior -- --nocapture

# Check property defaults
GST_DEBUG=rtspsrc2:5 gst-inspect-1.0 rtspsrc2
```

## Dependencies
- None (property system already exists)

## Estimated Effort
3 hours

## Risk Assessment
- Low risk - additive changes only
- Challenge: Testing network interface binding

## Success Confidence Score
9/10 - Straightforward property additions with clear examples