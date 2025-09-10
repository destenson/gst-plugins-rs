# RTSP Implementation Status

This document tracks the implementation status of all feature parity PRPs for rtspsrc2 vs original rtspsrc.

## Current Implementation Status (Updated: 2025-01-10)

### Overview
- **rtspsrc (original)**: 51 properties, 10 signals, 7 actions, 9 URI protocols
- **rtspsrc2 (current)**: 28 properties (7 matching + 21 unique), 0 signals, 0 actions, 3 URI protocols  
- **Progress**: 14% properties (7/51 matching), 0% signals, 0% actions, 33% URI protocols
- **Overall Feature Parity**: ~12% complete

### Properties Status (7/51 matching implemented = 14%)

**Original rtspsrc properties (from actual source code):**
- location, protocols, debug, retry, timeout, tcp-timeout, latency
- drop-on-latency, connection-speed, nat-method, do-rtcp, do-rtsp-keep-alive
- proxy, proxy-id, proxy-pw, rtp-blocksize, user-id, user-pw, buffer-mode
- port-range, udp-buffer-size, short-header, probation, udp-reconnect
- multicast-iface, ntp-sync, use-pipeline-clock, sdes, tls-validation-flags
- tls-database, tls-interaction, do-retransmission, ntp-time-source, user-agent
- max-rtcp-rtp-time-diff, rfc7273-sync, add-reference-timestamp-meta
- max-ts-offset-adjustment, max-ts-offset, default-rtsp-version, backchannel
- teardown-timeout, onvif-mode, onvif-rate-control, is-live, ignore-x-server-reply
- extra-http-request-headers, tcp-timestamp, force-non-compliant-url, client-managed-mikey

#### ✅ IMPLEMENTED (7/51 properties = 14%)
**Properties that match original rtspsrc:**
1. ✅ `location` - RTSP server URL and credentials
2. ✅ `protocols` - Allowed transport protocols (udp-mcast,udp,tcp)
3. ✅ `timeout` - Network activity timeout
4. ✅ `user-agent` - HTTP User-Agent string (rtspsrc2 has hardcoded default)
5. ✅ `latency` - Amount of ms to buffer (PRP-33) ⭐ NEW
6. ✅ `drop-on-latency` - Drop buffers when maximum latency is reached (PRP-33) ⭐ NEW
7. ✅ `probation` - Consecutive packet sequence numbers to accept the source (PRP-33) ⭐ NEW

**rtspsrc2-specific properties (21 properties - not in original):**
- `receive-mtu`, `port-start`, `retry-strategy`, `max-reconnection-attempts`
- `reconnection-timeout`, `initial-retry-delay`, `linear-retry-step`
- `connection-racing`, `max-parallel-connections`, `racing-delay-ms`, `racing-timeout`
- `metrics-connection-attempts`, `metrics-connection-successes`, `metrics-packets-received`, `metrics-bytes-received`
- `adaptive-learning`, `adaptive-persistence`, `adaptive-cache-ttl`, `adaptive-discovery-time`
- `adaptive-exploration-rate`, `adaptive-confidence-threshold`, `adaptive-change-detection`

#### 🔲 MISSING - Core Original Properties (47/51 missing = 92%)

**High Priority Phase 1 (PRPs 33-38, 39, 41, 45):**
- `latency` - Jitterbuffer latency configuration (PRP-33)
- `drop-on-latency` - Drop packets on latency issues (PRP-33)
- `probation` - Jitterbuffer probation period (PRP-33)
- `buffer-mode` - Buffer mode enum (PRP-34)
- `do-rtcp` - Enable/disable RTCP (PRP-35)
- `do-retransmission` - Enable retransmission (PRP-35)
- `max-rtcp-rtp-time-diff` - Maximum RTCP/RTP time difference (PRP-35)
- `do-rtsp-keep-alive` - Enable RTSP keep-alive (PRP-36)
- `tcp-timeout` - TCP timeout configuration (PRP-36)
- `teardown-timeout` - TEARDOWN timeout (PRP-36)
- `udp-reconnect` - UDP reconnection behavior (PRP-36)
- `multicast-iface` - Multicast interface selection (PRP-37)
- `port-range` - Port range specification (PRP-37)
- `udp-buffer-size` - UDP buffer size (PRP-37)
- `is-live` - Source is live indicator (PRP-38)
- `user-agent` - HTTP User-Agent string (PRP-38)
- `connection-speed` - Connection speed hint (PRP-38)
- `ntp-sync` - NTP timestamp synchronization (PRP-39)
- `rfc7273-sync` - RFC7273 clock synchronization (PRP-39)
- `ntp-time-source` - NTP time source configuration (PRP-39)
- `rtp-blocksize` - RTP packet block size (PRP-41)
- `tcp-timestamp` - TCP timestamp handling (PRP-41)
- `sdes` - SDES information (PRP-41)
- `backchannel` - ONVIF backchannel support (PRP-45)
- `onvif-mode` - ONVIF mode configuration (PRP-45)
- `onvif-rate-control` - ONVIF rate control (PRP-45)

