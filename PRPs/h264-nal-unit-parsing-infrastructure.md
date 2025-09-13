# PRP: H.264 NAL Unit Parsing Infrastructure

## Problem Statement

The Rust RTP implementation lacks H.264 video codec support, which blocks 90% of video streaming applications. H.264 RTP payloading requires parsing NAL (Network Abstraction Layer) units according to ITU-T H.264 specification and RFC 6184. This PRP establishes the foundational NAL unit parsing infrastructure that all H.264 RTP functionality depends on.

## Context & Requirements

### Current State Analysis
From the RTP Architecture Research Report, the original C implementation in `gst-plugins-good/gst/rtp/gstrtph264pay.c` provides reference behavior. The Rust implementation needs to follow similar patterns but with memory-safe parsing.

**Existing Patterns to Follow:**
- Look at `net/rtp/src/av1/common/` for video codec parsing patterns in Rust RTP
- Study `net/rtp/src/ac3/ac3_audio_utils.rs` for frame header parsing approach
- Reference `net/rtp/src/basepay/mod.rs` for integration patterns with RtpBasePay2

### H.264 NAL Unit Structure Requirements
**ITU-T H.264 Specification Context:**
- NAL units are the basic data structures in H.264 bitstreams
- Each NAL unit starts with a NAL unit header (1 byte)
- NAL unit types include SPS, PPS, IDR slices, non-IDR slices, etc.
- Start codes (0x000001 or 0x00000001) separate NAL units in Annex B format

**RFC 6184 RTP Payload Requirements:**
- NAL units must be identified and classified for RTP packetization
- Parameter sets (SPS/PPS) need special handling
- NAL unit size and boundaries must be determined accurately
- Support both Annex B (start codes) and AVCC (length prefixed) formats

## Implementation Plan

### Target Architecture
Create `net/rtp/src/h264/common/nal_unit.rs` following the pattern established in `av1/common/` module structure.

### Core Components to Implement

1. **NAL Unit Type Enumeration**
   - Define all H.264 NAL unit types according to ITU-T H.264 Table 7-1
   - Include parameter sets, slice types, SEI, etc.
   - Reference: ITU-T H.264 Section 7.4.1

2. **NAL Unit Header Parser**
   - Parse forbidden_zero_bit, nal_ref_idc, nal_unit_type fields
   - Validate header according to H.264 constraints
   - Handle error cases gracefully

3. **NAL Unit Boundary Detection**
   - Implement start code detection for Annex B format
   - Implement length-prefixed parsing for AVCC format
   - Auto-detect format based on input characteristics

4. **NAL Unit Classification**
   - Identify parameter sets (SPS, PPS) vs media data
   - Detect IDR frames for keyframe identification
   - Classify slice types for fragmentation decisions

5. **Memory-Safe Buffer Handling**
   - Use Rust slice references for zero-copy parsing
   - Implement bounds checking for all buffer access
   - Handle incomplete NAL units gracefully

### Integration Points
- Design for use by both payloader and depayloader implementations
- Provide iterator interface for processing multiple NAL units
- Include validation methods for RFC 6184 compliance
- Support both synchronous and streaming parsing modes

## Validation

### Functional Testing
```bash
# Syntax/Style validation
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit tests
cargo test h264::common::nal_unit --all-features -- --nocapture

# Integration with existing RTP framework
cargo test --all-targets --features="h264" -- --nocapture
```

### Test Data Requirements
- Valid H.264 NAL unit sequences from ffmpeg test vectors
- Invalid NAL units for error handling validation  
- Both Annex B and AVCC format samples
- Parameter set examples (SPS, PPS)
- Various slice types and frame structures

### Performance Validation
- Benchmark parsing performance vs C implementation reference
- Memory usage profiling for large video frames
- Zero-copy validation - ensure no unnecessary allocations

## Dependencies

### External Documentation
- **ITU-T H.264 Specification**: https://www.itu.int/rec/T-REC-H.264/en
- **RFC 6184**: https://tools.ietf.org/html/rfc6184 (H.264 RTP payload format)
- **H.264 NAL Unit Reference**: https://yumichan.net/video-processing/video-compression/introduction-to-h264-nal-unit/

### Codebase Dependencies
- Study existing `av1/common/` module for video codec patterns
- Reference `ac3/ac3_audio_utils.rs` for parsing utilities approach
- Follow error handling patterns from `basepay/mod.rs`

### External Libraries
- Consider `h264-reader` crate for reference implementation patterns
- Evaluate `nom` parser combinator library for robust parsing
- Review `bytes` crate for efficient buffer handling

## Success Criteria

1. **Functionality**
   - Parse all standard H.264 NAL unit types correctly
   - Handle both Annex B and AVCC formats
   - Detect NAL unit boundaries accurately
   - Classify parameter sets vs media data

2. **Performance**  
   - Zero-copy parsing where possible
   - Performance within 20% of reference C implementation
   - Minimal memory allocations during parsing

3. **Robustness**
   - Handle malformed input gracefully
   - Comprehensive error reporting
   - Memory safety guaranteed by Rust type system

4. **Integration**
   - Clean API for payloader/depayloader use
   - Compatible with RtpBasePay2 framework patterns
   - Testable with standard H.264 test vectors

## Risk Assessment

**Low Risk** - Foundation implementation with clear specifications and reference implementations available.

## Estimated Effort

**3-4 hours** - Focused implementation of core NAL unit parsing with comprehensive testing.

## Implementation Notes

### Key Patterns to Mirror
- Follow the module structure pattern from `av1/common/`
- Use similar error handling approach as `ac3_audio_utils.rs`
- Apply zero-copy parsing techniques from existing payloaders

### Critical Implementation Details
- Pay special attention to NAL unit header bit field parsing
- Ensure proper handling of emulation prevention bytes (0x03)
- Implement robust start code detection for various input formats
- Consider future extensibility for H.265 NAL unit parsing

This PRP establishes the critical foundation for all H.264 RTP functionality and unblocks subsequent H.264 implementation PRPs.

## Confidence Score: 8/10

High confidence due to clear specifications, existing patterns in codebase, and well-defined scope. The main risk is ensuring parsing performance meets requirements, but the scope is focused enough for successful single-pass implementation.