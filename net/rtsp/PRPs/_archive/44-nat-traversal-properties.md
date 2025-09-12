# PRP-RTSP-44: NAT Traversal Properties Implementation

## Overview
Implement NAT traversal properties (`nat-method`, `ignore-x-server-reply`, `force-non-compliant-url`) to match original rtspsrc network address translation and compatibility capabilities.

## Context
The original rtspsrc provides NAT traversal support through properties like `nat-method` (dummy packet sending), `ignore-x-server-reply` (server IP header handling), and `force-non-compliant-url` (legacy URL construction). These are critical for streaming through NAT devices and handling server compatibility issues.

## Research Context
- Original rtspsrc NAT properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- NAT hole-punching techniques for RTP/RTSP
- RTSP X-Server-IP-Address header handling
- Legacy RTSP URL construction methods
- Dummy packet techniques for keeping NAT mappings alive

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `nat-method` enumeration property (default: "dummy") 
2. Add `ignore-x-server-reply` boolean property (default: false)
3. Add `force-non-compliant-url` boolean property (default: false)
4. Define NatMethod enum with None and Dummy values

Does NOT implement:
- Actual dummy packet transmission
- NAT hole-punching logic
- X-Server-IP-Address header processing  
- Legacy URL construction algorithms

## Implementation Tasks
1. Define NatMethod enum: None, Dummy (matching original values)
2. Add NAT traversal fields to RtspSrcSettings struct
3. Implement `nat-method` enum property with string conversion
4. Implement `ignore-x-server-reply` boolean property
5. Implement `force-non-compliant-url` boolean property  
6. Add property change restrictions (changeable only in NULL or READY state)
7. Document NAT method behaviors and compatibility scenarios

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and NatMethod enum
- Enum to string conversion utilities

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_nat_properties --all-targets --all-features -- --nocapture

# NAT Method Enum Test
cargo test test_nat_method_enum_conversion --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
nat-method          : Method to use for traversing firewalls and NAT
                      flags: readable, writable, changeable only in NULL or READY state
                      Enum "NatMethod" Default: 1, "dummy"
                         (0): none             - None
                         (1): dummy            - Send Dummy packets

ignore-x-server-reply: Whether to ignore the x-server-ip-address server header reply
                       flags: readable, writable, changeable only in NULL or READY state
                       Boolean. Default: false

force-non-compliant-url: Revert to old non-compliant method of constructing URLs
                         flags: readable, writable, changeable only in NULL or READY state
                         Boolean. Default: false
```

## Property Behavior Details
- **nat-method**: Strategy for maintaining NAT mappings during streaming
- **ignore-x-server-reply**: Ignore server-provided IP addresses in RTSP responses
- **force-non-compliant-url**: Use legacy URL construction for old/broken servers

## NAT Method Values
- **none (0)**: No special NAT handling
- **dummy (1)**: Send dummy packets to keep NAT holes open (default)

## NAT Traversal Scenarios
- **Dummy packets**: Periodic transmission to maintain NAT port mappings
- **X-Server-IP ignore**: When server reports incorrect IP addresses behind NAT
- **Non-compliant URLs**: Compatibility with servers that expect old URL formats

## Compatibility Use Cases
- **ignore-x-server-reply**: For servers behind NAT that report internal IP addresses
- **force-non-compliant-url**: For legacy servers with incorrect URL parsing
- Both properties help with older or misconfigured RTSP servers

## Dependencies  
- **Enum type**: NatMethod enumeration with correct numeric values

## Success Criteria
- [ ] All three properties visible in gst-inspect output
- [ ] nat-method enum with correct values and default
- [ ] Boolean properties work correctly (true/false)
- [ ] Enum to string conversion works both ways
- [ ] Properties changeable only in NULL/READY states
- [ ] Property defaults match original rtspsrc exactly
- [ ] No actual NAT traversal logic implemented (out of scope)

## Risk Assessment
**LOW-MEDIUM RISK** - Enum property and boolean properties, straightforward implementation.

## Estimated Effort
2-3 hours

## Confidence Score
8/10 - Simple enum and boolean properties with well-defined behavior.