# PRP: Fix rtspsrc2 Unlinked Pad Error Handling

## Problem Statement

The rtspsrc2 element fails to stream RTP data because it treats unlinked pad errors as fatal, causing data flow tasks to terminate immediately. This creates the "pad has no peer" issue where RTSP negotiation succeeds but no media data flows.

## Context & Research

### Current Issue
- **Location**: `net/rtsp/src/rtspsrc/imp.rs:1813`
- **TODO Comment**: "TODO: Allow unlinked source pads" 
- **Symptom**: Endless "pad has no peer" messages, no video/audio output
- **Root Cause**: `appsrc.push_buffer()` fails with `FlowError::NotLinked` when ghost pads have no targets set yet

### Comparison with Original rtspsrc
Analysis of `C:\Users\deste\repos\gstreamer\subprojects\gst-plugins-good\gst\rtsp\gstrtspsrc.c` shows:

1. **Graceful Unlinked Handling**: Original rtspsrc continues data flow even when pads are unlinked
2. **Standard GStreamer Pattern**: Logs unlinked/flushing states but doesn't treat as fatal
3. **Buffer Management**: Drops or buffers data until downstream connections are established

### Architecture Problem
Current rtspsrc2 flow:
```
RTSP Server → TCP/UDP Tasks → AppSrc → RTP Manager → Ghost Pads → (Unlinked)
                                ↑
                          Fails here with FlowError::NotLinked
```

## Implementation Plan

### Task 1: Update Error Handling in TCP Task
- **File**: `net/rtsp/src/rtspsrc/imp.rs`
- **Function**: `tcp_task()` around line 1813
- **Change**: Replace fatal error handling with standard GStreamer pattern
- **Pattern**: Match on specific FlowErrors and handle gracefully

### Task 2: Update Error Handling in UDP RTP Task  
- **File**: `net/rtsp/src/rtspsrc/imp.rs`
- **Function**: `udp_rtp_task()` 
- **Change**: Apply same error handling pattern for UDP transport
- **Pattern**: Continue data flow for NotLinked/Flushing errors

### Task 3: Update Error Handling in UDP RTCP Task
- **File**: `net/rtsp/src/rtspsrc/imp.rs` 
- **Function**: `udp_rtcp_task()`
- **Change**: Apply consistent error handling for RTCP data
- **Pattern**: Handle unlinked RTCP pads gracefully

### Task 4: Add Debug Logging for Flow States
- **Purpose**: Help diagnose future flow issues
- **Pattern**: Log when pads transition from unlinked to linked
- **Location**: All task functions that call `push_buffer()`

## Validation Gates

```bash
# Build and test
cargo build -p gst-plugin-rtsp
cargo test -p gst-plugin-rtsp

# Manual validation with MediaMTX test stream
# Should see video output instead of "pad has no peer" messages
gst-launch-1.0 rtspsrc2 location=rtsp://127.0.0.1:8554/test ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink
```

## Success Criteria

1. **No Fatal Exits**: Data flow tasks continue running when pads are unlinked
2. **Video Output**: Pipeline produces actual video frames from RTSP stream  
3. **Clean Logs**: Replace "pad has no peer" spam with appropriate debug messages
4. **State Transitions**: Element handles NULL->READY->PAUSED->PLAYING correctly

## References

- **Original rtspsrc**: `C:\Users\deste\repos\gstreamer\subprojects\gst-plugins-good\gst\rtsp\gstrtspsrc.c`
- **GStreamer Flow Patterns**: How udpsrc, tcpclientsrc handle unlinked pads
- **Issue Location**: `net/rtsp/src/rtspsrc/imp.rs:1813` TODO comment
- **rtspsrc2 Status**: Only 14% feature parity per IMPLEMENTATION_STATUS.md

## Risk Assessment

**Low Risk**: This is a critical bug fix addressing a known TODO item. The change follows standard GStreamer patterns used throughout the codebase.

## Estimated Effort

**2-3 hours**: Straightforward error handling fix with established patterns.

## Confidence Score: 9/10

High confidence - this addresses the root cause identified through comparison with working rtspsrc implementation.