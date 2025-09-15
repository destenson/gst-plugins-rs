# Zero-Latency rtspsrc2 Implementation Plan

## Overview
This document summarizes the PRPs for removing default buffering from rtspsrc2 to achieve minimal latency operation.

## Problem Statement
The rtspsrc2 element currently defaults to buffering modes that prioritize stream stability over latency:
- Default buffer-mode: Auto (usually becomes Buffer mode)
- Default latency: 2000ms 
- Default drop-on-latency: false
- No fast-start optimization

**Critical Finding**: buffer-mode=none fails to display frames, indicating some minimal buffering is required.

## Implementation Phases

### Phase 1: Safe Defaults (PRPs 06, 07)
- **PRP-06**: Investigate why buffer-mode=none fails
- **PRP-07**: Implement Slave mode as minimal-latency default

### Phase 2: Core Optimizations (PRPs 02, 05, 04)
- **PRP-02**: Reduce default latency from 2000ms to 200ms
- **PRP-05**: Enable drop-on-latency by default
- **PRP-04**: Enable fast-start mode (start after 2 packets)

### Phase 3: Performance Optimizations (PRPs 03, 09, 11)
- **PRP-03**: Optimize buffer queue for zero-copy fast path
- **PRP-09**: Bypass queue for already-linked pads
- **PRP-11**: Remove buffer pool allocation overhead

### Phase 4: Advanced Features (PRPs 08, 10)
- **PRP-08**: Add zero-latency property for one-click configuration
- **PRP-10**: Optimize additional rtpbin properties

### Phase 5: Validation (PRP 12)
- **PRP-12**: Create comprehensive latency benchmarking suite

## Expected Outcomes
- Default latency reduced from 2000ms to <200ms
- First frame appears immediately (fast-start)
- Constant latency under network stress (drop-on-latency)
- Simple configuration via zero-latency property
- No frame display issues (using Slave mode, not None)

## Testing Strategy
Each PRP includes specific validation gates. Overall testing:
1. Unit tests for each property change
2. Integration tests with real RTSP streams
3. Benchmarks comparing before/after
4. Regression tests in CI

## Risk Mitigation
- Keep original defaults available via properties
- Document trade-offs for each optimization
- Provide zero-latency property for easy switching
- Extensive testing with various stream types

## Implementation Order
Recommended order based on dependencies:
1. PRP-06 (investigate None mode issue)
2. PRP-07 (Slave mode default)
3. PRP-02 (reduce latency)
4. PRP-05 (drop-on-latency)
5. PRP-04 (fast-start)
6. PRP-12 (benchmarking)
7. PRP-08 (zero-latency property)
8. PRP-03, 09, 11 (performance optimizations)
9. PRP-10 (rtpbin fine-tuning)

## Success Metrics
- End-to-end latency: <200ms (from 2000ms+)
- Time to first frame: <100ms (from 2000ms+)
- CPU usage: Reduced by 20%+
- Memory usage: Reduced by 10%+
- All existing tests pass
- No frame display issues