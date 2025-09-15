# PRP: Remove Buffer Pool Allocation Overhead

## Context
The buffer_pool.rs module may introduce allocation overhead. For zero-latency operation, we should investigate if buffer pooling can be optimized or bypassed for the fast path.

## Background Research
- buffer_pool.rs exists but may not be actively used
- GStreamer buffer pools can add latency during allocation
- Direct buffer allocation might be faster for small buffers
- Memory allocation is often a bottleneck in streaming

## Implementation Blueprint
1. Profile current buffer allocation paths
2. Identify if buffer pool is used in RTP path
3. Add option to bypass pool for small buffers
4. Implement direct allocation fast path
5. Compare memory usage and allocation time
6. Add metrics for allocation performance

## Affected Files
- net/rtsp/src/rtspsrc/buffer_pool.rs
- net/rtsp/src/rtspsrc/imp.rs (buffer allocation sites)

## Testing Strategy
- Profile allocation hotspots
- Measure allocation time per buffer
- Test memory fragmentation
- Check for memory leaks

## Validation Gates
```bash
# Memory profiling
valgrind --tool=massif cargo test -p gst-plugin-rtsp

# Allocation benchmarks
cargo bench -p gst-plugin-rtsp buffer_allocation

# Stress test
cargo test -p gst-plugin-rtsp allocation_stress_test
```

## Dependencies
Independent optimization

## Risks & Mitigations
- **Risk**: Increased memory fragmentation
- **Mitigation**: Use pool for large buffers only
- **Risk**: Memory leaks without pool
- **Mitigation**: Careful lifecycle management

## Success Metrics
- Reduced allocation time
- Lower memory overhead
- No memory leaks

## Confidence Score: 6/10
Requires profiling to determine impact.