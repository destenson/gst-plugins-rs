# PRP-RTSP-22: Real Camera Compatibility Testing Framework

## Overview
Create a comprehensive testing framework for validating rtspsrc2 against real IP cameras and RTSP servers, identifying and documenting compatibility issues.

## Current State
- README notes: "Test with market RTSP cameras"
- Only tested with live555 and gst-rtsp-server
- Unknown compatibility with real devices
- No systematic testing approach

## Success Criteria
- [ ] Test framework for real devices
- [ ] Compatibility test suite
- [ ] Issue detection and reporting
- [ ] Camera quirks documentation
- [ ] Automated regression tests

## Technical Details

### Test Categories
1. **Basic Connectivity**: Auth, transport negotiation
2. **Stream Formats**: H.264, H.265, MJPEG, audio codecs
3. **Features**: PTZ, ONVIF, events, backchannel
4. **Reliability**: Reconnection, timeout, errors
5. **Performance**: Latency, throughput, stability

### Test Devices/Servers
- Popular IP cameras (Hikvision, Dahua, Axis)
- ONVIF Profile S/T devices
- VLC RTSP server
- FFmpeg RTSP server
- Wowza, Red5 servers

### Compatibility Matrix
- Device model/firmware
- Supported features
- Known issues/workarounds
- Performance metrics

## Implementation Blueprint
1. Create camera_tests module
2. Add test configuration file support
3. Implement device discovery (ONVIF)
4. Create compatibility test suite
5. Add performance benchmarks
6. Generate compatibility reports
7. Document camera quirks
8. Setup CI with test servers

## Resources
- ONVIF Test Tool: https://www.onvif.org/conformance/test-tools/
- Camera SDKs and simulators
- Docker images for test servers
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/tests/examples/rtsp/

## Validation Gates
```bash
# Run compatibility tests
cargo test -p gst-plugin-rtsp compat -- --nocapture

# Test with specific camera
GST_DEBUG=rtspsrc2:5 cargo test -p gst-plugin-rtsp camera_hikvision

# Generate compatibility report
cargo test -p gst-plugin-rtsp --features compat-report
```

## Dependencies
- Previous PRPs for feature implementation

## Estimated Effort
4 hours (framework only, ongoing testing)

## Risk Assessment
- Medium complexity - device variability
- Challenge: Access to diverse cameras
- Benefit: Real-world validation

## Success Confidence Score
7/10 - Framework straightforward, device access varies