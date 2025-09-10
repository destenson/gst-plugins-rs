# PRP: Improve rtspsrc2 State Transition Handling

## Problem Statement

rtspsrc2 has incomplete state transition handling that can cause issues during NULL→READY→PAUSED→PLAYING transitions. Tasks may start before the element is ready, or fail to stop cleanly during shutdown.

## Context & Research

### Current Issues
- Tasks may start before element reaches appropriate state
- Shutdown doesn't cleanly terminate all async operations
- State changes don't properly coordinate with RTSP connection lifecycle
- No proper cleanup in failure scenarios

### Comparison with Original rtspsrc
Original rtspsrc has robust state handling:
1. **Coordinated Startup**: Tasks start only in appropriate states
2. **Clean Shutdown**: All resources properly released
3. **State Validation**: Checks current state before operations
4. **Error Recovery**: Proper handling of state transition failures

## Implementation Plan

### Task 1: Audit Current State Transition Implementation
- **File**: `net/rtsp/src/rtspsrc/imp.rs`
- **Functions**: `change_state()`, startup/shutdown logic
- **Goal**: Document current state handling and identify gaps
- **Compare**: With original rtspsrc state machine

### Task 2: Improve NULL to READY Transition
- **Purpose**: Ensure proper initialization without starting data flow
- **Actions**: Initialize connections, prepare resources
- **Validation**: Ready for operation but not active

### Task 3: Improve READY to PAUSED Transition  
- **Purpose**: Establish RTSP connection and negotiate streams
- **Actions**: Connect to server, setup RTP infrastructure
- **Timing**: Prepare for data flow but don't start yet

### Task 4: Improve PAUSED to PLAYING Transition
- **Purpose**: Start actual data flow tasks
- **Actions**: Begin TCP/UDP tasks, start streaming
- **Coordination**: Ensure all components are ready

### Task 5: Improve Reverse Transitions (Shutdown)
- **Purpose**: Clean shutdown of all async operations
- **PLAYING→PAUSED**: Stop data flow, keep connections
- **PAUSED→READY**: Close RTSP connections, cleanup
- **READY→NULL**: Release all resources

### Task 6: Add State Validation Guards
- **Purpose**: Prevent operations in wrong states
- **Pattern**: Check current state before starting tasks
- **Error Handling**: Return appropriate errors for invalid state operations

## Validation Gates

```bash
# Build and test
cargo build -p gst-plugin-rtsp
cargo test -p gst-plugin-rtsp

# Test state transitions:
gst-launch-1.0 rtspsrc2 location=rtsp://127.0.0.1:8554/test ! fakesink

# Test rapid state changes:
# Create pipeline, set to PLAYING, immediately back to NULL
# Should not hang or crash

# Test error recovery:
# Invalid RTSP URL should handle state transitions gracefully
```

## Success Criteria

1. **Clean Transitions**: All state changes complete without hanging
2. **Resource Management**: No leaked connections or tasks after shutdown  
3. **Error Recovery**: Failed state transitions don't leave element in inconsistent state
4. **Performance**: State changes complete in reasonable time

## Dependencies

**Prerequisites**: Should be implemented after the core data flow fixes to avoid complicating those changes.

## References

- **GStreamer State Machine**: Documentation on element state transitions
- **Original rtspsrc**: State handling implementation
- **Other Elements**: State patterns used in gst-plugins-rs
- **Async Coordination**: How to coordinate multiple async tasks during state changes

## Risk Assessment

**Low-Medium Risk**: State handling is well-documented in GStreamer, but coordination with async tasks can be tricky.

## Estimated Effort

**3-4 hours**: Systematic review and improvement of state handling.

## Confidence Score: 7/10

Good confidence - state handling patterns are well established, main challenge is coordinating with async operations.