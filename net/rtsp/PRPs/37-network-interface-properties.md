# PRP-RTSP-37: Network Interface Properties Implementation

## Overview
Implement network interface control properties (`multicast-iface`, `port-range`, `udp-buffer-size`) to match original rtspsrc network configuration capabilities. These properties control network interface selection and buffer sizing.

## Context
The original rtspsrc provides detailed network configuration through properties like `multicast-iface` (multicast interface selection), `port-range` (client port restrictions), and `udp-buffer-size` (kernel buffer sizing). These are essential for network optimization and multi-interface systems.

## Research Context
- Original rtspsrc network properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- Multicast interface selection in Linux/Windows networking
- UDP socket buffer sizing for high-bandwidth streaming
- RTP/RTCP port allocation and range restrictions
- Current rtspsrc2 `port-start` property relationship

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `multicast-iface` string property for interface name
2. Add `port-range` string property for port range specification (e.g., "3000-3005")
3. Add `udp-buffer-size` integer property (default: 524288 bytes)
4. Add property validation for ranges and interface names

Does NOT implement:
- Actual multicast interface binding
- Port range enforcement during allocation
- UDP socket buffer size application
- Network interface enumeration or validation

## Implementation Tasks
1. Add network interface fields to RtspSrcSettings struct
2. Implement `multicast-iface` string property (nullable, default: null)
3. Implement `port-range` string property with format validation (nullable, default: null)
4. Implement `udp-buffer-size` with integer range (0-2147483647, default: 524288)
5. Add property change restrictions (changeable only in NULL or READY state)
6. Add port range string format validation (e.g., "3000-3005")
7. Document relationship with existing `port-start` property

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and RtspSrcSettings  
- May need port range validation utility function

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_network_properties --all-targets --all-features -- --nocapture

# Port Range Validation Test
cargo test test_port_range_validation --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
multicast-iface     : The network interface on which to join the multicast group
                      flags: readable, writable, changeable only in NULL or READY state
                      String. Default: null

port-range          : Client port range that can be used to receive RTP and RTCP data, eg. 3000-3005 (NULL = no restrictions)
                      flags: readable, writable, changeable only in NULL or READY state  
                      String. Default: null

udp-buffer-size     : Size of the kernel UDP receive buffer in bytes, 0=default
                      flags: readable, writable, changeable only in NULL or READY state
                      Integer. Range: 0 - 2147483647 Default: 524288
```

## Property Behavior Details
- **multicast-iface**: Network interface name (e.g., "eth0", "wlan0") for multicast group joining
- **port-range**: Port range string format "start-end" (e.g., "3000-3005") restricts client port allocation  
- **udp-buffer-size**: Kernel UDP receive buffer size in bytes (0 = use system default, typically helps with packet loss)

## Port Range Format
Valid formats for `port-range`:
- `null` - No restrictions (system allocates any available ports)
- `"3000-3005"` - Use ports between 3000 and 3005 inclusive
- Single ports not supported (must be range)
- Range must be even number of ports (RTP + RTCP pairs)

## Dependencies
- **Relationship with**: Existing `port-start` property in rtspsrc2
- **Note**: `port-range` provides more specific control than `port-start`

## Success Criteria
- [ ] All three properties visible in gst-inspect output
- [ ] String properties accept null values and valid strings
- [ ] udp-buffer-size accepts full signed 32-bit integer range
- [ ] port-range validates format (start-end) when not null
- [ ] Properties changeable only in NULL/READY states
- [ ] Property defaults match original rtspsrc exactly  
- [ ] No actual network interface logic implemented (out of scope)

## Risk Assessment
**LOW RISK** - Property-only implementation, validation is straightforward.

## Estimated Effort
3-4 hours (including port range validation)

## Confidence Score
8/10 - Mostly straightforward, port range validation adds minor complexity.