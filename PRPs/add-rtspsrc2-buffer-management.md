# PRP: Add Proper Buffer Management to rtspsrc2

## Problem Statement

After fixing unlinked pad handling and timing issues, rtspsrc2 needs proper buffer management to handle scenarios where data arrives faster than it can be consumed or when downstream elements are temporarily unavailable.

## Context & Research

### Current State
rtspsrc2 has minimal buffer management:
- No buffering when pads are unlinked
- No handling of backpressure from downstream
- No memory management for accumulated data

### Comparison with Original rtspsrc
Original rtspsrc implements:
1. **Buffer Queuing**: Maintains internal buffers during unlinked states
2. **Memory Management**: Prevents unlimited buffer accumulation  
3. **Backpressure Handling**: Responds appropriately to downstream flow control
4. **Buffer Properties**: Configurable buffer sizes and thresholds

## Implementation Plan

### Task 1: Add Buffer Queue for Unlinked States
- **File**: `net/rtsp/src/rtspsrc/imp.rs`
- **Goal**: Queue buffers when AppSrc push fails with NotLinked
- **Pattern**: Use VecDeque or similar for FIFO buffer management
- **Limit**: Maximum buffer count/size to prevent memory leaks

### Task 2: Implement Buffer Flushing on Pad Link
- **Trigger**: When ghost pad targets are set in `pad_added_cb()`
- **Action**: Flush queued buffers to newly linked pads
- **Order**: Maintain temporal order of buffered data
- **Cleanup**: Clear buffers that are too old

### Task 3: Add Memory Management and Limits
- **Purpose**: Prevent unlimited memory consumption
- **Metrics**: Track buffer count and total memory usage
- **Thresholds**: Drop oldest buffers when limits exceeded
- **Logging**: Warn when buffer limits are reached

### Task 4: Handle Downstream Backpressure
- **Scenario**: When downstream elements return Flushing or other flow errors
- **Response**: Appropriate handling based on error type
- **Pattern**: Follow GStreamer best practices for flow control

## Validation Gates

```bash
# Build and test
cargo build -p gst-plugin-rtsp
cargo test -p gst-plugin-rtsp

# Test buffer management scenarios:
# 1. Connect/disconnect rapidly
# 2. Slow downstream processing  
# 3. Memory pressure scenarios
gst-launch-1.0 rtspsrc2 location=rtsp://127.0.0.1:8554/test ! queue ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink

# Monitor memory usage during long runs
```

## Success Criteria

1. **No Memory Leaks**: Bounded memory usage during extended operation
2. **Smooth Playback**: No dropped frames due to poor buffer management
3. **Graceful Degradation**: Appropriate behavior under memory pressure
4. **Quick Recovery**: Fast resume after temporary disconnections

## Dependencies

**Prerequisites**: 
1. "Fix rtspsrc2 Unlinked Pad Error Handling" - must handle unlinked states
2. "Fix rtspsrc2 Ghost Pad Target Timing" - must have proper pad timing

## References

- **Buffer Management Patterns**: How udpsrc, tcpclientsrc handle buffering
- **GStreamer Memory**: Buffer pool and memory management best practices
- **Original rtspsrc**: Buffer handling implementation patterns
- **Flow Control**: GStreamer documentation on backpressure handling

## Risk Assessment

**Medium Risk**: Buffer management is complex and can introduce memory leaks or performance issues if not implemented carefully.

## Estimated Effort

**4-5 hours**: More complex feature requiring careful memory management and testing.

## Confidence Score: 6/10

Moderate confidence - buffer management is more complex and requires careful design to avoid introducing new issues.