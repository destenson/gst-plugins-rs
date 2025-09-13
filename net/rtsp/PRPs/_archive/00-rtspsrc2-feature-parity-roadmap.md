# PRP-RTSP-00: rtspsrc2 Feature Parity Roadmap

## Overview
This roadmap outlines the comprehensive effort to achieve full feature parity between rtspsrc2 and the original rtspsrc element. The work is divided into 23 atomic PRPs covering all missing properties, signals, actions, and protocol support.

## Current Status
- **rtspsrc (original)**: 47+ properties, 10 signals, 7 actions, 9 URI protocols
- **rtspsrc2 (current)**: 14 properties, 3 signals, 0 actions, 3 URI protocols  
- **Gap**: 33+ missing properties, 7 missing signals, 7 missing actions, 6 missing URI protocols

## Feature Parity PRPs (PRP-30 through PRP-52)

### Phase 1: Authentication & Security (PRP-30 to PRP-32)
1. **PRP-30**: Basic Authentication Properties (`user-id`, `user-pw`)
2. **PRP-31**: Basic Authentication Implementation  
3. **PRP-32**: Digest Authentication Implementation

### Phase 2: Core Properties - Buffering & Control (PRP-33 to PRP-38)
4. **PRP-33**: Jitterbuffer Control Properties (`latency`, `drop-on-latency`, `probation`)
5. **PRP-34**: Buffer Mode Property (`buffer-mode` enum)
6. **PRP-35**: RTCP Control Properties (`do-rtcp`, `do-retransmission`, `max-rtcp-rtp-time-diff`)
7. **PRP-36**: Keep-Alive & Timeout Properties (`do-rtsp-keep-alive`, `tcp-timeout`, `teardown-timeout`, `udp-reconnect`)
8. **PRP-37**: Network Interface Properties (`multicast-iface`, `port-range`, `udp-buffer-size`)
9. **PRP-38**: Source Behavior Properties (`is-live`, `user-agent`, `connection-speed`)

### Phase 3: Advanced Timing & Protocols (PRP-39 to PRP-44)
10. **PRP-39**: Timestamp Synchronization Properties (`ntp-sync`, `rfc7273-sync`, `ntp-time-source`, etc.)
11. **PRP-40**: Transport Protocol Enhancements (missing URI protocols, `default-rtsp-version`)
12. **PRP-41**: RTP-Specific Properties (`rtp-blocksize`, `tcp-timestamp`, `sdes`)  
13. **PRP-42**: TLS/SSL Security Properties (`tls-database`, `tls-interaction`, `tls-validation-flags`)
14. **PRP-43**: Proxy & HTTP Tunneling Properties (`proxy`, `proxy-id`, `proxy-pw`, `extra-http-request-headers`)
15. **PRP-44**: NAT Traversal Properties (`nat-method`, `ignore-x-server-reply`, `force-non-compliant-url`)

### Phase 4: ONVIF & Professional Features (PRP-45)
16. **PRP-45**: ONVIF Backchannel Properties (`backchannel`, `onvif-mode`, `onvif-rate-control`)

### Phase 5: Signals & Application Integration (PRP-46 to PRP-52)
17. **PRP-46**: Core Signals Implementation (`on-sdp`, `select-stream`, `new-manager`)
18. **PRP-47**: Security Signals Implementation (`accept-certificate`, `before-send`, `request-rtcp-key`, `request-rtp-key`)  
19. **PRP-48**: RTSP Action Methods (`get-parameter`, `get-parameters`, `set-parameter`)
20. **PRP-49**: Backchannel Action Methods (`push-backchannel-buffer`, `push-backchannel-sample`, `set-mikey-parameter`, `remove-key`)
21. **PRP-50**: Jitterbuffer Limit Signals (`soft-limit`, `hard-limit`)
22. **PRP-51**: Remaining Compatibility Properties (`short-header`, `debug`, `use-pipeline-clock`, `client-managed-mikey`)
23. **PRP-52**: Server Interaction Signals (`handle-request`)

## Implementation Priority

### Phase 1: Core Foundation (Immediate Priority)
**Execute first in this exact order:**
- **PRP-33**: Jitterbuffer Control Properties (`latency`, `drop-on-latency`, `probation`)
- **PRP-34**: Buffer Mode Property (`buffer-mode` enum)
- **PRP-35**: RTCP Control Properties (`do-rtcp`, `do-retransmission`, `max-rtcp-rtp-time-diff`)
- **PRP-36**: Keep-Alive & Timeout Properties (`do-rtsp-keep-alive`, `tcp-timeout`, `teardown-timeout`, `udp-reconnect`)
- **PRP-37**: Network Interface Properties (`multicast-iface`, `port-range`, `udp-buffer-size`)
- **PRP-38**: Source Behavior Properties (`is-live`, `user-agent`, `connection-speed`)
- **PRP-39**: Timestamp Synchronization Properties (`ntp-sync`, `rfc7273-sync`, `ntp-time-source`, etc.)
- **PRP-41**: RTP-Specific Properties (`rtp-blocksize`, `tcp-timestamp`, `sdes`)  
- **PRP-45**: ONVIF Backchannel Properties (`backchannel`, `onvif-mode`, `onvif-rate-control`)

