# PRP-RTSP-36: Keep-Alive and Timeout Properties Implementation

## Overview
Implement keep-alive and timeout control properties (`do-rtsp-keep-alive`, `tcp-timeout`, `teardown-timeout`, `udp-reconnect`) to match original rtspsrc connection management capabilities.

## Context
The original rtspsrc provides comprehensive connection lifecycle management through properties like `do-rtsp-keep-alive` (prevent timeouts), `tcp-timeout` (TCP connection timeout), `teardown-timeout` (graceful shutdown delay), and `udp-reconnect` (UDP reconnection behavior). These are critical for robust RTSP connection handling.

## Research Context
- Original rtspsrc timeout properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- RTSP RFC 2326 keep-alive and session timeout requirements
- TCP socket timeout handling in network programming
- UDP vs TCP reconnection behavior differences in RTSP
- Current timeout handling in rtspsrc2's `timeout` property

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `do-rtsp-keep-alive` boolean property (default: true)
2. Add `tcp-timeout` microsecond property (default: 20000000, 20 seconds)
3. Add `teardown-timeout` nanosecond property (default: 100000000, 100ms)  
4. Add `udp-reconnect` boolean property (default: true)
5. Add proper range validation and state change restrictions

Does NOT implement:
- Actual keep-alive packet sending
- TCP timeout enforcement 
- Teardown delay logic
- UDP reconnection mechanisms

## Implementation Tasks
1. Add timeout and keep-alive fields to RtspSrcSettings struct
2. Implement `do-rtsp-keep-alive` boolean property
3. Implement `tcp-timeout` with microsecond range (0-18446744073709551615)
4. Implement `teardown-timeout` with nanosecond range (0-18446744073709551615)
5. Implement `udp-reconnect` boolean property
6. Add property change restrictions (changeable only in NULL or READY state)
7. Document timeout units clearly (microseconds vs nanoseconds)

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and RtspSrcSettings
- Property registration with correct units and ranges

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_timeout_properties --all-targets --all-features -- --nocapture

# Range Validation Test
cargo test test_timeout_property_ranges --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
do-rtsp-keep-alive  : Send RTSP keep alive packets, disable for old incompatible server.
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: true

tcp-timeout         : Fail after timeout microseconds on TCP connections (0 = disabled)
                      flags: readable, writable, changeable only in NULL or READY state
                      Unsigned Integer64. Range: 0 - 18446744073709551615 Default: 20000000

teardown-timeout    : When transitioning PAUSED-READY, allow up to timeout (in nanoseconds) delay in order to send teardown (0 = disabled)
                      flags: readable, writable, changeable only in NULL or READY state  
                      Unsigned Integer64. Range: 0 - 18446744073709551615 Default: 100000000

udp-reconnect       : Reconnect to the server if RTSP connection is closed when doing UDP
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: true
```

## Property Behavior Details
- **do-rtsp-keep-alive**: Periodically sends RTSP requests to prevent server timeout (some old servers don't support)
- **tcp-timeout**: Maximum time to wait for TCP connection establishment (0 = no timeout)
- **teardown-timeout**: Grace period for sending RTSP TEARDOWN when transitioning to READY state
- **udp-reconnect**: Whether to reconnect when RTSP control connection closes during UDP streaming

## Time Unit Clarification
- **tcp-timeout**: Microseconds (μs) - matches original rtspsrc  
- **teardown-timeout**: Nanoseconds (ns) - matches original rtspsrc
- This difference is intentional and matches the original implementation

## Dependencies
None - pure property infrastructure.

## Success Criteria
- [ ] All four properties visible in gst-inspect output
- [ ] Boolean properties work correctly (true/false)
- [ ] Timeout properties accept full 64-bit unsigned integer ranges  
- [ ] Properties changeable only in NULL/READY states
- [ ] Property defaults match original rtspsrc exactly
- [ ] Time units clearly documented and correct
- [ ] No actual timeout/keep-alive logic implemented (out of scope)

## Risk Assessment
**LOW RISK** - Property-only implementation with established patterns.

## Estimated Effort
2-3 hours

## Confidence Score
9/10 - Straightforward property additions with well-defined defaults and ranges.