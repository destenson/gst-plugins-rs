# RTSP 2.0 Migration Plan

## Overview

This document outlines the migration strategy from RTSP 1.0 to RTSP 2.0 support in the GStreamer RTSP plugin. Due to the fundamental incompatibility between versions, this plan focuses on dual-version support rather than migration.

## Migration Principles

1. **Backwards Compatibility First**: RTSP 1.0 remains the default
2. **Opt-in 2.0 Support**: Users explicitly enable RTSP 2.0
3. **Graceful Degradation**: Fall back to 1.0 when 2.0 unavailable
4. **Clear Version Boundaries**: Separate code paths for each version
5. **No Mixed Sessions**: One version per RTSP session

## Architecture Design

### Version Abstraction Layer

```
┌─────────────────────────────────────┐
│         Application Layer            │
├─────────────────────────────────────┤
│      Version Negotiator              │
├──────────────┬──────────────────────┤
│  RTSP 1.0    │    RTSP 2.0          │
│  Handler     │    Handler           │
├──────────────┴──────────────────────┤
│       Common Transport Layer         │
└─────────────────────────────────────┘
```

### Module Structure

```
rtspsrc/
├── version_detection.rs    # Version negotiation (✅ Created)
├── rtsp_v1/                # RTSP 1.0 specific
│   ├── parser.rs
│   ├── state_machine.rs
│   └── handlers.rs
├── rtsp_v2/                # RTSP 2.0 specific
│   ├── parser.rs
│   ├── state_machine.rs
│   ├── features.rs
│   └── handlers.rs
└── common/                 # Shared components
    ├── transport.rs
    ├── auth.rs
    └── media.rs
```

## Implementation Phases

### Phase 1: Foundation (Week 1-2) ✅ COMPLETED

**Status**: ✅ Complete

- [x] Research RFC 7826 thoroughly
- [x] Document all differences
- [x] Create version detection module
- [x] Add version negotiation logic
- [x] Write comprehensive tests

**Deliverables**:
- `version_detection.rs` module
- RTSP 2.0 investigation report
- Migration plan document

### Phase 2: Parser Preparation (Week 3-4)

**Goal**: Extend parsing capabilities for RTSP 2.0

**Tasks**:
1. Extend rtsp-types crate
   ```rust
   // New headers to add
   pub const REQUIRE: HeaderName = HeaderName("Require");
   pub const PROXY_REQUIRE: HeaderName = HeaderName("Proxy-Require");
   pub const SUPPORTED: HeaderName = HeaderName("Supported");
   pub const MEDIA_PROPERTIES: HeaderName = HeaderName("Media-Properties");
   ```

2. Add new status codes
   ```rust
   pub const VERSION_NOT_SUPPORTED: StatusCode = StatusCode(505);
   pub const OPTION_NOT_SUPPORTED: StatusCode = StatusCode(551);
   ```

3. Implement feature tag parser
   ```rust
   pub struct FeatureTag {
       category: String,
       feature: String,
   }
   ```

4. Update message builder for 2.0

**Validation**:
- Unit tests for all new headers
- Parser compatibility tests
- Round-trip serialization tests

### Phase 3: Dual Version Support (Week 5-8)

**Goal**: Implement parallel version handlers

**Tasks**:

1. **Version Router**
   ```rust
   pub trait VersionHandler {
       fn handle_options(&self, req: Request) -> Response;
       fn handle_describe(&self, req: Request) -> Response;
       fn handle_setup(&self, req: Request) -> Response;
       fn handle_play(&self, req: Request) -> Response;
       fn handle_teardown(&self, req: Request) -> Response;
   }
   ```

2. **RTSP 1.0 Handler**
   - Extract current implementation
   - Wrap in version-specific module
   - Maintain existing behavior

3. **RTSP 2.0 Handler Stub**
   - Implement basic 2.0 responses
   - Feature negotiation
   - New state machine

4. **Session Manager Updates**
   - Track version per session
   - Prevent version mixing
   - Handle server-initiated teardown

**Validation**:
- Parallel version tests
- Session isolation tests
- Version switching tests

### Phase 4: RTSP 2.0 Core Features (Week 9-12)

**Goal**: Implement RTSP 2.0 specific features

**Tasks**:

1. **Feature Negotiation Protocol**
   ```rust
   impl RtspV2Handler {
       fn negotiate_features(&mut self, require: Vec<FeatureTag>) 
           -> Result<Vec<FeatureTag>, Error> {
           // Match required vs supported features
       }
   }
   ```

2. **Request Pipelining**
   - Queue multiple requests
   - Maintain order guarantees
   - Handle pipeline failures

3. **Media Properties**
   - Parse media property headers
   - Apply to stream configuration
   - Update SDP handling

4. **Scale/Speed Support**
   - Implement playback rate control
   - Handle scale vs speed semantics
   - Update timing calculations

**Validation**:
- Feature negotiation tests
- Pipeline stress tests
- Playback control tests

### Phase 5: Advanced Features (Week 13-16)

**Goal**: Complete RTSP 2.0 implementation

**Tasks**:

1. **IPv6 Support**
   - Full IPv6 address handling
   - Dual-stack support
   - IPv6 literal in URIs

