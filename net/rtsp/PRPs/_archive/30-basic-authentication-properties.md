# PRP-RTSP-30: Basic Authentication Properties Implementation

## Overview
Implement basic authentication properties (`user-id`, `user-pw`) to match the original rtspsrc element's authentication capabilities. This is the foundation for RTSP authentication support.

## Context
The original rtspsrc supports basic authentication through `user-id` and `user-pw` properties, while rtspsrc2 currently has no authentication properties. This PRP adds these fundamental authentication properties without implementing the actual authentication logic.

## Research Context
- Original rtspsrc documentation: https://gstreamer.freedesktop.org/documentation/rtsp/rtspsrc.html
- RFC 2617 HTTP Authentication: Basic and Digest Access Authentication
- Reference implementation: `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- Current rtspsrc2 properties in `net/rtsp/src/rtspsrc/imp.rs`

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `user-id` string property for username
2. Add `user-pw` string property for password  
3. Add property storage and accessor methods
4. Add property validation and change notifications

Does NOT implement:
- Actual authentication protocol logic
- Base64 encoding
- Authentication header generation
- Challenge-response handling

## Implementation Tasks
1. Add authentication fields to RtspSrcSettings struct
2. Implement `user-id` property with string type, readable/writable flags
3. Implement `user-pw` property with string type, readable/writable flags
4. Add property change validation (changeable only in NULL or READY state)
5. Update property registration in imp.rs
6. Add getter/setter methods for both properties
7. Ensure properties are included in element inspection output

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and handlers
- Property registration and struct fields

## Validation Gates
```bash
# Syntax/Style  
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_authentication_properties --all-targets --all-features -- --nocapture

# Integration Test - Property Inspection
cargo test test_authentication_properties_inspection --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
user-id             : RTSP location URI user id for authentication
                      flags: readable, writable, changeable only in NULL or READY state
                      String. Default: null

user-pw             : RTSP location URI user password for authentication  
                      flags: readable, writable, changeable only in NULL or READY state
                      String. Default: null
```

## Success Criteria
- [ ] Properties visible in gst-inspect output
- [ ] Properties can be set/get via GStreamer property system
- [ ] Properties reject changes when element is PAUSED or PLAYING
- [ ] Properties store and retrieve values correctly
- [ ] No authentication logic implemented (out of scope)

## Dependencies
None - pure property infrastructure.

## Risk Assessment
**LOW RISK** - Only adds property plumbing, no protocol logic.

## Estimated Effort
2-3 hours

## Confidence Score
9/10 - Straightforward property addition following existing patterns.