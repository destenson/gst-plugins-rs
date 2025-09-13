# PRP: H.265 RTP Payloader Implementation

## Problem Statement

Implement H.265/HEVC RTP payloader (`rtph265pay2`) support according to RFC 7798 to enable next-generation video streaming applications. H.265 provides significant compression improvements over H.264 and is essential for 4K/8K video streaming. The implementation should leverage existing H.264 infrastructure while handling H.265-specific requirements.

## Context & Requirements

### H.265 vs H.264 Key Differences
**Parameter Sets:**
- H.265 uses VPS (Video Parameter Set) in addition to SPS/PPS
- Different parameter set dependencies and relationships
- Extended parameter set ID ranges and validation rules

**NAL Unit Types:**
- Different NAL unit type enumeration (ITU-T H.265 Table 7-1)
- IRAP (Intra Random Access Point) pictures vs IDR frames
- New slice types and temporal layer handling

**RTP Payload Differences (RFC 7798 vs RFC 6184):**
- Similar fragmentation (FU) and aggregation (AP) concepts
- Different NAL unit header structure (2 bytes vs 1 byte)
- Modified payload header formats for H.265-specific information

### Reference Implementation Analysis
Study the original C implementation in `gst-plugins-good/gst/rtp/gstrtph265pay.c` for:
- H.265-specific parameter set handling
- NAL unit type classification differences
- RTP payload header construction variations
- Caps negotiation with H.265-specific fields

**Leverage Existing H.264 Infrastructure:**
- Reuse fragmentation framework with H.265 adaptations
- Adapt parameter set management for VPS/SPS/PPS handling
- Extend NAL unit parsing for H.265 NAL unit types
- Follow RtpBasePay2 integration patterns from H.264 implementation

## Implementation Plan

### Target Architecture
Create `net/rtp/src/h265/` module structure mirroring H.264:
- `h265/common/nal_unit.rs` - H.265 NAL unit types and parsing
- `h265/common/parameter_sets.rs` - VPS/SPS/PPS management
- `h265/pay/imp.rs` - H.265 payloader implementation

### Core Components to Implement

1. **H.265 NAL Unit Support**
   - Define H.265 NAL unit types according to ITU-T H.265 specification
   - Handle 2-byte NAL unit headers vs H.264's 1-byte headers
   - Implement IRAP picture detection and keyframe identification
   - Support temporal layer and spatial layer information

2. **Extended Parameter Set Management**
   - Add VPS (Video Parameter Set) support alongside SPS/PPS
   - Handle parameter set dependencies specific to H.265
   - Implement proper VPS/SPS/PPS insertion timing
   - Support H.265 caps generation with parameter set information

3. **H.265 RTP Payload Formatting**
   - Implement RFC 7798 payload header construction
   - Handle H.265-specific fragmentation unit (FU) format
   - Support aggregation packets (AP) with H.265 NAL units
   - Proper temporal ID and layer ID handling in RTP packets

4. **Payloader Integration**
   - Adapt H.264 payloader patterns for H.265 requirements
   - Handle H.265-specific properties and configuration
   - Implement proper caps negotiation for H.265 streams
   - Support H.265 profile and level advertisement

5. **Profile and Level Support**
   - Support common H.265 profiles (Main, Main10, Main Still Picture)
   - Handle different tier and level combinations
   - Proper caps field generation for H.265 streams
   - Compatibility with various H.265 encoders

### Reusable Infrastructure Adaptation
- Extend fragmentation framework to handle 2-byte NAL headers
- Adapt parameter set caching for VPS inclusion
- Modify packet construction for H.265 payload headers
- Reuse RtpBasePay2 integration patterns from H.264

## Validation

### Functional Testing
```bash
# Syntax/Style validation
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit tests for H.265 payloader
cargo test h265::pay --all-features -- --nocapture

# Integration with RTP framework
cargo test --all-targets --features="h265" -- --nocapture

# H.265 encoder integration testing
GST_DEBUG=rtph265pay2:5 gst-launch-1.0 videotestsrc ! x265enc ! h265parse ! rtph265pay2 ! fakesink
```

### Interoperability Testing
- Test with x265 encoder and various H.265 content
- Validate with H.265 decoders and players
- Verify different profile/level combinations
- Test 4K and HDR content payloading

### Performance Validation
- Benchmark against H.264 payloader performance
- Validate high-bitrate content handling (4K/8K)
- Memory usage profiling with large parameter sets
- Fragmentation performance with large HEVC NAL units

## Dependencies

### External Documentation
- **RFC 7798**: H.265 RTP Payload Format specification
- **ITU-T H.265**: HEVC video coding specification
- **H.265 NAL Unit Reference**: Understanding HEVC NAL unit structure

### Codebase Dependencies
- Leverages H.264 infrastructure components (NAL parsing, fragmentation, parameter sets)
- Extends RtpBasePay2 framework patterns
- Uses existing video payloader patterns from AV1 and H.264 implementations

### Test Infrastructure
- H.265 encoder (x265) for test content generation
- Various H.265 profile and level test content
- 4K/HDR test sequences for validation
- H.265 decoder elements for output validation

## Success Criteria

1. **Functionality**
   - Generate RFC 7798 compliant H.265 RTP streams
   - Handle VPS/SPS/PPS parameter sets correctly
   - Support common H.265 profiles and levels
   - Proper fragmentation for large HEVC NAL units

2. **Performance**
   - Performance comparable to H.264 payloader
   - Support for high-bitrate 4K/8K content
   - Efficient parameter set management and insertion
   - Memory usage appropriate for HEVC content complexity

3. **Integration**
   - Standard GStreamer H.265 caps negotiation
   - Compatible with x265 and hardware encoders
   - Seamless integration with existing RTP infrastructure
   - Proper element registration and factory setup

4. **Quality**
   - Interoperability with H.265 decoders and players
   - Maintain stream integrity under network conditions
   - Proper timing and synchronization support
   - Robust error handling for H.265-specific scenarios

## Risk Assessment

**Low-Medium Risk** - Leveraging existing H.264 infrastructure reduces complexity. H.265 differences are well-documented in RFC 7798.

## Estimated Effort

**3-4 hours** - H.265 payloader implementation leveraging H.264 infrastructure with H.265-specific adaptations.

## Implementation Notes

### Key Differences to Handle
- 2-byte NAL unit headers require parsing and fragmentation adjustments
- VPS parameter sets add complexity to parameter set management
- Different IRAP picture types vs H.264 IDR frames
- Extended profile and level space for H.265

### Reuse Strategy
- Maximum reuse of H.264 fragmentation and aggregation logic
- Adapt parameter set management for VPS inclusion
- Extend NAL unit parsing for H.265-specific types
- Reuse RTP packet construction patterns with payload header differences

### Performance Considerations
- H.265 typically produces larger parameter sets requiring efficient caching
- HEVC NAL units can be significantly larger requiring robust fragmentation
- Higher compression efficiency may result in more complex timing requirements
- Consider optimization for common use cases (4K streaming, HDR content)

This PRP provides H.265 video codec support, completing the essential video codec coverage identified as critical for video streaming applications.

## Confidence Score: 8/10

High confidence based on leveraging proven H.264 infrastructure and clear RFC 7798 specification. H.265-specific adaptations are well-documented and achievable within the focused scope.