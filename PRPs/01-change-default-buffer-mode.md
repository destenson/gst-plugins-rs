# PRP: Change Default Buffer Mode to None

## Context
The rtspsrc2 element currently defaults to BufferMode::Auto, which automatically selects buffering strategies based on stream conditions. For applications requiring minimal latency, this introduces unnecessary buffering delays. The goal is to change the default to BufferMode::None for zero-buffering operation.

## Background Research
- Original rtspsrc uses `DEFAULT_BUFFER_MODE = BUFFER_MODE_AUTO` (gstrtspsrc.c:320)
- BufferMode enum defined in buffer_mode.rs with variants: None, Slave, Buffer, Auto, Synced
- Auto mode typically resolves to Buffer mode for most streams (imp.rs:4559-4562)
- rtpbin's buffer-mode property controls jitterbuffer behavior
- None mode = "Only use RTP timestamps" (no buffering)
- GStreamer documentation: https://gstreamer.freedesktop.org/documentation/rtpmanager/rtpjitterbuffer.html

## Implementation Blueprint
1. Locate BufferMode::default() implementation in buffer_mode.rs
2. Change default from BufferMode::Auto to BufferMode::None
3. Update any hardcoded Auto references in imp.rs
4. Verify rtpbin configuration logic handles None mode properly
5. Update property documentation to reflect new default
6. Test with various RTSP streams to ensure timestamps work correctly

## Affected Files
- net/rtsp/src/rtspsrc/buffer_mode.rs (line 15)
- net/rtsp/src/rtspsrc/imp.rs (line 661)

## Testing Strategy
Run existing test suite to ensure no regressions:
- Check property_tests.rs for buffer-mode property tests
- Verify integration tests still pass
- Test with live RTSP streams using mediamtx test server
- Compare latency measurements before/after change

## Validation Gates
```bash
# Format and lint check
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Run unit tests
cargo test -p gst-plugin-rtsp --all-features

# Run integration tests
cargo test -p gst-plugin-rtsp --test integration_tests
```

## Dependencies
None - this is a standalone change

## Risks & Mitigations
- **Risk**: Some streams may depend on buffering for smooth playback
- **Mitigation**: Users can explicitly set buffer-mode property if needed
- **Risk**: Timestamp-only mode may cause sync issues
- **Mitigation**: Test thoroughly with various stream types

## Success Metrics
- Default buffer-mode property returns "none" 
- No test regressions
- Measurable latency reduction in test streams

## Confidence Score: 9/10
Simple default value change with clear implementation path.