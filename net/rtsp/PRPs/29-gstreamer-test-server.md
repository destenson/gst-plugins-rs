# PRP-RTSP-29: GStreamer-based RTSP Test Server

## Overview
Create a real RTSP test server using the system's GStreamer installation (gst-rtsp-server) for more realistic testing of the rtspsrc2 element, including VOD seeking, pause, and other advanced features.

## Current State
- Mock server in PRP-02 doesn't provide real RTP/RTCP timing
- Cannot test real-world scenarios like seeking, buffering, network issues
- Tests fail without proper RTSP server infrastructure
- VOD seeking tests require Range header support

## Success Criteria
- [ ] Create test helper that launches gst-rtsp-server process
- [ ] Support both live and VOD content
- [ ] Enable Range header support for seeking tests
- [ ] Handle server lifecycle (start/stop) in tests
- [ ] Provide multiple test streams (audio, video, audio+video)
- [ ] Support authentication testing
- [ ] Enable RTCP feedback testing

## Technical Details

### Server Components
1. **Process Management**
   - Spawn gst-rtsp-server using std::process::Command
   - Find available port dynamically
   - Wait for server readiness
   - Clean shutdown on test completion

2. **Pipeline Templates**
   - Live: `videotestsrc ! x264enc ! rtph264pay`
   - VOD: `filesrc ! decodebin ! x264enc ! rtph264pay`
   - Audio: `audiotestsrc ! opusenc ! rtpopuspay`

3. **Server Configuration**
   - Enable seeking for VOD streams
   - Configure authentication if needed
   - Set up proper mount points
   - Enable RTCP feedback

### Implementation Approach
```rust
// tests/rtsp_test_server.rs
pub struct GstRtspTestServer {
    process: Child,
    port: u16,
    mount_point: String,
}

impl GstRtspTestServer {
    pub fn new_live() -> Result<Self> { ... }
    pub fn new_vod(file: &Path) -> Result<Self> { ... }
    pub fn with_auth(username: &str, password: &str) -> Result<Self> { ... }
    pub fn url(&self) -> String { ... }
}
```

### Using gst-launch-1.0 for Simple Cases
For basic testing, can use:
```bash
gst-launch-1.0 rtspsrc2 location=rtsp://localhost:8554/test ! fakesink
```

With test server:
```bash
gst-rtsp-server-1.0 "( videotestsrc ! x264enc ! rtph264pay name=pay0 )"
```

### VOD Server with Seeking Support
```bash
gst-rtsp-server-1.0 --gst-debug=3 \
  "( filesrc location=test.mp4 ! qtdemux ! h264parse ! rtph264pay name=pay0 )"
```

## Implementation Blueprint
1. Create `tests/rtsp_test_server.rs` module
2. Add server process management with timeouts
3. Implement pipeline builders for different content types
4. Add port discovery (try ports 8554-8654)
5. Create test fixtures for common scenarios
6. Update seek tests to use real server
7. Add integration tests for all RTSP features
8. Document server requirements and setup

## Resources
- gst-rtsp-server: https://gstreamer.freedesktop.org/modules/gst-rtsp-server.html
- test-launch: https://github.com/GStreamer/gst-rtsp-server/blob/main/examples/test-launch.c
- GStreamer testing: https://gstreamer.freedesktop.org/documentation/tutorials/basic/debugging-tools.html

## Validation Gates
```bash
# Check if gst-rtsp-server is available
which gst-rtsp-server-1.0 || echo "Not found"

# Test with real server
cargo test -p gst-plugin-rtsp --test seek_tests -- --ignored

# Test authentication
cargo test -p gst-plugin-rtsp auth_with_server -- --ignored

# Test seeking accuracy
cargo test -p gst-plugin-rtsp seek_accuracy_real -- --ignored
```

## Dependencies
- System GStreamer installation with gst-rtsp-server
- PRP-RTSP-01 (Unit Test Framework)
- PRP-RTSP-18 (VOD Seeking) for testing

## Platform Considerations
- **Linux/macOS**: gst-rtsp-server usually available
- **Windows**: May need to build gst-rtsp-server or use WSL
- **CI**: Need to install GStreamer in CI environment

## Estimated Effort
3 hours

## Risk Assessment
- Low technical risk - using standard GStreamer tools
- Main challenge: Platform differences and CI setup
- Dependency on system GStreamer installation

## Success Confidence Score
9/10 - Using real GStreamer components ensures compatibility