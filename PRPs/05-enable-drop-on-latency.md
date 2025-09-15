# PRP: Enable Drop-on-Latency by Default

## Context
The drop-on-latency property tells the jitterbuffer to drop old packets rather than increasing latency. Currently defaults to false, preserving all packets at the cost of increased delay. For real-time applications, dropping old packets is preferable to accumulating delay.

## Background Research
- DEFAULT_DROP_ON_LATENCY = false in imp.rs:76
- Property passed to rtpbin's jitterbuffer
- When true, maintains constant latency by dropping late packets
- Trade-off: May cause brief glitches vs unbounded delay
- Commonly enabled in video conferencing/live streaming

## Implementation Blueprint
1. Change DEFAULT_DROP_ON_LATENCY from false to true
2. Update property documentation to explain new behavior
3. Verify rtpbin receives and applies the setting
4. Test packet drop behavior under network congestion
5. Add statistics for dropped packet counting

## Affected Files
- net/rtsp/src/rtspsrc/imp.rs (line 76, 659, 1704)

## Testing Strategy
- Simulate network congestion with packet delays
- Verify old packets are dropped, not queued
- Measure latency remains constant under load
- Check drop statistics are accurate

## Validation Gates
```bash
# Build and lint
cargo build -p gst-plugin-rtsp
cargo fmt --check

# Test drop behavior
cargo test -p gst-plugin-rtsp drop_on_latency_test

# Stress test with delays
cargo test -p gst-plugin-rtsp network_congestion_test
```

## Dependencies
Best combined with reduced latency from PRP-02

## Risks & Mitigations
- **Risk**: Video artifacts from dropped frames
- **Mitigation**: Only affects packets beyond latency window
- **Risk**: Audio dropouts
- **Mitigation**: Consider different settings for audio/video

## Success Metrics
- Latency stays constant under network stress
- Dropped packets logged appropriately
- No accumulating delay in live streams

## Confidence Score: 9/10
Simple boolean change with predictable behavior.