# PRP-RTSP-49: Backchannel Action Methods Implementation

## Overview
Implement backchannel action methods (`push-backchannel-buffer`, `push-backchannel-sample`, `set-mikey-parameter`, `remove-key`) to match original rtspsrc bidirectional communication and encryption key management capabilities.

## Context
The original rtspsrc provides backchannel actions for sending audio data (`push-backchannel-buffer`, `push-backchannel-sample`) and encryption key management (`set-mikey-parameter`, `remove-key`). These are essential for two-way communication in security systems and SRTP key management.

## Research Context
- Original rtspsrc backchannel actions in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- GstBuffer vs GstSample for media data handling
- MIKEY (Multimedia Internet KEYing) protocol for SRTP key exchange
- Backchannel audio transmission over RTSP
- ONVIF backchannel specifications for security cameras

## Scope
This PRP implements ONLY the action method infrastructure:
1. Add `push-backchannel-buffer` action: `GstFlowReturn (guint stream_id, GstBuffer *buffer)`
2. Add `push-backchannel-sample` action: `GstFlowReturn (guint stream_id, GstSample *sample)`
3. Add `set-mikey-parameter` action: `gboolean (guint stream_id, GstCaps *caps, GstPromise *promise)`
4. Add `remove-key` action: `gboolean (guint stream_id)`

Does NOT implement:
- Actual backchannel media transmission
- MIKEY protocol implementation
- SRTP key installation or removal
- Backchannel RTP session management

## Implementation Tasks
1. Add backchannel action definitions to RtspSrc element class  
2. Register `push-backchannel-buffer` action with stream ID and buffer parameters
3. Register `push-backchannel-sample` action with stream ID and sample parameters
4. Register `set-mikey-parameter` action with stream ID, caps, and promise parameters
5. Register `remove-key` action with stream ID parameter
6. Add action implementations (placeholders returning appropriate values)
7. Add parameter validation for stream IDs and media objects

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Backchannel action registration and implementation
- Media object validation utilities (GstBuffer, GstSample)

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_backchannel_actions --all-targets --all-features -- --nocapture

# Backchannel Action Test
cargo test test_backchannel_action_registration --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
Element Actions:

  "push-backchannel-buffer" -> GstFlowReturn  :  g_signal_emit_by_name (element, "push-backchannel-buffer",
                                                                        guint arg0,
                                                                        GstBuffer * arg1,
                                                                        GstFlowReturn * p_return_value);

  "push-backchannel-sample" -> GstFlowReturn  :  g_signal_emit_by_name (element, "push-backchannel-sample",
                                                                        guint arg0,
                                                                        GstSample * arg1,
                                                                        GstFlowReturn * p_return_value);

  "set-mikey-parameter" -> gboolean  :  g_signal_emit_by_name (element, "set-mikey-parameter",
                                                               guint arg0,
                                                               GstCaps * arg1,
                                                               GstPromise * arg2,
                                                               gboolean * p_return_value);

  "remove-key" -> gboolean  :  g_signal_emit_by_name (element, "remove-key",
                                                      guint arg0,
                                                      gboolean * p_return_value);
```

## Action Method Details

### push-backchannel-buffer Action
- **arg0**: Stream index (guint)
- **arg1**: Buffer with media data (GstBuffer *)
- **return**: GstFlowReturn (GST_FLOW_OK = success, GST_FLOW_ERROR = failure)

### push-backchannel-sample Action
- **arg0**: Stream index (guint)  
- **arg1**: Sample with media data and caps (GstSample *)
- **return**: GstFlowReturn (GST_FLOW_OK = success, GST_FLOW_ERROR = failure)

### set-mikey-parameter Action
- **arg0**: Stream index (guint)
- **arg1**: MIKEY capabilities (GstCaps *)
- **arg2**: Promise for async result (GstPromise *)
- **return**: boolean (true = request accepted, false = error)

### remove-key Action  
- **arg0**: Stream index (guint)
- **return**: boolean (true = key removed, false = error/not found)

## Application Usage Examples

### Send Audio Buffer
```c
GstBuffer *audio_buffer = /* create audio buffer */;
GstFlowReturn result;
g_signal_emit_by_name (rtspsrc, "push-backchannel-buffer", 0, audio_buffer, &result);
if (result != GST_FLOW_OK) {
    g_warning ("Failed to send backchannel audio");
}
```

### Send Audio Sample
```c
GstSample *audio_sample = /* create audio sample with caps */;  
GstFlowReturn result;
g_signal_emit_by_name (rtspsrc, "push-backchannel-sample", 0, audio_sample, &result);
```

### Set MIKEY Parameters
```c
GstCaps *mikey_caps = /* create MIKEY caps */;
GstPromise *promise = gst_promise_new ();
gboolean result;
g_signal_emit_by_name (rtspsrc, "set-mikey-parameter", 0, mikey_caps, promise, &result);
```

## Backchannel Use Cases
- **Security cameras**: Send audio commands to cameras
- **Intercom systems**: Two-way audio communication  
- **Access control**: Voice communication through IP intercoms
- **Remote monitoring**: Interactive audio with remote locations

## MIKEY Parameter Management
- **set-mikey-parameter**: Install SRTP encryption keys via MIKEY protocol
- **remove-key**: Remove encryption keys for stream
- Used for secure backchannel communication
- Essential for encrypted two-way audio

## GstFlowReturn Values
- **GST_FLOW_OK**: Data accepted and processed successfully
- **GST_FLOW_ERROR**: General error during processing
- **GST_FLOW_NOT_SUPPORTED**: Backchannel not available/configured
- **GST_FLOW_WRONG_STATE**: Element not in correct state for backchannel

## Dependencies
- **Media types**: GstBuffer, GstSample for audio data
- **Flow types**: GstFlowReturn for media flow control
- **Promise types**: GstPromise for MIKEY parameter results

## Success Criteria
- [ ] All four backchannel actions visible in gst-inspect output
- [ ] Actions accept correct parameter types (stream ID, media objects)
- [ ] GstFlowReturn actions work with media data parameters  
- [ ] Boolean return actions handle success/failure correctly
- [ ] Stream ID validation rejects invalid values
- [ ] Media object parameter validation works correctly
- [ ] No actual backchannel transmission implemented (out of scope)

## Risk Assessment  
**MEDIUM-HIGH RISK** - Media object handling and flow return management.

## Estimated Effort
4-5 hours (media object parameters and flow return handling)

## Confidence Score
6/10 - Media object handling and GstFlowReturn integration add complexity.