2. **Enhanced Security**
   - Mandatory TLS support
   - Improved authentication
   - Certificate validation

3. **Server-Initiated Actions**
   - TEARDOWN from server
   - Terminate-Reason handling
   - Graceful disconnection

4. **Error Handling**
   - New status code handling
   - Detailed error reporting
   - Recovery strategies

**Validation**:
- IPv6 connectivity tests
- Security compliance tests
- Server action tests

## Migration Strategy for Users

### Configuration

```rust
// Property to control RTSP version preference
#[property(
    name = "rtsp-version",
    nick = "RTSP Version",
    blurb = "Preferred RTSP version (auto, 1.0, 2.0)",
    default = "auto",
)]
rtsp_version: RtspVersionPreference,
```

### Version Selection Logic

```rust
enum RtspVersionPreference {
    Auto,    // Detect and use best available
    V1_0,    // Force RTSP 1.0
    V2_0,    // Force RTSP 2.0 (fail if unsupported)
    Prefer2_0, // Prefer 2.0, fallback to 1.0
}
```

### User Migration Path

1. **Stage 1**: Default remains RTSP 1.0
   - No user action required
   - Existing pipelines work unchanged

2. **Stage 2**: Opt-in RTSP 2.0 testing
   ```bash
   gst-launch-1.0 rtspsrc2 location=rtsp://server/media \
                  rtsp-version=prefer2.0 ! ...
   ```

3. **Stage 3**: Auto-detection mature
   ```bash
   gst-launch-1.0 rtspsrc2 location=rtsp://server/media \
                  rtsp-version=auto ! ...
   ```

4. **Stage 4**: RTSP 2.0 by default (future)
   - When server adoption reaches critical mass
   - Maintain 1.0 fallback indefinitely

## Testing Strategy

### Unit Testing

Each module requires comprehensive tests:

```rust
#[cfg(test)]
mod tests {
    // Version detection tests
    #[test]
    fn test_version_detect_v1_0() { }
    
    #[test]
    fn test_version_detect_v2_0() { }
    
    #[test]
    fn test_version_negotiate_fallback() { }
    
    // Feature negotiation tests
    #[test]
    fn test_feature_negotiation_success() { }
    
    #[test]
    fn test_feature_negotiation_partial() { }
}
```

### Integration Testing

1. **Mock Servers**
   - RTSP 1.0 mock server
   - RTSP 2.0 mock server
   - Version-switching server

2. **Interoperability Matrix**
   | Client Version | Server Version | Expected Result |
   |---------------|---------------|-----------------|
   | 1.0 | 1.0 | Success |
   | 1.0 | 2.0 | Depends on server |
   | 2.0 | 1.0 | Fallback to 1.0 |
   | 2.0 | 2.0 | Success |

3. **Stress Testing**
   - Rapid version switching
   - Pipeline request flooding
   - Concurrent mixed-version sessions

### Compliance Testing

- RFC 7826 compliance suite
- Feature coverage validation
- Protocol conformance tests

## Rollback Plan

If RTSP 2.0 causes issues:

1. **Immediate**: Set default to force 1.0
2. **Short-term**: Feature-gate 2.0 code
3. **Long-term**: Maintain separate branches

## Success Metrics

### Phase Metrics

| Phase | Success Criteria | Measurement |
|-------|-----------------|-------------|
| 1 | Documentation complete | 100% tasks done |
| 2 | Parser extended | All 2.0 headers parseable |
| 3 | Dual version working | Both versions connect |
| 4 | Core features done | Feature negotiation works |
| 5 | Full implementation | RFC compliance >90% |

### Overall Metrics

1. **Compatibility**: No regression in RTSP 1.0 support
2. **Performance**: <5% overhead for version detection
3. **Adoption**: Successfully connect to all 2.0 test servers
4. **Stability**: Zero version-related crashes

## Risk Mitigation

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Parser incompatibility | Medium | High | Separate parsers per version |
| State machine conflicts | High | High | Independent state machines |
| Performance degradation | Low | Medium | Optimize hot paths |
| Memory overhead | Medium | Low | Share common structures |

### Adoption Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| No 2.0 servers | High | Medium | Focus on 1.0 reliability |
| Slow adoption | High | Low | Long-term maintenance plan |
| Breaking changes | Low | High | Extensive testing |

## Timeline Summary

```
Week 1-2:   ✅ Foundation (COMPLETED)
Week 3-4:   Parser Preparation
Week 5-8:   Dual Version Support
Week 9-12:  RTSP 2.0 Core Features
Week 13-16: Advanced Features
Week 17-18: Integration Testing
Week 19-20: Documentation & Polish
```

## Next Steps

1. ✅ Complete version detection module
2. ✅ Document investigation findings
3. ✅ Create migration plan
4. ⏱️ Review with team
5. ⏱️ Begin parser extension
6. ⏱️ Monitor RTSP 2.0 server availability

## Conclusion

This migration plan provides a structured approach to adding RTSP 2.0 support while maintaining full backwards compatibility. The phased implementation allows for incremental progress and reduces risk. The dual-version architecture ensures long-term maintainability as the ecosystem transitions from RTSP 1.0 to 2.0.

---
*Document Version: 1.0*
*Created: 2024-12-14*
*Next Review: After Phase 2 completion*