# PRP: H.264 RTP Payloader Core Implementation

## Problem Statement

Implement the core H.264 RTP payloader (`rtph264pay2`) that integrates NAL unit parsing, parameter set management, and fragmentation into a working GStreamer element. This payloader must follow RFC 6184 specifications and integrate with the modern RtpBasePay2 framework to enable H.264 video streaming applications.

## Context & Requirements

### Integration Architecture
This PRP builds upon the foundational components:
- NAL unit parsing infrastructure (previous PRP)
- Parameter set management system (previous PRP) 
- Fragmentation unit implementation (previous PRP)
- Existing RtpBasePay2 framework patterns

### RFC 6184 Payloader Requirements
**Core Functionality:**
- Accept H.264 elementary streams as input
- Generate RFC 6184 compliant RTP packets
- Handle single NAL unit packets, aggregation packets (STAP-A), and fragmentation units (FU-A)
- Support parameter set insertion and management
- Implement proper caps negotiation with clock-rate=90000

### Reference Implementation Study
Analyze the original C implementation in `gst-plugins-good/gst/rtp/gstrtph264pay.c` for:
- Buffer processing pipeline (lines 1400-1600)
- Caps negotiation logic (lines 200-400) 
- Packetization decision logic (single/aggregate/fragment)
- Integration with GstRTPBasePayload patterns

**Existing Rust Patterns to Follow:**
- Study `net/rtp/src/ac3/pay/imp.rs` for audio payloader structure with RtpBasePay2
- Reference `net/rtp/src/av1/pay/imp.rs` for video payloader patterns
- Follow property and configuration patterns from `net/rtp/src/opus/pay/imp.rs`

## Implementation Plan

### Target Architecture
Create `net/rtp/src/h264/pay/imp.rs` implementing H.264-specific payloader logic with RtpBasePay2 integration.

### Core Components to Implement

1. **Element Structure and Configuration**
   - Define RtpH264Pay struct with appropriate state management
   - Implement ObjectSubclass with proper element metadata
   - Add H.264-specific properties (aggregation-mode, sprop-parameter-sets, etc.)
   - Handle caps negotiation for H.264 RTP streams

2. **Buffer Processing Pipeline**
   - Implement handle_buffer() method for incoming H.264 data
   - Parse input buffers into NAL units using parsing infrastructure
   - Make packetization decisions based on NAL unit sizes and MTU
   - Generate appropriate RTP packets (single/STAP-A/FU-A)

3. **Packetization Strategy**
   - Single NAL unit packets for medium-sized NAL units
   - STAP-A aggregation for multiple small NAL units
   - FU-A fragmentation for large NAL units
   - Parameter set insertion at keyframes and intervals

4. **RTP Packet Generation**
   - Use RtpBasePay2 packet queueing system
   - Set appropriate timestamps and sequence numbers  
   - Handle marker bit setting for frame boundaries
   - Implement proper payload type and clock rate

5. **Configuration and Properties**
   - Support standard H.264 payloader properties
   - Handle sprop-parameter-sets caps field
   - Implement aggregation mode configuration
   - Support MTU-based packetization decisions

### Integration Points
- Seamless integration with existing RtpBasePay2 framework
- Proper GStreamer element registration and factory setup
- Support for standard H.264 caps negotiation
- Integration with parameter set management system

## Validation

### Functional Testing
```bash
# Syntax/Style validation
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit tests for payloader logic
cargo test h264::pay --all-features -- --nocapture

# Integration with RTP framework
cargo test --all-targets --features="h264" -- --nocapture

# GStreamer element functionality
GST_DEBUG=rtph264pay2:5 gst-launch-1.0 videotestsrc ! x264enc ! h264parse ! rtph264pay2 ! fakesink
```

### Interoperability Testing
- Test with various H.264 encoders (x264, openh264, hardware encoders)
- Validate with standard RTP receivers and players
- Verify caps negotiation with downstream elements
- Test packetization decisions with various content types

### Performance Validation
- Benchmark payloadization throughput vs C implementation
- Memory allocation profiling during payloading
- Latency measurement for various packetization strategies
- High-bitrate video streaming validation

## Dependencies

### External Documentation
- **RFC 6184**: Complete H.264 RTP payload format specification
- **GStreamer RTP documentation**: RTP caps and element patterns
- **ITU-T H.264**: For NAL unit type understanding

### Codebase Dependencies
- Requires NAL unit parsing infrastructure
- Depends on parameter set management system
- Uses fragmentation unit implementation
- Integrates with RtpBasePay2 framework

### Test Infrastructure
- H.264 encoder elements for test content generation
- RTP stream analysis tools for validation
- Various H.264 profile/level test content
- Network simulation for MTU testing

## Success Criteria

1. **Functionality**
   - Generate RFC 6184 compliant RTP streams
   - Handle all standard H.264 content types
   - Proper packetization strategy selection
   - Parameter set insertion and management

2. **Performance**
   - Payloadization performance within 20% of C implementation  
   - Memory usage comparable to existing payloaders
   - Support high-bitrate video content (4K/8K)

3. **Integration**
   - Seamless GStreamer element integration
   - Standard H.264 caps negotiation support
   - Compatible with existing RTP infrastructure
   - Proper error handling and state management

4. **Interoperability**
   - Works with common H.264 encoders
   - Compatible with standard RTP receivers
   - Supports various H.264 profiles and levels
   - Handles diverse network MTU requirements

## Risk Assessment

**Medium Risk** - Complex integration of multiple components, but building on solid foundations with clear specifications.

## Estimated Effort

**4 hours** - Core payloader implementation integrating existing infrastructure components with comprehensive validation.

## Implementation Notes

### Critical Design Decisions
- Packetization strategy must balance latency vs efficiency
- Parameter set insertion timing affects decoder compatibility
- MTU handling must be robust across network conditions
- Error recovery should maintain stream integrity

### Integration Patterns to Follow
- Mirror the element structure from existing video payloaders
- Use similar property naming and behavior as C implementation
- Follow RtpBasePay2 patterns for packet queueing and timing
- Implement standard GStreamer element lifecycle management

### Performance Considerations
- Minimize buffer copies during packetization
- Use existing infrastructure efficiently to avoid redundant processing
- Consider zero-copy paths for single NAL unit packets
- Optimize common case scenarios (1080p video at typical bitrates)

This PRP delivers the complete H.264 RTP payloader functionality, enabling H.264 video streaming applications with the Rust RTP implementation.

## Confidence Score: 8/10

High confidence based on solid foundational components and clear integration patterns from existing payloaders. The main complexity is in the integration logic, but the scope is well-defined and achievable.