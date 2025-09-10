# PRP-RTSP-20: Selective Stream Control

## Overview
Implement ability to selectively enable/disable specific streams (audio/video/metadata) from an RTSP source, reducing bandwidth and processing requirements.

## Current State
- Listed as missing: "Allow ignoring specific streams"
- All streams must be linked
- Cannot disable unwanted streams
- Bandwidth waste on unused streams

## Success Criteria
- [ ] Property to select desired streams
- [ ] Skip SETUP for ignored streams
- [ ] Create pads only for selected streams
- [ ] Handle dynamic stream selection
- [ ] Tests verify stream filtering

## Technical Details

### Stream Selection Methods
1. By media type (audio/video/application)
2. By stream index (0, 1, 2...)
3. By codec (H264, AAC, etc.)
4. By SDP attributes
5. Dynamic selection via signals

### Properties to Add
- select-streams (flags): audio, video, metadata
- stream-filter (string): codec filter expression
- require-all-streams (boolean): fail if not all linked

### Implementation Flow
1. Parse SDP for all streams
2. Apply selection filters
3. SETUP only selected streams
4. Create pads for selected
5. Skip ignored in pipeline

## Implementation Blueprint
1. Add stream selection properties
2. Create stream_filter module
3. Parse selection criteria
4. Modify SDP processing
5. Filter SETUP requests
6. Selective pad creation
7. Add selection signals
8. Test various filters

## Resources
- GStreamer stream selection: https://gstreamer.freedesktop.org/documentation/gstreamer/gststreams.html
- SDP media sections: https://datatracker.ietf.org/doc/html/rfc4566#section-5.14
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-base/gst/playback/gstdecodebin3.c

## Validation Gates
```bash
# Test stream selection
cargo test -p gst-plugin-rtsp stream_select -- --nocapture

# Test codec filtering
cargo test -p gst-plugin-rtsp codec_filter -- --nocapture

# Verify bandwidth savings
cargo test -p gst-plugin-rtsp selective_setup -- --nocapture
```

## Dependencies
- None (modifies existing stream handling)

## Estimated Effort
3 hours

## Risk Assessment
- Low risk - filtering reduces complexity
- Challenge: Dynamic selection changes

## Success Confidence Score
8/10 - Clear requirements with good examples