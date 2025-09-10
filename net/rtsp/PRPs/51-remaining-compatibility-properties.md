# PRP-RTSP-51: Remaining Compatibility Properties Implementation

## Overview
Implement remaining compatibility properties (`short-header`, `debug`, `use-pipeline-clock`, `client-managed-mikey`) to match original rtspsrc server compatibility and legacy support capabilities.

## Context
The original rtspsrc provides various compatibility properties for working with older or non-standard servers: `short-header` (minimal headers for broken encoders), `debug` (deprecated message logging), `use-pipeline-clock` (deprecated clock usage), and `client-managed-mikey` (SRTP key management mode).

## Research Context
- Original rtspsrc compatibility properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- RTSP header compatibility with broken encoders/servers
- GStreamer logging integration for debug messages
- Pipeline clock vs NTP time usage in RTCP
- Client-managed MIKEY mode for SRTP key exchange

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `short-header` boolean property (default: false)
2. Add `debug` boolean property (default: false, deprecated)
3. Add `use-pipeline-clock` boolean property (default: false, deprecated)  
4. Add `client-managed-mikey` boolean property (default: false)
5. Add property validation and deprecation warnings

Does NOT implement:
- Actual header reduction logic
- Debug message output implementation
- Pipeline clock integration
- MIKEY key management protocol

## Implementation Tasks
1. Add compatibility fields to RtspSrcSettings struct
2. Implement `short-header` boolean property for encoder compatibility
3. Implement `debug` property with deprecation warning
4. Implement `use-pipeline-clock` property with deprecation warning
5. Implement `client-managed-mikey` boolean property
6. Add property change restrictions (changeable only in NULL or READY state)
7. Add deprecation documentation and alternative recommendations

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Compatibility property definitions
- Property deprecation warning system

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_compatibility_properties --all-targets --all-features -- --nocapture

# Deprecation Warning Test
cargo test test_deprecated_property_warnings --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
short-header        : Only send the basic RTSP headers for broken encoders
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: false

debug               : Dump request and response messages to stdout(DEPRECATED: Printed all RTSP message to gstreamer log as 'log' level)
                      flags: readable, writable, deprecated, changeable only in NULL or READY state
                      Boolean. Default: false

use-pipeline-clock  : Use the pipeline running-time to set the NTP time in the RTCP SR messages(DEPRECATED: Use ntp-time-source property)
                      flags: readable, writable, deprecated, changeable only in NULL or READY state
                      Boolean. Default: false

client-managed-mikey: Enable client-managed MIKEY mode
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: false
```

## Property Behavior Details
- **short-header**: Send only essential RTSP headers for compatibility with broken encoders
- **debug**: Legacy debug message output (deprecated, use GStreamer logging instead)
- **use-pipeline-clock**: Legacy clock usage (deprecated, use ntp-time-source property)
- **client-managed-mikey**: Enable client-controlled SRTP key management via MIKEY

## Compatibility Scenarios

### short-header Usage
- **Broken encoders**: Some encoders fail with full RTSP headers
- **Minimal protocol**: Reduces message size for bandwidth-constrained networks
- **Legacy devices**: Older RTSP implementations with limited header parsing

### Deprecated Properties
- **debug**: Originally dumped messages to stdout, now use GST_DEBUG logging
- **use-pipeline-clock**: Use `ntp-time-source` property with "running-time" or "clock-time" instead

### MIKEY Client Mode
- **Key exchange**: Client proposes encryption keys instead of server
- **Re-keying**: Allows periodic key updates for security
- **Axis cameras**: Some cameras require client-managed key mode

## Deprecation Handling
```rust
// Example deprecation warning implementation
if debug_enabled {
    gst::warning!(CAT, obj: element, 
        "debug property is deprecated. Use GST_DEBUG=rtspsrc2:LOG instead");
}

if use_pipeline_clock {
    gst::warning!(CAT, obj: element,
        "use-pipeline-clock is deprecated. Use ntp-time-source property instead");
}
```

## Property Migration Guide
- **debug** → Use `GST_DEBUG=rtspsrc2:LOG` environment variable
- **use-pipeline-clock** → Set `ntp-time-source` to "running-time" or "clock-time"

## Application Examples

### Broken Encoder Compatibility
```c
g_object_set (rtspsrc, "short-header", TRUE, NULL);
// Now sends minimal RTSP headers for compatibility
```

### Client-Managed SRTP Keys
```c
g_object_set (rtspsrc, "client-managed-mikey", TRUE, NULL);
// Client will propose SRTP keys via MIKEY protocol
```

## MIKEY Protocol Integration
- Client-managed mode: Client generates and proposes encryption keys
- Server-managed mode (default): Server generates and provides keys
- Re-keying: Periodic key updates for enhanced security
- Compatible with SRTP/SRTCP encrypted streaming

## Dependencies
- **Logging system**: For deprecation warnings
- **Property flags**: For marking properties as deprecated

## Success Criteria
- [ ] All four compatibility properties visible in gst-inspect output
- [ ] Deprecated properties show deprecation flag and warning text
- [ ] Boolean properties accept true/false values correctly
- [ ] Properties changeable only in NULL/READY states
- [ ] Property defaults match original rtspsrc exactly
- [ ] Deprecation warnings logged when deprecated properties are used
- [ ] No actual compatibility logic implemented (out of scope)

## Risk Assessment
**LOW RISK** - Simple boolean properties with deprecation handling.

## Estimated Effort
2-3 hours (including deprecation warning system)

## Confidence Score
8/10 - Straightforward boolean properties with well-established deprecation patterns.