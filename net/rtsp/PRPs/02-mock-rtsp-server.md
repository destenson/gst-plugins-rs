# PRP-RTSP-02: Mock RTSP Server for Testing

## Overview
Create a lightweight mock RTSP server for testing the rtspsrc2 element without external dependencies. This enables reliable, repeatable testing of RTSP protocol interactions.

## Current State
- No mock server exists for testing
- Testing requires external RTSP servers (live555, gst-rtsp-server)
- Cannot test error conditions or edge cases reliably

## Success Criteria
- [ ] Mock server responds to basic RTSP commands (OPTIONS, DESCRIBE, SETUP, PLAY)
- [ ] Returns configurable SDP for testing different media formats
- [ ] Simulates RTP/RTCP data delivery
- [ ] Can simulate error conditions and timeouts

## Technical Details

### RTSP Server Components
1. TCP listener on configurable port (default 8554)
2. RTSP request parser using rtsp-types crate
3. Response generator with proper sequence numbers
4. Basic session management
5. Mock RTP data generator

### Reference Implementation
- Study test-launch.c from gst-rtsp-server/examples/
- Use rtsp-types crate for message parsing (already a dependency)
- Follow patterns from net/reqwest/tests/ mock server

### Mock Server Features
- Configurable SDP responses
- Adjustable latency simulation
- Error injection (connection drops, malformed responses)
- Basic authentication support (stub for testing)
- Multiple concurrent client support

## Implementation Blueprint
1. Create tests/mock_server.rs module
2. Implement RTSPMockServer struct with tokio TcpListener
3. Add request parsing using rtsp-types
4. Implement response handlers for each RTSP method
5. Add mock RTP packet generator
6. Create test helpers for common scenarios
7. Write tests using the mock server
8. Document usage patterns

## Resources
- rtsp-types crate documentation: https://docs.rs/rtsp-types/
- RTSP RFC 2326: https://datatracker.ietf.org/doc/html/rfc2326
- gst-rtsp-server test-launch example: https://github.com/GStreamer/gst-rtsp-server/blob/master/examples/test-launch.c

## Validation Gates
```bash
# Test the mock server
cargo test -p gst-plugin-rtsp mock_server -- --nocapture

# Ensure it works with actual element
cargo test -p gst-plugin-rtsp integration -- --nocapture

# Check for race conditions
cargo test -p gst-plugin-rtsp mock_server -- --test-threads=1
```

## Dependencies
- PRP-RTSP-01 (Unit Test Framework Setup)

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - requires understanding RTSP protocol
- Main challenge: Simulating realistic RTP timing

## Success Confidence Score
7/10 - rtsp-types crate simplifies implementation significantly