**Authentication & Security (PRPs 30-32, 42, 47):**
- `user-id` - Basic authentication username (PRP-30)
- `user-pw` - Basic authentication password (PRP-30)
- `tls-database` - TLS certificate database (PRP-42)
- `tls-interaction` - TLS interaction callbacks (PRP-42)
- `tls-validation-flags` - TLS validation flags (PRP-42)

**Protocol Extensions (PRPs 40, 43, 44):**
- `default-rtsp-version` - Default RTSP version (PRP-40)
- `retry` - Max retries when allocating RTP ports
- `debug` - Debug output control (deprecated)
- `proxy` - HTTP proxy configuration (PRP-43)
- `proxy-id` - Proxy username (PRP-43)
- `proxy-pw` - Proxy password (PRP-43)
- `extra-http-request-headers` - Additional HTTP headers (PRP-43)
- `nat-method` - NAT traversal method (PRP-44)
- `ignore-x-server-reply` - Ignore X-Server replies (PRP-44)
- `force-non-compliant-url` - Force non-compliant URL handling (PRP-44)

**Additional Missing Core Properties:**
- `tcp-timeout` - Fail after timeout on TCP connections
- `add-reference-timestamp-meta` - Add reference timestamp meta
- `max-ts-offset-adjustment` - Max timestamp offset adjustment
- `max-ts-offset` - Maximum timestamp offset

**Compatibility & Misc (PRP-51):**
- `short-header` - Use short RTSP headers (PRP-51)
- `debug` - Debug output control (PRP-51)
- `use-pipeline-clock` - Use pipeline clock (PRP-51)
- `client-managed-mikey` - Client-managed MIKEY (PRP-51)

### Signals Status (0/10 implemented = 0%)

#### 🔲 MISSING - Core Signals (PRP-46)
- `on-sdp` - SDP message received
- `select-stream` - Stream selection callback
- `new-manager` - New RTP manager created

#### 🔲 MISSING - Security Signals (PRP-47)
- `accept-certificate` - TLS certificate validation
- `before-send` - Pre-send message modification
- `request-rtcp-key` - RTCP encryption key request
- `request-rtp-key` - RTP encryption key request

#### 🔲 MISSING - Buffer Monitoring (PRP-50)
- `soft-limit` - Jitterbuffer soft limit reached
- `hard-limit` - Jitterbuffer hard limit reached

#### 🔲 MISSING - Server Interaction (PRP-52)
- `handle-request` - Handle server requests

### Actions Status (0/7 implemented = 0%)

#### 🔲 MISSING - RTSP Actions (PRP-48)
- `get-parameter` - GET_PARAMETER method
- `get-parameters` - GET_PARAMETERS method (batch)
- `set-parameter` - SET_PARAMETER method

#### 🔲 MISSING - Backchannel Actions (PRP-49)
- `push-backchannel-buffer` - Push buffer to backchannel
- `push-backchannel-sample` - Push sample to backchannel
- `set-mikey-parameter` - Set MIKEY parameter
- `remove-key` - Remove encryption key

### URI Protocols Status (3/9 implemented = 33%)

#### ✅ IMPLEMENTED
- `rtsp://` - Standard RTSP
- `rtsps://` - RTSP over TLS (basic support)
- `rtsp-unix://` - RTSP over Unix socket (if supported)

#### 🔲 MISSING - Protocol Extensions (PRP-40)
- `rtspt://` - RTSP over HTTP tunnel
- `rtspu://` - RTSP over UDP
- `rtspv://` - RTSP with VOD support
- `rtsph://` - RTSP over HTTP
- `rtsphs://` - RTSP over HTTPS
- `rtsp-tcp://` - RTSP forcing TCP transport

## Phase 1 Implementation Priority

**NEXT: Start with PRP-33 (Jitterbuffer Control Properties)**

### Phase 1 PRPs (Immediate Priority - 9 PRPs)
1. ⭐ **PRP-33**: Jitterbuffer Control Properties - Essential for streaming performance
2. ⭐ **PRP-34**: Buffer Mode Property - Buffer management control
3. ⭐ **PRP-35**: RTCP Control Properties - Protocol reliability
4. ⭐ **PRP-36**: Keep-Alive & Timeout Properties - Connection stability
5. ⭐ **PRP-37**: Network Interface Properties - Multi-interface support
6. ⭐ **PRP-38**: Source Behavior Properties - Source characteristics
7. ⭐ **PRP-39**: Timestamp Synchronization Properties - Professional timing
8. ⭐ **PRP-41**: RTP-Specific Properties - Packet optimization
9. ⭐ **PRP-45**: ONVIF Backchannel Properties - Security camera support

