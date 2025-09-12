# PRP-RTSP-48: RTSP Action Methods Implementation

## Overview
Implement RTSP action methods (`get-parameter`, `get-parameters`, `set-parameter`) to match original rtspsrc server parameter control capabilities. These actions allow applications to send GET_PARAMETER and SET_PARAMETER RTSP requests.

## Context
The original rtspsrc provides RTSP action methods for server parameter manipulation: `get-parameter` (single parameter), `get-parameters` (multiple parameters), and `set-parameter` (parameter setting). These actions are essential for camera configuration and server control via RTSP protocol extensions.

## Research Context
- Original rtspsrc actions in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- RTSP RFC 2326 GET_PARAMETER and SET_PARAMETER methods
- GstPromise for asynchronous action results
- Parameter name/value string formatting
- ONVIF parameter control extensions

## Scope
This PRP implements ONLY the action method infrastructure:
1. Add `get-parameter` action: `gboolean (const gchar *param, const gchar *content_type, GstPromise *promise)`
2. Add `get-parameters` action: `gboolean (const gchar **params, const gchar *content_type, GstPromise *promise)`  
3. Add `set-parameter` action: `gboolean (const gchar *param, const gchar *value, const gchar *content_type, GstPromise *promise)`
4. Add action registration and parameter validation

Does NOT implement:
- Actual GET_PARAMETER/SET_PARAMETER request generation
- RTSP server communication for parameters
- Parameter value parsing or validation
- Promise fulfillment with results

## Implementation Tasks
1. Add action method definitions to RtspSrc element class
2. Register `get-parameter` action with parameter, content-type, and promise arguments
3. Register `get-parameters` action with parameter array and promise arguments
4. Register `set-parameter` action with parameter, value, and promise arguments
5. Add action method implementations (placeholders returning false)
6. Add parameter name validation and null checking
7. Document action usage and parameter formats

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Action method registration and implementation
- Action parameter validation utilities

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_action_methods --all-targets --all-features -- --nocapture

# Action Registration Test
cargo test test_rtsp_action_registration --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
Element Actions:

  "get-parameter" -> gboolean  :  g_signal_emit_by_name (element, "get-parameter",
                                                         const gchar * arg0,
                                                         const gchar * arg1,
                                                         GstPromise * arg2,
                                                         gboolean * p_return_value);

  "get-parameters" -> gboolean  :  g_signal_emit_by_name (element, "get-parameters",
                                                          const gchar ** arg0,
                                                          const gchar * arg1,
                                                          GstPromise * arg2,
                                                          gboolean * p_return_value);

  "set-parameter" -> gboolean  :  g_signal_emit_by_name (element, "set-parameter",
                                                         const gchar * arg0,
                                                         const gchar * arg1,
                                                         const gchar * arg2,
                                                         GstPromise * arg3,
                                                         gboolean * p_return_value);
```

## Action Method Details

### get-parameter Action
- **arg0**: Parameter name (const gchar *)
- **arg1**: Content type (const gchar *, can be NULL)
- **arg2**: Promise for async result (GstPromise *)
- **return**: boolean (true = request sent, false = error)

### get-parameters Action  
- **arg0**: Parameter names array (const gchar **)
- **arg1**: Content type (const gchar *, can be NULL)
- **arg2**: Promise for async result (GstPromise *)
- **return**: boolean (true = request sent, false = error)

### set-parameter Action
- **arg0**: Parameter name (const gchar *)
- **arg1**: Parameter value (const gchar *)
- **arg2**: Content type (const gchar *, can be NULL)  
- **arg3**: Promise for async result (GstPromise *)
- **return**: boolean (true = request sent, false = error)

## Application Usage Examples

### Single Parameter Get
```c
GstPromise *promise = gst_promise_new ();
gboolean result;
g_signal_emit_by_name (rtspsrc, "get-parameter", "position", NULL, promise, &result);
```

### Multiple Parameters Get
```c
const gchar *params[] = {"position", "scale", NULL};
GstPromise *promise = gst_promise_new ();
gboolean result;  
g_signal_emit_by_name (rtspsrc, "get-parameters", params, NULL, promise, &result);
```

### Parameter Set
```c
GstPromise *promise = gst_promise_new ();
gboolean result;
g_signal_emit_by_name (rtspsrc, "set-parameter", "position", "0", NULL, promise, &result);
```

## Parameter Examples
Common RTSP parameters:
- **position**: Current playback position
- **scale**: Playback speed scale factor
- **volume**: Audio volume level
- **brightness**: Video brightness (cameras)
- **contrast**: Video contrast (cameras)

## Promise Handling
- Actions return immediately with boolean success/failure
- Actual results delivered asynchronously via GstPromise
- Applications must handle promise results in callbacks
- Promise contains RTSP response data or error information

## Dependencies
- **GstPromise**: For asynchronous result handling
- **Parameter validation**: String validation utilities

## Success Criteria
- [ ] All three actions visible in gst-inspect output
- [ ] Action signatures match original rtspsrc exactly
- [ ] Actions can be called via g_signal_emit_by_name
- [ ] Parameter validation rejects null/empty names
- [ ] Actions return boolean indicating immediate success/failure
- [ ] Promise parameter handling works correctly
- [ ] No actual RTSP request logic implemented (out of scope)

## Risk Assessment
**MEDIUM RISK** - Action registration and GstPromise parameter handling.

## Estimated Effort
3-4 hours (action registration and parameter validation)

## Confidence Score
7/10 - Action system integration with promise handling adds moderate complexity.