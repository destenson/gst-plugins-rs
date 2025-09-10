# PRP-RTSP-16: VOD PAUSE Support

## Overview
Implement PAUSE support for Video-On-Demand (VOD) streams, allowing users to pause playback and resume from the same position.

## Current State
- Listed as missing: "PAUSE support with VOD"
- Can only play continuously
- No pause/resume capability
- Required for VOD user experience

## Success Criteria
- [ ] Send PAUSE request correctly
- [ ] Maintain session during pause
- [ ] Resume with PLAY from position
- [ ] Handle pause state transitions
- [ ] Tests verify pause/resume cycle

## Technical Details

### PAUSE Implementation
1. PAUSE method in RTSP protocol
2. Session maintenance during pause
3. RTP-Info tracking for position
4. State machine updates
5. Buffer management during pause

### State Transitions
- PLAYING -> PAUSED (on PAUSE request)
- PAUSED -> PLAYING (on PLAY request)
- Keep-alive during PAUSED state
- Preserve timestamps

### GStreamer Integration
- Handle GST_STATE_PAUSED correctly
- Stop requesting data
- Keep connection alive
- Flush=false on resume

## Implementation Blueprint
1. Add PAUSE to RTSP methods
2. Implement pause request builder
3. Handle state change to PAUSED
4. Stop RTP reception
5. Continue session keep-alive
6. Track pause position
7. Resume with PLAY
8. Test pause/resume cycles

## Resources
- RTSP RFC 2326 Section 10.6: https://datatracker.ietf.org/doc/html/rfc2326#section-10.6
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c (PAUSE handling)
- GStreamer states: https://gstreamer.freedesktop.org/documentation/gstreamer/gststate.html

## Validation Gates
```bash
# Test PAUSE support
cargo test -p gst-plugin-rtsp vod_pause -- --nocapture

# Test pause/resume cycle
cargo test -p gst-plugin-rtsp pause_resume -- --nocapture

# Long pause test
cargo test -p gst-plugin-rtsp long_pause -- --nocapture
```

## Dependencies
- PRP-RTSP-08 (Session timeout) - for keep-alive during pause

## Estimated Effort
3 hours

## Risk Assessment
- Low complexity - straightforward protocol addition
- Challenge: Buffer management during pause

## Success Confidence Score
8/10 - Clear specification and examples available