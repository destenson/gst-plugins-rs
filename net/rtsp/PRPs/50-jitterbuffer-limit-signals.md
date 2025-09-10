# PRP-RTSP-50: Jitterbuffer Limit Signals Implementation

## Overview
Implement jitterbuffer limit signals (`soft-limit`, `hard-limit`) to match original rtspsrc buffer monitoring capabilities and provide applications with buffer overflow notifications.

## Context
The original rtspsrc provides jitterbuffer monitoring through `soft-limit` and `hard-limit` signals that notify applications when buffer thresholds are reached. These signals are critical for adaptive streaming applications that need to monitor buffer health and adjust streaming parameters.

## Research Context
- Original rtspsrc limit signals in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- GStreamer jitterbuffer implementation and limit detection
- Buffer watermark monitoring in streaming applications
- Adaptive bitrate streaming based on buffer levels
- Network congestion detection via buffer monitoring

## Scope
This PRP implements ONLY the signal infrastructure:
1. Add `soft-limit` signal: `void (GstElement, guint stream_id)`
2. Add `hard-limit` signal: `void (GstElement, guint stream_id)`  
3. Add signal registration and parameter validation
4. Add signal emission placeholder functions

Does NOT implement:
- Actual jitterbuffer monitoring logic
- Buffer level detection mechanisms
- Limit threshold configuration
- Automatic signal emission on buffer events

## Implementation Tasks
1. Add jitterbuffer limit signal definitions to RtspSrc element class
2. Register `soft-limit` signal with stream ID parameter
3. Register `hard-limit` signal with stream ID parameter
4. Add signal emission placeholder functions
5. Add stream ID parameter validation
6. Document signal timing and usage scenarios
7. Add signal parameter type validation

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Limit signal registration and implementation
- Signal parameter validation utilities

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_limit_signals --all-targets --all-features -- --nocapture

# Limit Signal Registration Test
cargo test test_jitterbuffer_limit_signals --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
Element Signals:

  "soft-limit" :  void user_function (GstElement * object,
                                      guint arg0,
                                      gpointer user_data);

  "hard-limit" :  void user_function (GstElement * object,
                                      guint arg0,
                                      gpointer user_data);
```

## Signal Behavior Details
- **soft-limit**: Emitted when jitterbuffer reaches soft threshold (warning level)
- **hard-limit**: Emitted when jitterbuffer reaches hard threshold (critical level)

## Signal Parameter Details

### soft-limit Signal
- **object**: The rtspsrc2 element
- **arg0**: Stream index (guint) experiencing soft limit
- **return**: void
- **purpose**: Warn application of buffer fill approaching limit

### hard-limit Signal  
- **object**: The rtspsrc2 element
- **arg0**: Stream index (guint) experiencing hard limit
- **return**: void  
- **purpose**: Alert application of critical buffer overflow condition

## Application Usage Examples

### Buffer Monitoring
```c
g_signal_connect (rtspsrc, "soft-limit", G_CALLBACK (soft_limit_callback), NULL);
g_signal_connect (rtspsrc, "hard-limit", G_CALLBACK (hard_limit_callback), NULL);

static void soft_limit_callback (GstElement *element, guint stream_id, gpointer data) {
    g_print ("Stream %u: Soft buffer limit reached, consider reducing bitrate\n", stream_id);
    // Implement adaptive bitrate reduction
}

static void hard_limit_callback (GstElement *element, guint stream_id, gpointer data) {
    g_print ("Stream %u: Hard buffer limit reached, dropping frames likely\n", stream_id);
    // Implement emergency measures (pause, reduce bitrate dramatically)
}
```

### Adaptive Streaming Response
```c
static void soft_limit_callback (GstElement *element, guint stream_id, gpointer data) {
    // Request lower bitrate stream from server
    AppData *app = (AppData *)data;
    reduce_stream_bitrate (app, stream_id, 0.8); // Reduce by 20%
}

static void hard_limit_callback (GstElement *element, guint stream_id, gpointer data) {
    // Emergency response - request lowest bitrate or pause
    AppData *app = (AppData *)data;
    request_minimum_bitrate (app, stream_id);
}
```

## Buffer Limit Scenarios
- **Network congestion**: Slower network causes buffer fill
- **Processing delays**: CPU overload causes buffer accumulation  
- **Clock drift**: Sender/receiver clock mismatch causes buffering issues
- **Bandwidth mismatch**: Stream bitrate exceeds available bandwidth

## Typical Response Actions
### Soft Limit Responses
- Reduce requested stream bitrate
- Increase buffer processing speed
- Log buffer health warnings
- Prepare for potential hard limit

### Hard Limit Responses
- Request minimum bitrate stream
- Drop non-essential streams  
- Pause playback temporarily
- Alert user of network issues

## Use Cases
- **Adaptive streaming**: Automatic bitrate adjustment based on buffer health
- **Network monitoring**: Track connection quality via buffer levels
- **Quality control**: Maintain smooth playback by monitoring buffer state
- **Performance tuning**: Optimize buffer sizes based on limit frequency

## Dependencies
None - signal-only implementation with simple unsigned int parameters.

## Success Criteria
- [ ] Both limit signals visible in gst-inspect output
- [ ] Signals accept guint stream ID parameter correctly
- [ ] Signal registration succeeds without errors
- [ ] Signals can be connected from application code
- [ ] Stream ID parameter validation works correctly
- [ ] Signal emission functions are prepared (placeholder)
- [ ] No actual buffer monitoring logic implemented (out of scope)

## Risk Assessment
**LOW RISK** - Simple signal registration with single unsigned integer parameter.

## Estimated Effort
2-3 hours

## Confidence Score
8/10 - Straightforward signal implementation with well-defined parameters.