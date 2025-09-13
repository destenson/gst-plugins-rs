# PRP: H.264 Parameter Set Management System

## Problem Statement

H.264 RTP payloading requires sophisticated parameter set (SPS/PPS) management according to RFC 6184. Parameter sets contain critical decoding information and must be cached, validated, and retransmitted appropriately. This system is essential for reliable H.264 video streaming and decoder compatibility.

## Context & Requirements

### RFC 6184 Parameter Set Requirements
Parameter sets in H.264 RTP streams must be handled according to specific rules:
- **Caching**: Store SPS/PPS for retransmission and validation
- **Insertion**: Include parameter sets in RTP streams at appropriate intervals
- **Validation**: Verify parameter set consistency across frames
- **Bandwidth Optimization**: Avoid redundant parameter set transmission

### Reference Implementation Analysis
Study the original C implementation in `gst-plugins-good/gst/rtp/gstrtph264pay.c` around lines 400-600 for parameter set handling patterns. The C implementation maintains parameter set state and inserts them strategically.

**Existing Patterns to Follow:**
- Look at `net/rtp/src/opus/pay/imp.rs` for configuration caching patterns
- Reference `net/rtp/src/ac3/pay/imp.rs` for stream state management
- Study `net/rtp/src/basepay/mod.rs` for caps and configuration handling

## Implementation Plan

### Target Architecture  
Create `net/rtp/src/h264/common/parameter_sets.rs` with caching and management functionality.

### Core Components to Implement

1. **Parameter Set Storage**
   - Cache SPS and PPS NAL units with unique identifiers
   - Track parameter set versions and changes
   - Implement efficient lookup by parameter set ID
   - Handle multiple parameter sets per stream

2. **Parameter Set Validation**
   - Validate SPS/PPS structure according to H.264 specification
   - Check parameter set dependencies and consistency
   - Detect changes requiring decoder reinitialization
   - Validate profile and level compatibility

3. **Insertion Strategy**
   - Determine when parameter sets need insertion into RTP stream
   - Implement keyframe-based insertion logic
   - Handle periodic refresh for long-running streams
   - Support out-of-band parameter set delivery

4. **Configuration Integration**
   - Extract parameter sets from input caps or buffers
   - Generate RTP caps with parameter set information
   - Handle codec_data format variations
   - Support dynamic parameter set changes

5. **Memory Management**
   - Efficient storage of parameter set data
   - Reference counting for shared parameter sets
   - Cleanup of outdated parameter sets
   - Memory bounds validation

### Integration Requirements
- Integrate with NAL unit parsing infrastructure from previous PRP
- Provide clean API for payloader configuration
- Support both in-band and out-of-band parameter set delivery
- Handle caps negotiation and parameter set advertisement

## Validation

### Functional Testing
```bash
# Syntax/Style validation  
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit tests for parameter set management
cargo test h264::common::parameter_sets --all-features -- --nocapture

# Integration with H.264 payloader infrastructure
cargo test h264::pay --all-features -- --nocapture
```

### Test Data Requirements
- Valid SPS/PPS pairs from various H.264 profiles
- Invalid parameter sets for error handling
- Parameter set sequences with dependencies
- Codec data in both AVCC and Annex B formats
- Streams with parameter set changes

### Behavioral Validation
- Verify parameter sets extracted correctly from caps
- Confirm appropriate insertion timing
- Validate caching and retrieval functionality
- Test memory cleanup under various scenarios

## Dependencies

### External Documentation
- **RFC 6184 Section 8.2**: Parameter set handling
- **ITU-T H.264 Section 7.3.2**: Parameter set syntax
- **GStreamer H.264 caps documentation**: Parameter set encoding in caps

### Codebase References
- Study parameter caching patterns in `opus/pay/imp.rs`
- Reference caps handling in existing payloaders
- Follow error handling patterns from `basepay` implementations

### Prerequisites
- Depends on H.264 NAL unit parsing infrastructure PRP
- Requires understanding of GStreamer caps parameter encoding
- Needs access to ITU-T H.264 specification for validation rules

## Success Criteria

1. **Functionality**
   - Extract parameter sets from various input formats
   - Cache and manage multiple parameter set versions
   - Insert parameter sets at appropriate stream positions
   - Handle parameter set changes during streaming

2. **Performance**
   - Efficient parameter set lookup and storage
   - Minimal overhead for parameter set management
   - Fast parameter set validation and processing

3. **Robustness** 
   - Handle malformed parameter sets gracefully
   - Recover from parameter set inconsistencies
   - Memory-safe parameter set storage and cleanup

4. **Compatibility**
   - Support standard GStreamer H.264 caps formats
   - Compatible with common encoder parameter set output
   - Interoperable with existing H.264 decoders

## Risk Assessment

**Medium Risk** - Complexity in parameter set dependency handling and timing, but well-specified in RFC 6184.

## Estimated Effort

**3-4 hours** - Parameter set management with caching, validation, and insertion logic.

## Implementation Notes

### Critical Design Considerations
- Parameter set IDs must be tracked carefully for consistency
- Memory usage should be bounded for long-running streams  
- Threading considerations for parameter set access
- Error recovery when parameter sets become inconsistent

### Key Integration Points
- Must integrate cleanly with NAL unit parsing
- Should provide simple API for payloader use
- Needs to handle GStreamer caps parameter encoding
- Must support dynamic reconfiguration scenarios

This PRP provides the essential parameter set management foundation required for RFC 6184-compliant H.264 RTP payloading.

## Confidence Score: 7/10

Good confidence based on clear RFC specification and existing GStreamer patterns. Some complexity in parameter set timing and dependency management, but scope is focused and achievable.