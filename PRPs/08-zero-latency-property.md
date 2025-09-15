# PRP: Add Zero-Latency Property for One-Click Optimization

## Context
Instead of requiring users to configure multiple properties for minimal latency, provide a single "zero-latency" boolean property that configures all settings optimally for lowest possible latency.

## Background Research
- Similar to x264enc's "zero-latency" tuning preset
- Would configure: buffer-mode, latency, drop-on-latency, faststart
- Provides easy way to switch between low-latency and stable modes
- Common pattern in GStreamer elements for preset configurations

## Implementation Blueprint
1. Add zero_latency boolean property (default: false for compatibility)
2. When true, override these settings:
   - buffer_mode = Slave
   - latency_ms = 0 (or minimal working value)
   - drop_on_latency = true
   - faststart_min_packets = 1
3. Apply settings in READY->PAUSED transition
4. Document the exact settings it applies
5. Allow individual property overrides after zero-latency

## Affected Files
- net/rtsp/src/rtspsrc/imp.rs (Settings struct)
- net/rtsp/src/rtspsrc/imp.rs (property definitions)
- net/rtsp/src/rtspsrc/imp.rs (state change handling)

## Testing Strategy
- Verify all settings applied correctly
- Test latency with zero-latency=true
- Ensure individual overrides still work
- Compare with manual configuration

## Validation Gates
```bash
# Test zero-latency mode
gst-launch-1.0 rtspsrc2 location=rtsp://localhost:8554/test zero-latency=true ! decodebin ! autovideosink

# Verify properties
gst-inspect-1.0 rtspsrc2 | grep zero-latency

# Integration test
cargo test -p gst-plugin-rtsp zero_latency_test
```

## Dependencies
Depends on findings from PRP-06 for optimal values

## Risks & Mitigations
- **Risk**: May not work for all stream types
- **Mitigation**: Document limitations and use cases
- **Risk**: Conflicts with individual properties
- **Mitigation**: Clear precedence rules

## Success Metrics
- Single property configures all settings
- Achieves minimal latency
- No loss of functionality

## Confidence Score: 9/10
Clear UX improvement with straightforward implementation.