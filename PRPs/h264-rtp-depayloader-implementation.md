# PRP: H.264 RTP Depayloader Implementation

## Problem Statement

Implement the H.264 RTP depayloader (`rtph264depay2`) that receives RFC 6184 compliant RTP packets and reconstructs H.264 elementary streams. This depayloader must handle single NAL packets, aggregated packets (STAP-A), and fragmented packets (FU-A) while maintaining stream integrity and proper timing.

## Context & Requirements

### RFC 6184 Depayloader Requirements
**Core Functionality:**
- Process incoming H.264 RTP packets according to RFC 6184
- Reconstruct NAL units from single packets, STAP-A, and FU-A packets
- Generate proper H.264 elementary stream output
- Handle parameter set extraction and caps generation
- Implement packet loss detection and recovery

### Integration Architecture
This PRP leverages existing infrastructure:
- NAL unit parsing for reconstructed stream validation
- Parameter set management for caps generation and validation
- Fragmentation unit handling for FU-A packet processing
- RtpBaseDepay2 framework for RTP packet processing

### Reference Implementation Analysis
Study the original C implementation in `gst-plugins-good/gst/rtp/gstrtph264depay.c` for:
- RTP packet processing pipeline (lines 800-1200)
- Fragment reassembly logic (lines 1200-1500)  
- Parameter set extraction and caps generation (lines 400-600)
- Output buffer construction and timing

**Existing Rust Patterns to Follow:**
- Study `net/rtp/src/ac3/depay/imp.rs` for RtpBaseDepay2 integration patterns
- Reference `net/rtp/src/av1/depay/imp.rs` for video depayloader structure
- Follow packet processing patterns from `net/rtp/src/opus/depay/imp.rs`

## Implementation Plan

### Target Architecture
Create `net/rtp/src/h264/depay/imp.rs` implementing H.264-specific depayloader logic with RtpBaseDepay2 integration.

### Core Components to Implement

1. **Element Structure and State Management**
   - Define RtpH264Depay struct with fragmentation state tracking
   - Implement ObjectSubclass with proper element metadata
   - Handle RTP packet sequence tracking and loss detection
   - Manage reassembly buffers and timing information

2. **RTP Packet Processing**
   - Implement handle_packet() method for incoming RTP packets
   - Identify packet types (single NAL, STAP-A, FU-A)
   - Extract payload data and validate RTP header consistency
   - Handle sequence number validation and gap detection

3. **NAL Unit Reconstruction**
   - Process single NAL unit packets directly
   - Handle STAP-A packet disaggregation into multiple NAL units
   - Implement FU-A fragment reassembly with proper ordering
   - Validate reconstructed NAL units for completeness

4. **Stream Output Generation**
   - Construct H.264 elementary stream buffers
   - Apply proper timestamps from RTP packet timing
   - Generate appropriate caps with parameter set information
   - Handle frame boundaries and marker bit interpretation

5. **Error Recovery and Robustness**
   - Detect and handle missing RTP packets
   - Implement fragment loss recovery strategies
   - Handle corrupted or invalid packet data
   - Maintain stream continuity under network conditions

### Integration Requirements
- Use RtpBaseDepay2 framework for RTP packet handling
- Integrate with NAL unit parsing for stream validation
- Utilize parameter set management for caps generation
- Support standard GStreamer buffer and caps semantics

## Validation

### Functional Testing
```bash
# Syntax/Style validation
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit tests for depayloader logic
cargo test h264::depay --all-features -- --nocapture

# Integration with RTP framework
cargo test --all-targets --features="h264" -- --nocapture

# End-to-end payloader/depayloader testing
GST_DEBUG=rtph264depay2:5 gst-launch-1.0 videotestsrc ! x264enc ! rtph264pay2 ! rtph264depay2 ! fakesink
```

### Interoperability Testing
- Test with various H.264 RTP senders (including C payloader)
- Validate with different H.264 decoders downstream
- Test fragment reassembly with simulated packet loss
- Verify timing and synchronization accuracy

### Robustness Testing
- Simulate network packet loss scenarios
- Test with corrupted RTP packet data
- Validate behavior with out-of-order packet delivery
- Handle various H.264 profile and level combinations

## Dependencies

### External Documentation
- **RFC 6184**: H.264 RTP payload format specification
- **RFC 3550**: RTP specification for packet handling
- **ITU-T H.264**: For NAL unit validation and stream structure

### Codebase Dependencies
- Requires NAL unit parsing infrastructure for validation
- Uses parameter set management for caps generation
- Depends on fragmentation handling for FU-A processing
- Integrates with RtpBaseDepay2 framework

### Test Infrastructure
- H.264 RTP test streams from payloader implementation
- Packet loss simulation tools for robustness testing
- H.264 decoder elements for output validation
- Timing analysis tools for synchronization testing

## Success Criteria

1. **Functionality**
   - Correctly process all RFC 6184 packet types
   - Reconstruct valid H.264 elementary streams
   - Handle fragmentation and aggregation properly
   - Generate accurate caps and parameter sets

2. **Robustness**
   - Graceful handling of packet loss scenarios
   - Recovery from fragment reassembly errors
   - Proper error reporting and stream continuity
   - Stable operation under network stress

3. **Performance**
   - Depayloading performance comparable to C implementation
   - Minimal buffering latency for real-time applications
   - Efficient memory usage for reassembly operations

4. **Integration**
   - Standard GStreamer element behavior
   - Compatible with existing H.264 ecosystem
   - Proper caps negotiation and buffer handling
   - Support for various downstream decoders

## Risk Assessment

**Medium Risk** - Fragment reassembly complexity and timing requirements, but well-specified protocols and existing patterns to follow.

## Estimated Effort

**4 hours** - Focused depayloader implementation with comprehensive packet handling and reassembly logic.

## Implementation Notes

### Critical Design Considerations
- Fragment reassembly must handle out-of-order packet delivery
- Timing reconstruction from RTP timestamps is essential for A/V sync
- Parameter set extraction affects downstream decoder configuration
- Memory management for reassembly buffers must be bounded

### Performance Optimization Opportunities
- Minimize buffer copies during reassembly process
- Use efficient data structures for fragment tracking
- Implement zero-copy paths for single NAL unit packets
- Consider memory pool reuse for reassembly operations

### Error Recovery Strategies
- Detect fragment loss and skip incomplete NAL units
- Handle parameter set corruption gracefully
- Implement adaptive buffering based on network conditions
- Provide useful error messages for debugging

### Integration Patterns
- Follow RtpBaseDepay2 patterns for packet processing
- Mirror caps handling from existing video depayloaders  
- Use consistent error handling and logging approaches
- Support standard GStreamer element lifecycle management

This PRP completes the H.264 RTP implementation by providing the essential depayloader functionality for receiving and reconstructing H.264 video streams.

## Confidence Score: 8/10

High confidence based on clear RFC specification and strong foundational infrastructure. Fragment reassembly adds complexity, but existing patterns and focused scope make this very achievable.