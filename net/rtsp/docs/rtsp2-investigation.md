# RTSP 2.0 Investigation Report

## Executive Summary

This document presents a comprehensive investigation of RTSP 2.0 (RFC 7826) requirements and provides groundwork for future implementation in the GStreamer RTSP plugin. RTSP 2.0 is a complete rewrite of RTSP 1.0 and is **not backwards compatible**, requiring careful planning for migration.

**Key Finding**: There is no RTSP 1.1 specification. The protocol went directly from 1.0 (RFC 2326, 1998) to 2.0 (RFC 7826, 2016).

## Protocol Versions

| Version | RFC | Year | Status | Compatibility |
|---------|-----|------|--------|---------------|
| RTSP 1.0 | RFC 2326 | 1998 | Active | Baseline |
| RTSP 1.1 | N/A | N/A | **Does not exist** | N/A |
| RTSP 2.0 | RFC 7826 | 2016 | Proposed Standard | Not compatible with 1.0 |

## Major RTSP 2.0 Changes

### 1. Mandatory Features

#### Transport Requirements
- **TCP**: Mandatory implementation (was optional in 1.0)
- **TLS over TCP**: Mandatory for secure connections
- **UDP for RTSP messages**: Removed (only RTP/RTCP over UDP remains)

#### Protocol Features
- **Version negotiation mechanism**: Required for all implementations
- **Feature tags**: New extension mechanism using Require/Proxy-Require headers
- **Request pipelining**: For quick session startup
- **IPv6 support**: Full support required

### 2. Removed Features

#### Methods Removed
- **RECORD**: No longer part of the specification
- **ANNOUNCE**: Removed completely
- Related status codes removed:
  - 201 (Created)
  - 250 (Low On Storage Space)

#### Transport Changes
- **UDP for RTSP messages**: Removed due to lack of interest and broken specification
- **rtspu:// scheme**: Deprecated (unreliable transport)

#### Behavioral Changes
- **PLAY for keep-alive**: No longer allowed in Play state
- **Header extensibility**: Undefined syntax headers no longer permitted

### 3. New Protocol Elements

#### New Headers
| Header | Purpose | Usage |
|--------|---------|-------|
| `Require` | Specify required features | Client → Server |
| `Proxy-Require` | Specify proxy requirements | Client → Proxy |
| `Supported` | List supported features | Server → Client |
| `Proxy-Supported` | List proxy capabilities | Proxy → Client |
| `Media-Properties` | Media stream properties | Server → Client |
| `Scale` | Playback speed control | Client → Server |
| `Speed` | Alternative speed control | Client → Server |
| `Terminate-Reason` | Session termination reason | Server → Client |
| `Pipelined-Requests` | Pipeline request count | Client → Server |

#### New Methods
- **Server → Client TEARDOWN**: Server can now initiate session teardown

#### New Status Codes
- **505**: RTSP Version Not Supported
- **551**: Option Not Supported
- **451**: Parameter Not Understood
- **464**: Data Transport Not Ready
- Additional 4xx/5xx codes for granular error reporting

### 4. Feature Negotiation

RTSP 2.0 introduces a sophisticated feature negotiation mechanism:

```
Client → Server: OPTIONS rtsp://example.com/media RTSP/2.0
                 Require: play.scale, play.speed

Server → Client: RTSP/2.0 200 OK
                 Supported: play.basic, play.scale, setup.rtp.rtcp.mux
```

Feature tags follow the format: `<category>.<feature>`

Common feature tags:
- `play.basic`: Basic playback
- `play.scale`: Scale-based speed control
- `play.speed`: Speed-based control
- `setup.rtp.rtcp.mux`: RTP/RTCP multiplexing

## Breaking Changes Analysis

### 1. Incompatibility Reasons

The IETF explicitly made RTSP 2.0 incompatible with 1.0 due to:

1. **Header Safety**: Most extensible headers in 1.0 lacked defined syntax
2. **State Machine**: Completely reworked for consistency
3. **PLAY Behavior**: Changed semantics when received in Play state
4. **Extension Model**: New mechanism incompatible with 1.0 approach
5. **URI vs URL**: Messages now use URIs rather than URLs

### 2. Migration Challenges

#### Code Level
- Different header parsing requirements
- New state machine implementation
- Feature negotiation layer needed
- Modified transport handling

#### Protocol Level
- Cannot mix 1.0 and 2.0 in same session
- Proxies must understand both versions
- No gradual migration path

#### Deployment Level
- Limited server support (few RTSP 2.0 servers exist)
- Client must support both versions
- Version detection required before communication

## Version Detection Strategy

### Proposed Implementation

```rust
// Pseudocode for version detection
1. Send OPTIONS request with RTSP/1.0
2. If response includes RTSP 2.0 headers (Supported, etc.):
   - Server supports 2.0
   - Retry with RTSP/2.0 if desired
3. If 505 (Version Not Supported):
   - Server only supports different version
   - Adjust accordingly
4. Otherwise:
   - Continue with RTSP 1.0
```

