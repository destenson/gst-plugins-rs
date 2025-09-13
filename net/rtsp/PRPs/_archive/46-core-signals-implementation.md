# PRP-RTSP-46: Core Signals Implementation

## Overview
Implement core element signals (`on-sdp`, `select-stream`, `new-manager`) to match original rtspsrc callback capabilities and provide application control over stream setup and SDP processing.

## Context
The original rtspsrc provides critical signals for application interaction: `on-sdp` (SDP inspection/modification), `select-stream` (stream selection control), and `new-manager` (RTP manager configuration). These signals are essential for applications that need fine-grained control over RTSP session setup.

## Research Context
- Original rtspsrc signals in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- GStreamer signal system: https://gstreamer.freedesktop.org/documentation/gstobject/#signals  
- SDP (Session Description Protocol) structure and modification
- Stream selection mechanisms in media streaming
- RTP session manager (rtpbin) configuration

## Scope
This PRP implements ONLY the signal infrastructure:
1. Add `on-sdp` signal: `void (GstElement, GstSDPMessage)`
2. Add `select-stream` signal: `gboolean (GstElement, guint, GstCaps)`  
3. Add `new-manager` signal: `void (GstElement, GstElement)`
4. Add signal registration and emission points (placeholder)

Does NOT implement:
- Actual SDP parsing or processing
- Stream selection logic implementation
- RTP manager creation or configuration
- Signal emission at correct pipeline stages

## Implementation Tasks
1. Add signal definitions to RtspSrc element class initialization
2. Register `on-sdp` signal with GstSDPMessage parameter
3. Register `select-stream` signal with boolean return and stream/caps parameters  
4. Register `new-manager` signal with GstElement manager parameter
5. Add signal emission placeholder functions
6. Document signal usage scenarios and parameter meanings
7. Add signal parameter validation

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Signal registration and class initialization
- Signal parameter type definitions and validation

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests  
cargo test rtspsrc_core_signals --all-targets --all-features -- --nocapture

# Signal Registration Test
cargo test test_signal_registration --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
Element Signals:

  "on-sdp" :  void user_function (GstElement * object,
                                  GstSDPMessage * arg0,
                                  gpointer user_data);

  "select-stream" :  gboolean user_function (GstElement * object,
                                             guint arg0,
                                             GstCaps * arg1,
                                             gpointer user_data);

  "new-manager" :  void user_function (GstElement * object,
                                       GstElement * arg0,
                                       gpointer user_data);
```

## Signal Behavior Details
- **on-sdp**: Emitted when SDP is received, allows inspection/modification before stream setup
- **select-stream**: Emitted for each stream, return true to select stream for playback  
- **new-manager**: Emitted when RTP manager (rtpbin) is created, allows configuration

## Signal Parameter Details

### on-sdp Signal
- **object**: The rtspsrc2 element
- **arg0**: GstSDPMessage containing session description
- **return**: void
- **purpose**: Allow application to inspect/modify SDP before processing

### select-stream Signal  
- **object**: The rtspsrc2 element
- **arg0**: Stream index (unsigned int)
- **arg1**: Stream capabilities (GstCaps)
- **return**: boolean (true = select stream, false = skip stream)
- **purpose**: Allow application to choose which streams to receive

### new-manager Signal
- **object**: The rtspsrc2 element  
- **arg0**: RTP session manager element (typically rtpbin)
- **return**: void
- **purpose**: Allow application to configure RTP manager properties

## Application Usage Examples

### SDP Inspection
```c
g_signal_connect (rtspsrc, "on-sdp", G_CALLBACK (on_sdp_callback), NULL);

static void on_sdp_callback (GstElement *element, GstSDPMessage *sdp, gpointer data) {
    // Inspect SDP contents, modify if needed
    const gchar *session_name = gst_sdp_message_get_session_name (sdp);
    g_print ("Session: %s\n", session_name);
}
```

### Stream Selection  
```c
g_signal_connect (rtspsrc, "select-stream", G_CALLBACK (select_stream_callback), NULL);

static gboolean select_stream_callback (GstElement *element, guint stream_id, GstCaps *caps, gpointer data) {
    // Return TRUE to select stream, FALSE to skip
    return stream_id == 0; // Only select first stream
}
```

## Dependencies
- **GStreamer types**: GstSDPMessage, GstCaps, GstElement
- **Signal system**: GObject signal registration and emission

## Success Criteria
- [ ] All three signals visible in gst-inspect output  
- [ ] Signal parameter types match original rtspsrc exactly
- [ ] Signal registration succeeds without errors
- [ ] Signals can be connected from application code
- [ ] Signal emission functions are prepared (placeholder)
- [ ] No actual signal emission logic implemented (out of scope)

## Risk Assessment
**MEDIUM RISK** - Signal system integration and parameter type handling.

## Estimated Effort
3-4 hours (signal registration and type handling)

## Confidence Score
7/10 - Signal system requires careful parameter type management.