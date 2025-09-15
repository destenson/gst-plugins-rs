# PRP: Enable RTPBin Fast Start Mode

## Context
The rtpjitterbuffer element within rtpbin supports a "faststart-min-packets" property that allows immediate playback without waiting for the full latency period. This can significantly reduce startup time for low-latency applications.

## Background Research
- rtpbin creates internal rtpjitterbuffer elements
- faststart-min-packets property allows starting after N packets
- Default is 0 (disabled), waits for full latency period
- Property documented at: https://gstreamer.freedesktop.org/documentation/rtpmanager/rtpjitterbuffer.html
- Original rtspsrc doesn't set this by default

## Implementation Blueprint
1. Add faststart_min_packets field to Settings struct
2. Set default value to 2 (start after 2 packets)
3. Add property "faststart-min-packets" to rtspsrc2
4. Pass value to rtpbin during configuration
5. Connect to "request-jitterbuffer" signal for per-stream config
6. Test with various packet rates

## Affected Files
- net/rtsp/src/rtspsrc/imp.rs (Settings struct)
- net/rtsp/src/rtspsrc/imp.rs (apply_buffer_mode or apply_rtcp_settings)
- net/rtsp/src/rtspsrc/imp.rs (property definitions)

## Testing Strategy
- Verify jitterbuffer starts after minimal packets
- Measure time to first frame
- Test with slow-start streams
- Check for glitches in early playback

## Validation Gates
```bash
# Build and check
cargo build -p gst-plugin-rtsp
cargo clippy -p gst-plugin-rtsp -- -D warnings

# Test fast start behavior
GST_DEBUG=rtpjitterbuffer:5 cargo test -p gst-plugin-rtsp faststart_test

# Integration test
cargo test -p gst-plugin-rtsp --test integration_tests startup_latency
```

## Dependencies
Works best with reduced latency from PRP-02

## Risks & Mitigations
- **Risk**: Early frames may have poor timing
- **Mitigation**: Use small value (2-3 packets) initially
- **Risk**: Incompatible with some stream types
- **Mitigation**: Make configurable, test with various codecs

## Success Metrics
- First frame appears faster than latency period
- No quality degradation in steady state
- Startup time reduced by 50% or more

## Confidence Score: 8/10
Well-defined property with clear benefits.