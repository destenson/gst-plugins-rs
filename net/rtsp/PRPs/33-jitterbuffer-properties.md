# PRP-RTSP-33: Jitterbuffer Control Properties Implementation

## Overview
Implement jitterbuffer control properties (`latency`, `drop-on-latency`, `probation`) to match original rtspsrc buffering capabilities. These properties control network jitter handling and buffer management.

## Context
The original rtspsrc provides extensive control over jitterbuffer behavior through properties like `latency` (buffer duration), `drop-on-latency` (strict latency enforcement), and `probation` (packet sequence validation). rtspsrc2 currently lacks these critical buffering controls.

## Research Context
- Original rtspsrc jitterbuffer properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- GStreamer RTP jitterbuffer documentation: https://gstreamer.freedesktop.org/documentation/rtpmanager/rtpjitterbuffer.html
- RTP jitter buffer implementation details and property effects
- Current rtspsrc2 structure in `net/rtsp/src/rtspsrc/imp.rs`

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `latency` property (milliseconds, default: 2000)
2. Add `drop-on-latency` property (boolean, default: false)  
3. Add `probation` property (unsigned int, default: 2)
4. Add property storage and change validation
5. Prepare property values for future jitterbuffer configuration

Does NOT implement:
- Actual jitterbuffer creation or configuration
- RTP session manager integration  
- Buffer drop logic implementation
- Packet sequence validation

## Implementation Tasks
1. Add jitterbuffer fields to RtspSrcSettings struct
2. Implement `latency` property with millisecond range validation (0-4294967295)
3. Implement `drop-on-latency` boolean property
4. Implement `probation` property with packet count validation (0-4294967295) 
5. Add property change restrictions (changeable only in NULL or READY state)
6. Update property registration and getter/setter methods
7. Add property documentation matching original rtspsrc format

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and RtspSrcSettings
- Property registration code for jitterbuffer controls

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_jitterbuffer_properties --all-targets --all-features -- --nocapture

# Property Inspection Test
cargo test test_jitterbuffer_properties_inspection --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
latency             : Amount of ms to buffer
                      flags: readable, writable, changeable only in NULL or READY state
                      Unsigned Integer. Range: 0 - 4294967295 Default: 2000

drop-on-latency     : Tells the jitterbuffer to never exceed the given latency in size  
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: false

probation           : Consecutive packet sequence numbers to accept the source
                      flags: readable, writable, changeable only in NULL or READY state  
                      Unsigned Integer. Range: 0 - 4294967295 Default: 2
```

## Property Behavior Details
- **latency**: Controls how much data to buffer before playback starts (reduces network jitter)
- **drop-on-latency**: When true, discard late packets to maintain strict latency bounds
- **probation**: Number of sequential packets required before accepting a new RTP source

## Dependencies
None - pure property infrastructure.

## Success Criteria
- [ ] All three properties visible in gst-inspect output
- [ ] Properties accept valid ranges and reject invalid values
- [ ] Properties changeable only in NULL/READY states
- [ ] Property values stored and retrieved correctly
- [ ] Properties match original rtspsrc behavior and defaults
- [ ] No actual jitterbuffer logic implemented (out of scope)

## Risk Assessment
**LOW RISK** - Property-only changes following established patterns.

## Estimated Effort
2-3 hours

## Confidence Score
9/10 - Straightforward property implementation with well-defined ranges and defaults.