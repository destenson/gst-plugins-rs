# PRP-RTSP-38: Source Behavior Properties Implementation

## Overview
Implement source behavior control properties (`is-live`, `user-agent`, `connection-speed`) to match original rtspsrc stream behavior and server interaction capabilities.

## Context
The original rtspsrc provides control over element behavior through properties like `is-live` (live source flag), `user-agent` (HTTP header), and `connection-speed` (bandwidth hint). These properties affect stream negotiation, buffering behavior, and server interaction.

## Research Context
- Original rtspsrc behavior properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- GStreamer live source concepts: https://gstreamer.freedesktop.org/documentation/additional/design/live-source.html
- HTTP User-Agent header RFC 7231 specification
- RTSP server bandwidth negotiation and stream selection
- Connection speed hints for adaptive streaming

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `is-live` boolean property (default: true)
2. Add `user-agent` string property (default: "GStreamer/{VERSION}")
3. Add `connection-speed` 64-bit unsigned integer property (default: 0, unknown)
4. Add property validation and state change restrictions

Does NOT implement:
- Live source behavior modification
- User-Agent header injection into requests
- Connection speed-based stream selection
- Version string substitution in User-Agent

## Implementation Tasks
1. Add source behavior fields to RtspSrcSettings struct
2. Implement `is-live` boolean property affecting live source behavior
3. Implement `user-agent` string property with default value
4. Implement `connection-speed` with 64-bit range (0-18446744073709551 kbps)
5. Add property change restrictions (changeable only in NULL or READY state)
6. Document connection speed units (kilobits per second)
7. Add User-Agent string format documentation

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and RtspSrcSettings
- Property registration with correct defaults and ranges

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_behavior_properties --all-targets --all-features -- --nocapture

# User Agent Test
cargo test test_user_agent_property --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
is-live             : Whether to act as a live source
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: true

user-agent          : The User-Agent string to send to the server
                      flags: readable, writable, changeable only in NULL or READY state
                      String. Default: "GStreamer/{VERSION}"

connection-speed    : Network connection speed in kbps (0 = unknown)
                      flags: readable, writable, changeable only in NULL or READY state
                      Unsigned Integer64. Range: 0 - 18446744073709551 Default: 0
```

## Property Behavior Details
- **is-live**: Controls whether element acts as live source (affects buffering, clock, and seeking behavior)
- **user-agent**: HTTP User-Agent header sent to RTSP server (helps server compatibility)
- **connection-speed**: Bandwidth hint in kilobits per second for server stream selection (0 = unknown/unlimited)

## User-Agent Default Value
- Original rtspsrc uses `"GStreamer/{VERSION}"` where {VERSION} is substituted with GStreamer version
- For this property-only PRP, store literal string as placeholder
- Actual version substitution would be implemented in future PRP

## Connection Speed Units
- Units: **kilobits per second (kbps)**
- Range: 0 to 18,446,744,073,709,551 kbps (essentially unlimited)
- 0 = unknown/unspecified speed
- Servers may use this to select appropriate stream bitrates

## Dependencies
None - pure property infrastructure.

## Success Criteria
- [ ] All three properties visible in gst-inspect output
- [ ] is-live boolean property works correctly
- [ ] user-agent accepts string values including default
- [ ] connection-speed accepts full 64-bit unsigned range
- [ ] Properties changeable only in NULL/READY states
- [ ] Property defaults match original rtspsrc
- [ ] No actual behavior modification implemented (out of scope)

## Risk Assessment
**LOW RISK** - Straightforward property implementation.

## Estimated Effort
2-3 hours

## Confidence Score
9/10 - Simple property additions with well-established patterns.