### Estimated Impact
- **After Phase 1**: ~60% property coverage, ~45% overall feature parity
- **Effort**: 24-33 hours (3-4 weeks part-time)
- **Key Benefits**: Production-ready streaming performance and reliability

## PRP Implementation Status

| PRP | Phase | Status | Properties | Signals | Actions | Effort Est. |
|-----|-------|--------|------------|---------|---------|-------------|
| PRP-30 | 1-Auth | 📋 Planned | user-id, user-pw | - | - | 2-3h |
| PRP-31 | 1-Auth | 📋 Planned | - | - | - | 3-4h |
| PRP-32 | 1-Auth | 📋 Planned | - | - | - | 3-4h |
| **PRP-33** | **1-Core** | **✅ COMPLETE** | **latency, drop-on-latency, probation** | - | - | **2-3h** |
| **PRP-34** | **1-Core** | **⭐ NEXT** | **buffer-mode** | - | - | **2-3h** |
| PRP-35 | 1-Core | 📋 Planned | do-rtcp, do-retransmission, max-rtcp-rtp-time-diff | - | - | 2-3h |
| PRP-36 | 1-Core | 📋 Planned | do-rtsp-keep-alive, tcp-timeout, teardown-timeout, udp-reconnect | - | - | 2-3h |
| PRP-37 | 1-Core | 📋 Planned | multicast-iface, port-range, udp-buffer-size | - | - | 2-3h |
| PRP-38 | 1-Core | 📋 Planned | is-live, user-agent, connection-speed | - | - | 2-3h |
| PRP-39 | 1-Core | 📋 Planned | ntp-sync, rfc7273-sync, ntp-time-source | - | - | 4-5h |
| PRP-40 | 2-Proto | 📋 Planned | default-rtsp-version | - | - | 3-4h |
| PRP-41 | 1-Core | 📋 Planned | rtp-blocksize, tcp-timestamp, sdes | - | - | 4-5h |
| PRP-42 | 2-Sec | 📋 Planned | tls-database, tls-interaction, tls-validation-flags | - | - | 4-5h |
| PRP-43 | 2-Proto | 📋 Planned | proxy, proxy-id, proxy-pw, extra-http-request-headers | - | - | 3-4h |
| PRP-44 | 2-Proto | 📋 Planned | nat-method, ignore-x-server-reply, force-non-compliant-url | - | - | 2-3h |
| PRP-45 | 1-Core | 📋 Planned | backchannel, onvif-mode, onvif-rate-control | - | - | 4-5h |
| PRP-46 | 3-App | 📋 Planned | - | on-sdp, select-stream, new-manager | - | 3-4h |
| PRP-47 | 2-Sec | 📋 Planned | - | accept-certificate, before-send, request-rtcp-key, request-rtp-key | - | 4-5h |
| PRP-48 | 3-App | 📋 Planned | - | - | get-parameter, get-parameters, set-parameter | 3-4h |
| PRP-49 | 3-App | 📋 Planned | - | - | push-backchannel-buffer, push-backchannel-sample, set-mikey-parameter, remove-key | 4-5h |
| PRP-50 | 4-Spec | 📋 Planned | - | soft-limit, hard-limit | - | 3-4h |
| PRP-51 | 4-Spec | 📋 Planned | short-header, debug, use-pipeline-clock, client-managed-mikey | - | - | 2-3h |
| PRP-52 | 4-Spec | 📋 Planned | - | handle-request | - | 3-4h |

**Legend:**
- ⭐ NEXT - Ready for immediate implementation
- 📋 Planned - PRP file exists, ready for implementation
- ✅ Complete - Fully implemented and tested
- 🔄 In Progress - Currently being implemented
- ❌ Blocked - Dependencies not met

## Quality Gates

Each PRP must pass all quality gates before being marked complete:

1. ✅ **Compilation** - Clean build with no warnings
2. ✅ **Clippy** - All Rust linting checks pass
3. ✅ **Unit Tests** - Property/signal registration and functionality tests
4. ✅ **Integration** - gst-inspect output verification  
5. ✅ **Documentation** - Clear property descriptions matching original
6. ✅ **Validation** - Comparison testing against original rtspsrc behavior

---

**Last Updated**: 2025-01-10  
**Next Action**: Execute PRP-34 (Buffer Mode Property)  
**Phase 1 Progress**: 1/9 PRPs complete (11%)  
**Overall Progress**: 7/51 properties (14%), 0/10 signals (0%), 0/7 actions (0%)  
**Recent Progress**: ✅ **PRP-33 COMPLETED** - Added 3 jitterbuffer control properties  
**Critical Finding**: rtspsrc2 now has 7 properties that match the original rtspsrc (14% coverage). Most current properties are still rtspsrc2-specific enhancements.