### Phase 2: Authentication & Security
- Authentication (PRP-30-32): Basic/Digest auth for server access
- Security (PRP-42, PRP-47): TLS/SSL and encryption support

### Phase 3: Protocol Extensions
- Transport Protocols (PRP-40): Missing URI protocol support
- Proxy Support (PRP-43): Corporate network traversal
- NAT Traversal (PRP-44): Network compatibility

### Phase 4: Application Integration
- Core Signals (PRP-46): SDP and stream selection callbacks
- RTSP Actions (PRP-48): GET/SET_PARAMETER server control
- Backchannel Actions (PRP-49): Two-way audio communication

### Phase 5: Specialized Features
- Compatibility (PRP-51): Legacy server support
- Advanced Signals (PRP-50, PRP-52): Buffer monitoring, server requests

## Validation Strategy
Each PRP includes:
- Comprehensive unit tests for property/signal registration
- Integration tests with mock servers where applicable
- Property inspection validation via gst-inspect
- Compatibility verification against original rtspsrc behavior

## Risk Assessment

### Low Risk PRPs (Property-only)
- PRP-30, PRP-33-38, PRP-44-45, PRP-51: Simple property additions
- **Estimated effort**: 2-3 hours each

### Medium Risk PRPs (Protocol/Logic)  
- PRP-31-32, PRP-40-41, PRP-43, PRP-46, PRP-48, PRP-50, PRP-52: Logic implementation
- **Estimated effort**: 3-4 hours each

### High Risk PRPs (Complex Integration)
- PRP-39, PRP-42, PRP-47, PRP-49: Complex types and object management  
- **Estimated effort**: 4-5 hours each

## Success Criteria
Upon completion of all PRPs, rtspsrc2 should:
1. **Properties**: Match all 47+ properties of original rtspsrc
2. **Signals**: Provide all 10 signals for application interaction
3. **Actions**: Support all 7 action methods for server control  
4. **Protocols**: Handle all 9 URI protocol variants
5. **Compatibility**: Work as drop-in replacement for rtspsrc in most applications
6. **Inspection**: Show identical gst-inspect output (excluding implementation details)

## Total Estimated Effort
- **Phase 1 Priority PRPs (9 PRPs)**: 
  - Low Risk (6): PRP-33-38 = 12-18 hours
  - High Risk (3): PRP-39, 41, 45 = 12-15 hours  
  - **Phase 1 Total**: 24-33 hours (3-4 weeks part-time)
- **Remaining PRPs**: 38-52 hours
- **Grand Total**: 62-85 hours (8-11 weeks part-time)

## Phase 1 Quick Start Benefits
Implementing the prioritized PRPs 33-38, 39, 41, 45 first provides:
- **Core buffering control**: Essential for streaming performance
- **Network configuration**: Multi-interface and timeout handling  
- **Timestamp sync**: Professional timing capabilities
- **RTP optimization**: Packet size and timestamping control
- **ONVIF support**: Security camera compatibility
- **~40% feature coverage** with just 9 PRPs

## Dependencies Between PRPs
- PRP-31 depends on PRP-30 (auth properties before auth implementation)
- PRP-32 depends on PRP-31 (digest builds on basic auth)
- All signal PRPs can be implemented independently
- Property PRPs are mostly independent except for auth sequence

## Testing Infrastructure
- Mock RTSP server for protocol testing (from existing PRP work)
- Property validation test framework  
- Signal connection and emission testing
- Action method invocation testing
- Integration tests with real RTSP servers

## Quality Gates
Each PRP must pass:
1. **Compilation**: Clean build with no warnings
2. **Clippy**: All Rust linting checks
3. **Unit Tests**: Property/signal registration and basic functionality
4. **Integration**: gst-inspect output verification
5. **Documentation**: Clear property descriptions matching original

## Confidence Assessment
- **Overall confidence**: 7.5/10 for achieving full feature parity
- **Property implementation**: 9/10 confidence (well-established patterns)
- **Signal/Action implementation**: 7/10 confidence (GStreamer integration complexity)
- **Protocol extensions**: 6/10 confidence (authentication and security protocols)

This roadmap provides a comprehensive path to making rtspsrc2 as capable as the original rtspsrc element through systematic, atomic implementation of all missing features.
