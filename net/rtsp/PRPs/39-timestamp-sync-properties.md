# PRP-RTSP-39: Timestamp Synchronization Properties Implementation

## Overview
Implement timestamp and synchronization properties (`ntp-sync`, `rfc7273-sync`, `ntp-time-source`, `max-ts-offset`, `max-ts-offset-adjustment`, `add-reference-timestamp-meta`) to match original rtspsrc advanced timing capabilities.

## Context
The original rtspsrc provides sophisticated timestamp synchronization through properties for NTP synchronization, RFC 7273 clock references, timestamp offset management, and reference timestamp metadata. These are critical for professional streaming applications requiring precise timing.

## Research Context
- Original rtspsrc timestamp properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- RFC 5905: Network Time Protocol Version 4 (NTP)
- RFC 7273: Real-time Transport Protocol (RTP) Clock Source Signalling
- GStreamer clock synchronization: https://gstreamer.freedesktop.org/documentation/additional/design/synchronisation.html
- Reference timestamp metadata in GStreamer

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `ntp-sync` boolean property (default: false)
2. Add `rfc7273-sync` boolean property (default: false)
3. Add `ntp-time-source` enumeration property (default: "ntp") 
4. Add `max-ts-offset` signed 64-bit property (default: 3000000000 ns)
5. Add `max-ts-offset-adjustment` unsigned 64-bit property (default: 0)
6. Add `add-reference-timestamp-meta` boolean property (default: false)

Does NOT implement:
- Actual NTP synchronization logic
- RFC 7273 clock source signalling
- Timestamp offset correction algorithms
- Reference timestamp metadata generation

## Implementation Tasks
1. Define NtpTimeSource enum: Ntp, Unix, RunningTime, ClockTime
2. Add timestamp synchronization fields to RtspSrcSettings struct  
3. Implement all boolean properties with correct defaults
4. Implement `ntp-time-source` enum property with 4 values
5. Implement `max-ts-offset` with signed 64-bit nanosecond range
6. Implement `max-ts-offset-adjustment` with unsigned 64-bit range
7. Add property change restrictions and documentation

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and enums
- May need separate enum type for NtpTimeSource

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_timestamp_properties --all-targets --all-features -- --nocapture

# Enum Property Test  
cargo test test_ntp_time_source_enum --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
ntp-sync            : Synchronize received streams to the NTP clock
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: false

rfc7273-sync        : Synchronize received streams to the RFC7273 clock (requires clock and offset to be provided)  
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: false

ntp-time-source     : NTP time source for RTCP packets
                      flags: readable, writable, changeable only in NULL or READY state
                      Enum "NtpTimeSource" Default: 0, "ntp"
                         (0): ntp              - NTP time based on realtime clock
                         (1): unix             - UNIX time based on realtime clock
                         (2): running-time     - Running time based on pipeline clock
                         (3): clock-time       - Pipeline clock time

max-ts-offset       : The maximum absolute value of the time offset in (nanoseconds). Note, if the ntp-sync parameter is set the default value is changed to 0 (no limit)
                      flags: readable, writable, changeable only in NULL or READY state
                      Integer64. Range: 0 - 9223372036854775807 Default: 3000000000

max-ts-offset-adjustment: The maximum number of nanoseconds per frame that time stamp offsets may be adjusted (0 = no limit).
                          flags: readable, writable, changeable only in NULL or READY state
                          Unsigned Integer64. Range: 0 - 18446744073709551615 Default: 0

add-reference-timestamp-meta: Add Reference Timestamp Meta to buffers with the original clock timestamp before any adjustments when syncing to an RFC7273 clock.
                              flags: readable, writable, changeable only in NULL or READY state
                              Boolean. Default: false
```

## Property Behavior Details
- **ntp-sync**: Synchronize streams to NTP clock instead of pipeline clock
- **rfc7273-sync**: Use RFC 7273 clock source signalling for synchronization
- **ntp-time-source**: Source of NTP time for RTCP timestamp calculations
- **max-ts-offset**: Maximum allowed timestamp offset in nanoseconds
- **max-ts-offset-adjustment**: Rate limit for timestamp offset corrections per frame
- **add-reference-timestamp-meta**: Add metadata with original timestamps before RFC 7273 adjustments

## NTP Time Source Values
- **ntp**: Standard NTP time based on system realtime clock
- **unix**: UNIX timestamp based on realtime clock  
- **running-time**: Pipeline running time as timestamp source
- **clock-time**: Direct pipeline clock time

## Dependencies
- **Enum type**: NtpTimeSource enumeration for time source selection

## Success Criteria
- [ ] All six properties visible in gst-inspect output
- [ ] Boolean properties work correctly (true/false)
- [ ] NtpTimeSource enum with all 4 values and correct default
- [ ] max-ts-offset accepts signed 64-bit nanosecond range
- [ ] max-ts-offset-adjustment accepts unsigned 64-bit range
- [ ] Properties changeable only in NULL/READY states
- [ ] Property defaults match original rtspsrc exactly
- [ ] No actual synchronization logic implemented (out of scope)

## Risk Assessment
**MEDIUM RISK** - Complex enum property with multiple 64-bit integer properties.

## Estimated Effort
4-5 hours (enum and multiple complex properties)

## Confidence Score
7/10 - Multiple properties with enum types and 64-bit ranges add complexity.