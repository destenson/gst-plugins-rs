# PRP-RTSP-07: GET_PARAMETER and SET_PARAMETER Support

## Overview
Implement RTSP GET_PARAMETER and SET_PARAMETER methods for camera control and status queries, essential for PTZ cameras and ONVIF devices.

## Current State
- Listed as missing feature in README.md
- Cannot query or control camera parameters
- Required for ONVIF camera control

## Success Criteria
- [ ] Send GET_PARAMETER requests
- [ ] Send SET_PARAMETER requests
- [ ] Handle parameter responses
- [ ] Expose as element actions/signals
- [ ] Tests verify parameter operations

## Technical Details

### RTSP Parameter Commands
1. GET_PARAMETER - retrieve server/stream parameters
2. SET_PARAMETER - modify server/stream parameters
3. Used for keep-alive (empty GET_PARAMETER)
4. Used for camera control (PTZ, focus, etc.)
5. Content-Type typically text/parameters

### Implementation Components
- Action signals for get/set operations
- Parameter name/value parsing
- Request/response correlation
- Async parameter operations
- Keep-alive timer using GET_PARAMETER

### Signal Interface
- get-parameter signal: returns parameter value
- set-parameter signal: sets parameter value
- parameters-changed signal: notifies changes

## Implementation Blueprint
1. Add GET_PARAMETER to RTSP methods enum
2. Add SET_PARAMETER to RTSP methods enum
3. Implement parameter request builders
4. Add action signals to element
5. Parse text/parameters responses
6. Add keep-alive using empty GET_PARAMETER
7. Create parameter operation tests
8. Document signal usage

## Resources
- RTSP RFC 2326 Section 10.8 & 10.9: https://datatracker.ietf.org/doc/html/rfc2326#section-10.8
- ONVIF streaming spec: https://www.onvif.org/specs/stream/ONVIF-Streaming-Spec.pdf
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c (GET_PARAMETER)

## Validation Gates
```bash
# Test parameter operations
cargo test -p gst-plugin-rtsp parameter -- --nocapture

# Test keep-alive with GET_PARAMETER
cargo test -p gst-plugin-rtsp keepalive -- --nocapture

# Integration test with mock server
cargo test -p gst-plugin-rtsp parameter_integration -- --nocapture
```

## Dependencies
- PRP-RTSP-02 (Mock Server) - needs parameter support

## Estimated Effort
3 hours

## Risk Assessment
- Low complexity - straightforward protocol addition
- Challenge: Async signal handling in GStreamer

## Success Confidence Score
8/10 - Clear specification, simple implementation