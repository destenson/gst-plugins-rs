# PRP-RTSP-18: Basic VOD Seeking Support

## Overview
Implement basic seeking support for VOD streams using Range headers, enabling users to jump to different positions in recorded content.

## Current State
- Listed as missing: "Seeking support with VOD"
- No seek handling
- Cannot jump to positions
- Only linear playback supported

## Success Criteria
- [ ] Handle seek events from GStreamer
- [ ] Generate Range headers for PLAY
- [ ] Parse Range responses
- [ ] Update timestamps correctly
- [ ] Tests verify basic seeking

## Technical Details

### Seeking Components
1. GST seek event handling
2. Range header generation (npt, smpte, clock)
3. PLAY request with Range
4. Flush and segment handling
5. Timestamp adjustment

### Range Formats
- **npt** (Normal Play Time): "Range: npt=10-20"
- **clock**: "Range: clock=20030101T143720Z-"
- **smpte**: "Range: smpte=0:10:20-"

### Seek Handling Flow
1. Receive GST_EVENT_SEEK
2. Send PAUSE (optional)
3. Flush pipeline
4. Send PLAY with Range
5. Update segment
6. Resume playback

## Implementation Blueprint
1. Add seek event handler
2. Implement Range header formatter
3. Parse seek position/format
4. Modify PLAY request builder
5. Handle seek response
6. Update segment events
7. Add flush handling
8. Test various seek scenarios

## Resources
- RTSP Range header: https://datatracker.ietf.org/doc/html/rfc2326#section-12.29
- GStreamer seeking: https://gstreamer.freedesktop.org/documentation/gstreamer/gstsegment.html
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c (seek handling)

## Validation Gates
```bash
# Test basic seeking
cargo test -p gst-plugin-rtsp seek_basic -- --nocapture

# Test seek accuracy
cargo test -p gst-plugin-rtsp seek_accuracy -- --nocapture

# Test segment updates
cargo test -p gst-plugin-rtsp seek_segment -- --nocapture
```

## Dependencies
- PRP-RTSP-16 (PAUSE support) - may need pause before seek

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - timestamp management critical
- Challenge: Accurate segment handling

## Success Confidence Score
6/10 - Seeking is complex with many edge cases