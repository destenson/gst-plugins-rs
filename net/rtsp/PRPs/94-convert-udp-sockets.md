# PRP-RTSP-94: Convert UDP Socket Handling from Tokio to GIO

## Overview
Replace Tokio UdpSocket with GIO Socket for RTP/RTCP UDP transport, removing async tasks and using GStreamer's data flow.

## Context
Current implementation spawns async tasks for UDP receive:
- `udp_rtp_task` - receives RTP packets
- `udp_rtcp_task` - receives RTCP packets
These should use GIO sockets with GStreamer's streaming threads.

## Prerequisites
- PRP-93 completed (Tokio runtime removed)
- Understanding of GIO Socket API

## Scope
This PRP ONLY covers:
1. Replace tokio::net::UdpSocket with gio::Socket
2. Remove UDP async tasks
3. Implement GIO-based receive
4. Integrate with buffer queue
5. Handle multicast properly

Does NOT include:
- TCP socket conversion
- RTSP message handling

## Implementation Tasks
1. Update transport.rs:
   - Replace UdpSocket with gio::Socket
   - Remove async from socket creation
   - Use GIO socket options
2. Remove UDP tasks from imp.rs:
   - Delete udp_rtp_task function
   - Delete udp_rtcp_task function
   - Remove task spawning at lines 2613, 2631, 2679, 2696
3. Create GIO receive handlers:
   - Add socket watch sources
   - Handle data in MainContext callbacks
   - Push buffers to appsrc
4. Update multicast handling:
   - Use GIO multicast methods
   - Set proper socket options
5. Fix port binding:
   - Use GIO bind methods
   - Handle port allocation

## Code Locations
- `transport.rs` - All UdpSocket usage
- `imp.rs:2613-2700` - UDP task spawning
- `imp.rs:3800-3900` - udp_rtp_task function
- `imp.rs:3900-4000` - udp_rtcp_task function

## GIO Socket Pattern
Reference the threadshare udpsrc implementation:
- `generic/threadshare/src/udpsrc/imp.rs`
- Uses gio::Socket with GSource

## Validation Gates
```bash
# No tokio UDP references
! grep -q "tokio.*UdpSocket" net/rtsp/src/rtspsrc/

# GIO socket usage
grep -q "gio::Socket" net/rtsp/src/rtspsrc/transport.rs

# UDP tests pass
cargo test -p gst-plugin-rtsp udp

# Multicast works
cargo test -p gst-plugin-rtsp multicast
```

## Expected Behavior
- UDP packets received without async tasks
- Multicast membership works
- Buffer flow to appsrc unchanged
- No packet loss

## Success Criteria
- [ ] All UdpSocket replaced with gio::Socket
- [ ] UDP tasks removed
- [ ] GIO callbacks working
- [ ] Tests pass
- [ ] Multicast functional

## Risk Assessment
**MEDIUM-HIGH RISK** - Critical data path modification.

## Estimated Effort
4 hours

## Confidence Score
6/10 - Complex socket and data flow changes