# PRP: Optimize RTPBin Properties for Minimal Latency

## Context
Beyond buffer-mode and latency, rtpbin has numerous properties that affect latency and performance. Configuring these optimally can further reduce delays without changing defaults.

## Background Research
- rtpbin properties: do-lost, do-retransmission, ntp-sync, rtcp-sync
- max-dropout-time and max-misorder-time affect reordering
- rtp-profile affects timing calculations
- Reference: https://gstreamer.freedesktop.org/documentation/rtpmanager/rtpbin.html

## Implementation Blueprint
1. Add configuration for key rtpbin properties:
   - do-lost = false (skip lost packet handling)
   - ntp-sync = false (skip NTP synchronization)
   - max-dropout-time = 500 (reduce from default 60000)
   - max-misorder-time = 500 (reduce from default 2000)
2. Make configurable via rtspsrc2 properties
3. Apply in rtpbin configuration
4. Test impact on latency and stability
5. Document trade-offs for each setting

## Affected Files
- net/rtsp/src/rtspsrc/imp.rs (Settings struct)
- net/rtsp/src/rtspsrc/imp.rs (apply_rtcp_settings or new method)

## Testing Strategy
- Test each property individually
- Measure latency impact
- Check for side effects (sync issues, drops)
- Test with various network conditions

## Validation Gates
```bash
# Test rtpbin configuration
GST_DEBUG=rtpbin:5 gst-launch-1.0 rtspsrc2 location=rtsp://localhost:8554/test ! fakesink

# Property verification
cargo test -p gst-plugin-rtsp rtpbin_properties_test

# Latency comparison
cargo test -p gst-plugin-rtsp latency_comparison_test
```

## Dependencies
Can be combined with zero-latency property from PRP-08

## Risks & Mitigations
- **Risk**: Loss of synchronization features
- **Mitigation**: Make each setting configurable
- **Risk**: Reduced error resilience
- **Mitigation**: Document when to use each setting

## Success Metrics
- Further latency reduction (target: <100ms additional)
- Configurable per use case
- No stability regressions

## Confidence Score: 7/10
Multiple settings to tune and test.