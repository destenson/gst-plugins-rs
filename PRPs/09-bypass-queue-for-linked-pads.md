# PRP: Bypass Buffer Queue for Already-Linked Pads

## Context
The current implementation always checks the buffer queue even for linked pads. Adding a fast-path that bypasses the queue entirely for already-linked pads would reduce overhead and latency.

## Background Research
- push_buffer_with_queue always attempts direct push first
- Queue only needed for NotLinked errors during setup
- Most buffers go to already-linked pads
- Checking queue state adds unnecessary overhead
- AppSrc state can be cached for fast checking

## Implementation Blueprint
1. Add linked state tracking per AppSrc
2. Cache pad link status to avoid repeated checks
3. Implement inline fast-path for linked pads
4. Skip queue mutex lock for linked pads
5. Update cache on pad link/unlink events
6. Add performance metrics for bypass rate

## Affected Files
- net/rtsp/src/rtspsrc/imp.rs (push_buffer_with_queue)
- net/rtsp/src/rtspsrc/imp.rs (BufferingAppSrc struct)

## Testing Strategy
- Benchmark buffer throughput before/after
- Verify queue still works for unlinked pads
- Test pad state transitions
- Measure CPU usage reduction

## Validation Gates
```bash
# Performance benchmark
cargo bench -p gst-plugin-rtsp buffer_push_performance

# Test linking behavior
cargo test -p gst-plugin-rtsp pad_linking_test

# CPU profiling
perf record -g cargo test -p gst-plugin-rtsp --release
```

## Dependencies
Related to PRP-03 but independent

## Risks & Mitigations
- **Risk**: Race conditions in link state
- **Mitigation**: Use atomic operations for state
- **Risk**: Memory overhead from caching
- **Mitigation**: Minimal per-pad state

## Success Metrics
- 90%+ buffers bypass queue
- Measurable CPU reduction
- No functional regressions

## Confidence Score: 7/10
Performance optimization requiring careful implementation.