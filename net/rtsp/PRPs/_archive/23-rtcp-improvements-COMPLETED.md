# PRP-RTSP-23: RTCP Enhancements and Statistics

## Overview
Enhance RTCP handling with better statistics collection, feedback messages, and reporting for improved stream quality monitoring and control.

## Current State
- Basic RTCP SR/RR support exists
- Limited statistics collection
- No advanced RTCP feedback
- Missing detailed quality metrics

## Success Criteria
- [ ] Extended RTCP statistics
- [ ] RTCP XR (Extended Reports) support
- [ ] Feedback message handling
- [ ] Statistics properties/signals
- [ ] Tests verify RTCP features

## Technical Details

### RTCP Enhancements
1. **Statistics Collection**
   - Packet loss percentage
   - Jitter measurements
   - Round-trip time
   - Bandwidth usage

2. **RTCP XR (RFC 3611)**
   - Loss RLE reports
   - Duplicate RLE
   - Packet Receipt Times
   - Receiver Reference Time

3. **Feedback Messages**
   - Generic NACK (RFC 4585)
   - PLI (Picture Loss Indication)
   - FIR (Full Intra Request)
   - REMB (Receiver Estimated Max Bitrate)

### Properties/Signals
- stats property: current statistics
- stats-updated signal: periodic updates
- rtcp-feedback signal: feedback events

## Implementation Blueprint
1. Enhance RTCP parser for XR
2. Create statistics collector
3. Add feedback message types
4. Implement stats aggregation
5. Add properties and signals
6. Create stats reporting
7. Add RTCP test cases
8. Document statistics API

## Resources
- RFC 3611 (RTCP XR): https://datatracker.ietf.org/doc/html/rfc3611
- RFC 4585 (RTCP Feedback): https://datatracker.ietf.org/doc/html/rfc4585
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtpmanager/
- GStreamer RTP stats: https://gstreamer.freedesktop.org/documentation/rtpmanager/

## Validation Gates
```bash
# Test RTCP statistics
cargo test -p gst-plugin-rtsp rtcp_stats -- --nocapture

# Test XR reports
cargo test -p gst-plugin-rtsp rtcp_xr -- --nocapture

# Test feedback messages
cargo test -p gst-plugin-rtsp rtcp_feedback -- --nocapture
```

## Dependencies
- None (enhances existing RTCP)

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - protocol extensions
- Challenge: Parsing various RTCP formats

## Success Confidence Score
7/10 - Clear specifications but many variants