### Detection Module

A `version_detection.rs` module has been created with:

1. **VersionNegotiator**: State machine for version negotiation
2. **Protocol detection**: From server responses
3. **Feature detection**: RTSP 2.0 capability discovery
4. **Error handling**: Version mismatch detection
5. **Request building**: Version-appropriate request construction

## Migration Plan

### Phase 1: Foundation (Current)
✅ Version detection module
✅ Version negotiation logic
✅ Test infrastructure
✅ Documentation

### Phase 2: Parser Extension
- [ ] Extend rtsp-types crate for 2.0 headers
- [ ] Add new status codes
- [ ] Implement feature tag parsing
- [ ] Handle new header syntax

### Phase 3: Core Implementation
- [ ] Dual version state machines
- [ ] Feature negotiation protocol
- [ ] Pipelined request support
- [ ] Server-initiated TEARDOWN

### Phase 4: Advanced Features
- [ ] Scale/Speed controls
- [ ] Media properties handling
- [ ] Full IPv6 support
- [ ] Enhanced error reporting

### Phase 5: Testing & Validation
- [ ] Interoperability testing
- [ ] Compliance validation
- [ ] Performance optimization
- [ ] Production readiness

## Current Implementation Status

### Existing Code Issues

1. **Incorrect Enum**: The `RtspVersion` enum in `imp.rs` includes `V1_1` which doesn't exist
   - Should be removed or marked as reserved/invalid
   
2. **Version Hardcoding**: Currently hardcoded to RTSP 1.0
   - All requests use `Version::V1_0`
   
3. **No Detection**: No version detection mechanism
   - Added via `version_detection.rs` module

### Completed Work

✅ Created `version_detection.rs` module with:
- Version detection from responses
- Version negotiation state machine  
- Feature detection for RTSP 2.0
- Comprehensive test suite
- Error detection for version mismatches

## Server Availability

### Current Status
- **RTSP 2.0 Servers**: Extremely limited availability
- **Test Servers**: No public test servers found
- **Production Servers**: Not widely deployed

### Known Implementations
- Live555: RTSP 1.0 only
- GStreamer rtsp-server: RTSP 1.0 only
- FFmpeg: RTSP 1.0 only
- VLC: RTSP 1.0 only

### Testing Strategy
When RTSP 2.0 servers become available:
1. Use mock servers for unit testing
2. Implement test RTSP 2.0 server
3. Interoperability testing with available implementations

## Recommendations

### Immediate Actions
1. **Fix RtspVersion enum**: Remove non-existent V1_1
2. **Integrate version detection**: Use new module for server detection
3. **Prepare parser**: Extend rtsp-types for 2.0 support

### Short-term (3-6 months)
1. Monitor RTSP 2.0 server availability
2. Implement basic 2.0 parsing
3. Add feature negotiation framework

### Long-term (6-12 months)
1. Full RTSP 2.0 implementation when servers available
2. Maintain dual-version support
3. Performance optimization for pipelining

## Risk Assessment

### Technical Risks
- **Low**: Investigation phase only, no breaking changes
- **Parser complexity**: RTSP 2.0 requires more sophisticated parsing
- **State machine**: Complete rewrite needed for 2.0

### Adoption Risks
- **Server availability**: Very few RTSP 2.0 servers exist
- **Client compatibility**: Must maintain 1.0 for legacy servers
- **Testing challenges**: Limited real-world test targets

### Mitigation Strategies
1. Maintain strict version separation
2. Default to RTSP 1.0 for compatibility
3. Implement comprehensive mock testing
4. Monitor ecosystem for 2.0 adoption

## Conclusion

RTSP 2.0 represents a significant evolution of the protocol with breaking changes that prevent backwards compatibility. While server support remains limited, preparing the groundwork now ensures readiness when adoption increases. The version detection module provides a solid foundation for future dual-version support.

### Key Takeaways
1. RTSP 2.0 is not backwards compatible by design
2. No intermediate versions exist (no RTSP 1.1)
3. Feature negotiation is central to RTSP 2.0
4. Server support remains very limited
5. Dual-version support will be required long-term

## References

- [RFC 2326](https://datatracker.ietf.org/doc/html/rfc2326): RTSP 1.0 Specification
- [RFC 7826](https://datatracker.ietf.org/doc/html/rfc7826): RTSP 2.0 Specification
- [RFC 7826 Appendix I](https://datatracker.ietf.org/doc/html/rfc7826#appendix-I): Complete list of changes
- [rtsp-types crate](https://docs.rs/rtsp-types/): Rust RTSP types library

---
*Generated: 2024-12-14*
*Status: Investigation Complete*
*Next Review: When RTSP 2.0 servers become available*