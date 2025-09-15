# PRP: Optimize Buffer Queue for Zero-Copy Fast Path

## Context
The BufferQueue implementation in imp.rs currently queues buffers when AppSrc pads are unlinked. For minimal latency, we should optimize this to avoid queuing when possible and add a fast-path for immediate buffer delivery.

## Background Research
- BufferQueue defined in imp.rs:326-493
- Used for handling NotLinked flow errors
- MAX_QUEUE_BUFFERS = 1000, MAX_QUEUE_BYTES = 10MB
- BufferingAppSrc wrapper at line 495
- Queue is checked on every push_buffer_with_queue call

## Implementation Blueprint
1. Add bypass flag to skip queuing when latency-critical
2. Implement zero-copy fast path in push_buffer_with_queue
3. Add property "buffer-queue-enabled" (default: false for low latency)
4. Skip queue allocation when disabled
5. Direct push to AppSrc when queue disabled
6. Add metrics for queue bypass rate

## Affected Files
- net/rtsp/src/rtspsrc/imp.rs (BufferQueue implementation)
- net/rtsp/src/rtspsrc/imp.rs (push_buffer_with_queue method)

## Testing Strategy
- Verify buffers pass through without queuing when disabled
- Test NotLinked error handling still works
- Measure memory usage reduction
- Compare CPU usage with/without queue

## Validation Gates
```bash
# Build with optimizations
cargo build --release -p gst-plugin-rtsp

# Test buffer flow
cargo test -p gst-plugin-rtsp buffer_queue_test

# Benchmark throughput
cargo bench -p gst-plugin-rtsp buffer_throughput
```

## Dependencies
None - independent optimization

## Risks & Mitigations
- **Risk**: Dropped buffers during pad linking
- **Mitigation**: Keep minimal queue for transition periods
- **Risk**: Memory leaks if buffers not freed
- **Mitigation**: Ensure proper buffer lifecycle management

## Success Metrics
- Zero allocations in fast path
- Reduced memory footprint
- Lower CPU usage for buffer handling

## Confidence Score: 7/10
Requires careful implementation to maintain compatibility.