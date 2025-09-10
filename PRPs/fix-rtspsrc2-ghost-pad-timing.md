# PRP: Fix rtspsrc2 Ghost Pad Target Timing Race Condition

## Problem Statement

rtspsrc2 has a race condition where RTP data starts flowing before ghost pads have their targets set, causing data loss even after the unlinked pad handling is fixed. Ghost pads are created immediately but only get targets when rtpbin creates source pads dynamically.

## Context & Research

### Current Architecture Issue
- **Ghost pads created**: Line ~700 in `make_rtp_appsrc()`
- **Data starts flowing**: Immediately in TCP/UDP tasks
- **Targets set**: Later in `pad_added_cb()` callback
- **Problem**: Data flows and gets lost during this timing window

### Comparison with Original rtspsrc
Analysis shows original rtspsrc:
1. **Synchronized Initialization**: Ensures pads are ready before data flows
2. **Proper Buffering**: Handles data that arrives before full pipeline setup
3. **Atomic Pad Operations**: Minimizes timing windows

## Implementation Plan

### Task 1: Analyze Current Ghost Pad Creation
- **File**: `net/rtsp/src/rtspsrc/imp.rs`
- **Function**: `make_rtp_appsrc()` around line 698
- **Goal**: Understand current ghost pad setup timing
- **Document**: Current sequence of events and timing issues

### Task 2: Research rtpbin Pad-Added Callback Timing
- **File**: `net/rtsp/src/rtspsrc/imp.rs` 
- **Function**: `pad_added_cb()`
- **Goal**: Understand when rtpbin creates source pads
- **Compare**: With how original rtspsrc handles this timing

### Task 3: Implement Early Ghost Pad Target Setting
- **Goal**: Set ghost pad targets as early as possible
- **Options**: 
  - Delay data flow until targets are set
  - Buffer data until targets are available
  - Create temporary targets that get replaced

### Task 4: Add Synchronization Between Data Flow and Pad Setup
- **Purpose**: Ensure data doesn't flow until pads are properly configured
- **Pattern**: Use locks or state flags to coordinate timing
- **Reference**: How other elements in gst-plugins-rs handle this

## Validation Gates

```bash
# Build and test
cargo build -p gst-plugin-rtsp
cargo test -p gst-plugin-rtsp

# Test rapid connect/disconnect scenarios
# Should not lose initial frames or have timing issues
gst-launch-1.0 rtspsrc2 location=rtsp://127.0.0.1:8554/test ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink

# Monitor for timing-related errors in logs
```

## Success Criteria

1. **No Lost Frames**: Initial RTP packets are not lost during pad setup
2. **Clean Transitions**: Ghost pads get targets before significant data flow
3. **No Race Conditions**: Consistent behavior across multiple test runs
4. **Reduced Latency**: Minimize delay between connection and first frame

## Dependencies

**Prerequisite**: "Fix rtspsrc2 Unlinked Pad Error Handling" must be completed first, as this addresses the next layer of timing issues.

## References

- **Ghost Pad Creation**: `net/rtsp/src/rtspsrc/imp.rs:698`
- **Pad Added Callback**: `net/rtsp/src/rtspsrc/imp.rs` pad_added_cb function
- **Original rtspsrc**: Ghost pad and rtpbin integration patterns
- **GStreamer Documentation**: Dynamic pad creation best practices

## Risk Assessment

**Medium Risk**: Timing changes can introduce new race conditions. Requires careful testing of connection sequences.

## Estimated Effort

**3-4 hours**: More complex than error handling fix, involves understanding rtpbin timing.

## Confidence Score: 7/10

Good confidence - addresses known timing issue, but requires careful implementation to avoid new race conditions.