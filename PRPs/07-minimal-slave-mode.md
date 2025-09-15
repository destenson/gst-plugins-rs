# PRP: Implement Minimal Slave Mode as Default

## Context
Since BufferMode::None doesn't display frames, we need an alternative minimal-buffering mode. Slave mode synchronizes to sender clock with minimal buffering, making it a better choice for low-latency than Auto or Buffer modes.

## Background Research
- BufferMode::Slave = "Slave receiver to sender clock"
- Less buffering than Buffer mode, more reliable than None
- Used in original rtspsrc when use_buffering=false
- Should provide good balance of latency and reliability
- See gstrtspsrc.c:4215 for slave mode selection logic

## Implementation Blueprint
1. Change default from BufferMode::Auto to BufferMode::Slave
2. Ensure slave mode works with reduced latency settings
3. Test synchronization with sender clock
4. Verify frames display correctly
5. Compare latency with Auto mode
6. Document the synchronization behavior

## Affected Files
- net/rtsp/src/rtspsrc/buffer_mode.rs (default implementation)
- net/rtsp/src/rtspsrc/imp.rs (buffer_mode initialization)

## Testing Strategy
- Test with various RTSP streams
- Measure end-to-end latency
- Verify A/V sync is maintained
- Check frame delivery consistency

## Validation Gates
```bash
# Test with slave mode
gst-launch-1.0 rtspsrc2 location=rtsp://localhost:8554/test buffer-mode=slave ! decodebin ! autovideosink

# Run tests
cargo test -p gst-plugin-rtsp --all-features

# Latency measurement
GST_DEBUG=rtpjitterbuffer:5 cargo test -p gst-plugin-rtsp slave_mode_latency
```

## Dependencies
Alternative to PRP-01, replaces None with Slave

## Risks & Mitigations
- **Risk**: May still have more latency than desired
- **Mitigation**: Combine with other optimizations
- **Risk**: Clock sync issues with some senders
- **Mitigation**: Test with various RTSP servers

## Success Metrics
- Frames display correctly
- Lower latency than Auto/Buffer modes
- Stable synchronization

## Confidence Score: 8/10
Proven mode with better compatibility than None.