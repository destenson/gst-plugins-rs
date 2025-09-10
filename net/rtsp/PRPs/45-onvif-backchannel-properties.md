# PRP-RTSP-45: ONVIF Backchannel Properties Implementation

## Overview
Implement ONVIF and backchannel properties (`backchannel`, `onvif-mode`, `onvif-rate-control`) to match original rtspsrc security camera integration capabilities and bidirectional communication support.

## Context
The original rtspsrc provides comprehensive ONVIF (Open Network Video Interface Forum) support through properties for backchannel communication, ONVIF client mode, and rate control. These are essential for professional security camera integration and two-way audio communication.

## Research Context
- Original rtspsrc ONVIF properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- ONVIF Profile S specification for streaming
- RTSP backchannel extensions for bidirectional communication
- Audio backchannel over RTSP for security cameras
- Rate control mechanisms in ONVIF implementations

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `backchannel` enumeration property (default: "none")
2. Add `onvif-mode` boolean property (default: false)  
3. Add `onvif-rate-control` boolean property (default: true)
4. Define BackchannelType enum with None and Onvif values

Does NOT implement:
- Actual backchannel media transmission
- ONVIF protocol extensions
- Rate control negotiation
- Bidirectional RTP session management

## Implementation Tasks  
1. Define BackchannelType enum: None, Onvif (matching original values)
2. Add ONVIF fields to RtspSrcSettings struct
3. Implement `backchannel` enum property with string conversion
4. Implement `onvif-mode` boolean property
5. Implement `onvif-rate-control` boolean property
6. Add property change restrictions (changeable only in NULL or READY state)
7. Document ONVIF integration scenarios and backchannel usage

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and BackchannelType enum
- Enum conversion utilities for backchannel types

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_onvif_properties --all-targets --all-features -- --nocapture

# Backchannel Enum Test  
cargo test test_backchannel_enum_conversion --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
backchannel         : The type of backchannel to setup. Default is 'none'.
                      flags: readable, writable, changeable only in NULL or READY state
                      Enum "BackchannelType" Default: 0, "none"
                         (0): none             - No backchannel
                         (1): onvif            - ONVIF audio backchannel

onvif-mode          : Act as an ONVIF client
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: false

onvif-rate-control  : When in onvif-mode, whether to set Rate-Control to yes or no  
                      flags: readable, writable, changeable only in NULL or READY state
                      Boolean. Default: true
```

## Property Behavior Details
- **backchannel**: Type of bidirectional communication channel to establish
- **onvif-mode**: Enable ONVIF-specific client behavior and extensions
- **onvif-rate-control**: Control rate limiting in ONVIF mode (prevents overwhelming cameras)

## Backchannel Type Values
- **none (0)**: No backchannel communication (default, receive-only)
- **onvif (1)**: ONVIF audio backchannel for two-way audio with security cameras

## ONVIF Integration Scenarios
- **Security cameras**: Two-way audio communication with IP cameras
- **Intercom systems**: Bidirectional audio for access control
- **Rate control**: Prevents overwhelming camera with too many requests

## ONVIF Rate Control Behavior
- **true (default)**: Send "Rate-Control: yes" in RTSP requests
- **false**: Send "Rate-Control: no" or omit header
- Helps cameras manage bandwidth and processing load

## Professional Use Cases
- **Surveillance systems**: Monitor and communicate through security cameras
- **Access control**: Two-way communication at security checkpoints  
- **Remote monitoring**: Interactive communication with monitored locations

## Dependencies
- **Enum type**: BackchannelType enumeration matching original values

## Success Criteria
- [ ] All three properties visible in gst-inspect output
- [ ] backchannel enum with correct values and default (none)
- [ ] Boolean properties work correctly for ONVIF settings
- [ ] Enum to string conversion works bidirectionally
- [ ] Properties changeable only in NULL/READY states
- [ ] Property defaults match original rtspsrc exactly
- [ ] No actual ONVIF protocol logic implemented (out of scope)

## Risk Assessment
**LOW RISK** - Enum and boolean properties following established patterns.

## Estimated Effort
2-3 hours

## Confidence Score
8/10 - Straightforward property implementation with well-defined ONVIF standards.