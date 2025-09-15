# PRP: Investigate and Fix Buffer-Mode None Frame Display Issue

## Context
User reports that setting buffer-mode=none results in no frames being displayed. This suggests that some minimal buffering/synchronization is required for proper frame delivery. Need to investigate why None mode fails and implement a proper minimal-buffering solution.

## Background Research
- BufferMode::None means "Only use RTP timestamps" 
- May break synchronization between audio/video
- rtpjitterbuffer might not output frames without minimal buffering
- Original rtspsrc implementation: check how None mode is handled
- Reference: https://gitlab.freedesktop.org/gstreamer/gstreamer/-/issues

## Implementation Blueprint
1. Create test case reproducing the None mode issue
2. Add debug logging to trace buffer flow in None mode
3. Check rtpbin's handling of buffer-mode=0 (None)
4. Investigate if rtpjitterbuffer requires minimum configuration
5. Identify minimal buffering requirements for frame display
6. Document the actual behavior vs expected behavior

## Affected Files
- net/rtsp/src/rtspsrc/imp.rs (apply_buffer_mode method)
- net/rtsp/tests/ (new test file for buffer modes)

## Testing Strategy
- Test each buffer mode with simple RTSP stream
- Log buffer flow at each stage (AppSrc -> rtpbin -> output)
- Compare timestamps and buffer counts
- Use GST_DEBUG=rtpjitterbuffer:6 for detailed logs

## Validation Gates
```bash
# Test all buffer modes
for mode in none slave buffer auto synced; do
  GST_DEBUG=rtpjitterbuffer:6,rtspsrc2:6 gst-launch-1.0 rtspsrc2 location=rtsp://localhost:8554/test buffer-mode=$mode ! fakesink
done

# Run specific buffer mode tests
cargo test -p gst-plugin-rtsp buffer_mode_none_test --nocapture
```

## Dependencies
Blocks PRP-01 (changing default to None)

## Risks & Mitigations
- **Risk**: None mode may be fundamentally broken
- **Mitigation**: Find minimal working configuration instead
- **Risk**: Issue may be in rtpbin, not rtspsrc2
- **Mitigation**: Test with manual pipeline construction

## Success Metrics
- Identify root cause of None mode failure
- Document minimal buffering requirements
- Implement working low-latency alternative

## Confidence Score: 6/10
Requires investigation before implementation.