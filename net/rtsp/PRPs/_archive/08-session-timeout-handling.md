# PRP-RTSP-08: Session Timeout and Keep-Alive Management

## Overview
Implement proper RTSP session timeout handling with automatic keep-alive to prevent session expiration during streaming.

## Current State
- Basic session management exists
- No automatic keep-alive implementation
- Sessions may timeout during long streams
- No timeout value parsing from server

## Success Criteria
- [ ] Parse Session header timeout values
- [ ] Send keep-alive before timeout
- [ ] Support multiple keep-alive methods
- [ ] Handle session expiration gracefully
- [ ] Tests verify timeout prevention

## Technical Details

### Session Timeout Components
1. Parse "Session: <id>;timeout=60" headers
2. Default timeout if not specified (60 seconds)
3. Keep-alive at 80% of timeout interval
4. Multiple keep-alive methods:
   - Empty GET_PARAMETER (preferred)
   - OPTIONS request
   - RTCP RR packets (for RTP/RTCP)

### Keep-Alive Strategy
- Timer per session
- Send keep-alive at timeout * 0.8
- Track last server response time
- Reconnect if session expires
- Cancel timer on session teardown

## Implementation Blueprint
1. Parse timeout from Session headers
2. Create SessionManager struct
3. Implement keep-alive timer with tokio
4. Add keep-alive method selection
5. Send appropriate keep-alive messages
6. Handle timeout responses
7. Add session-expired signal
8. Test with various timeout values

## Resources
- RTSP Session headers: https://datatracker.ietf.org/doc/html/rfc2326#section-12.37
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c (timeout handling)
- tokio interval timers: https://docs.rs/tokio/latest/tokio/time/struct.Interval.html

## Validation Gates
```bash
# Test timeout handling
cargo test -p gst-plugin-rtsp timeout -- --nocapture

# Test keep-alive mechanisms
cargo test -p gst-plugin-rtsp keepalive -- --nocapture

# Long-running stream test
cargo test -p gst-plugin-rtsp long_session -- --nocapture --test-threads=1
```

## Dependencies
- PRP-RTSP-07 (GET_PARAMETER) - for keep-alive method

## Estimated Effort
3 hours

## Risk Assessment
- Low risk - improves stability
- Challenge: Testing time-based behavior

## Success Confidence Score
8/10 - Well-defined behavior in RFC