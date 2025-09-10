# PRP-RTSP-14: ONVIF Backchannel Preparation

## Overview
Prepare infrastructure for ONVIF backchannel support, enabling bidirectional audio communication with ONVIF-compliant IP cameras.

## Current State
- Listed as missing: "ONVIF backchannel support"
- Only receives media, cannot send
- No ONVIF-specific extensions
- Cannot support two-way audio

## Success Criteria
- [ ] Parse ONVIF backchannel SDP attributes
- [ ] Identify backchannel streams
- [ ] Create sink pads for backchannel
- [ ] Handle REQUIRE header for ONVIF
- [ ] Tests verify ONVIF SDP parsing

## Technical Details

### ONVIF Backchannel Components
1. SDP a=sendonly/recvonly/sendrecv parsing
2. REQUIRE: www.onvif.org/ver20/backchannel
3. Backchannel stream identification
4. Sink pad creation for audio return
5. Content-Type: application/sdp handling

### SDP Extensions
- Parse a=setup:active/passive
- Handle backchannel media sections
- Support ONVIF-specific attributes
- Track stream directions

### Element Changes
- Add sink pad template
- Dynamic sink pad creation
- Backchannel property flag
- Signal for backchannel detection

## Implementation Blueprint
1. Add ONVIF detection in SDP parser
2. Parse backchannel attributes
3. Create sink pad template
4. Add backchannel-mode property
5. Implement REQUIRE header support
6. Handle sink pad data flow (stub)
7. Add ONVIF SDP tests
8. Document ONVIF usage

## Resources
- ONVIF Streaming Specification: https://www.onvif.org/specs/stream/ONVIF-Streaming-Spec.pdf
- GStreamer pad templates: https://gstreamer.freedesktop.org/documentation/gstreamer/gstpadtemplate.html
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/tests/examples/rtsp/test-onvif.c

## Validation Gates
```bash
# Test ONVIF SDP parsing
cargo test -p gst-plugin-rtsp onvif_sdp -- --nocapture

# Test backchannel detection
cargo test -p gst-plugin-rtsp backchannel -- --nocapture

# Verify sink pad creation
cargo test -p gst-plugin-rtsp onvif_pads -- --nocapture
```

## Dependencies
- None (prepares for future backchannel data flow)

## Estimated Effort
3 hours

## Risk Assessment
- Low risk - preparatory work only
- Challenge: Understanding ONVIF specifications

## Success Confidence Score
7/10 - Clear spec but ONVIF has quirks