# PRP: Reduce Default Jitterbuffer Latency

## Context
The rtspsrc2 element currently uses DEFAULT_LATENCY_MS = 2000 (2 seconds), matching the original rtspsrc. This high default latency adds unnecessary delay for applications prioritizing real-time responsiveness over stream stability.

## Background Research
- Current default: 2000ms defined in imp.rs:75
- Property "latency" controls rtpbin's jitterbuffer size
- Lower latency = faster stream startup, less buffering
- Trade-off: Lower values may cause more packet drops on poor networks
- Reference: https://gstreamer.freedesktop.org/documentation/rtpmanager/rtpjitterbuffer.html#rtpjitterbuffer:latency

## Implementation Blueprint
1. Change DEFAULT_LATENCY_MS constant from 2000 to 200 (or 0 for no buffering)
2. Update property definition default value
3. Verify rtpbin receives correct latency value
4. Update property documentation with new default
5. Consider making this configurable via environment variable for testing

## Affected Files
- net/rtsp/src/rtspsrc/imp.rs (lines 75, 658, 1698)

## Testing Strategy
- Verify property default value changes correctly
- Test with high-jitter network conditions
- Measure actual latency reduction using timestamps
- Check for increased packet drops at lower values

## Validation Gates
```bash
# Build and lint
cargo build --release -p gst-plugin-rtsp
cargo clippy -p gst-plugin-rtsp -- -D warnings

# Test property defaults
cargo test -p gst-plugin-rtsp test_default_properties

# Integration test with timing
cargo test -p gst-plugin-rtsp latency_test --nocapture
```

## Dependencies
Works best when combined with BufferMode::None from PRP-01

## Risks & Mitigations
- **Risk**: Increased packet drops on poor networks
- **Mitigation**: Document trade-offs, allow runtime configuration
- **Risk**: Audio/video sync issues at very low values
- **Mitigation**: Test with 200ms first, then try lower values

## Success Metrics
- Default latency property returns new value
- Measurable reduction in end-to-end latency
- No significant increase in packet drops on good networks

## Confidence Score: 8/10
Clear implementation, but needs careful testing for optimal value.