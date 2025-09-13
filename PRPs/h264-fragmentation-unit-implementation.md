# PRP: H.264 Fragmentation Unit (FU-A) Implementation

## Problem Statement

H.264 NAL units often exceed RTP MTU size and must be fragmented across multiple RTP packets using Fragmentation Units (FU-A) according to RFC 6184 Section 5.8. This is essential for transmitting high-definition video content where NAL units can be hundreds of kilobytes. Proper fragmentation ensures reliable transmission while maintaining temporal and sequence integrity.

## Context & Requirements

### RFC 6184 Fragmentation Specification
**FU-A Packet Structure Requirements:**
- FU indicator (replaces NAL unit header)
- FU header (start/end/reserved bits + NAL unit type)
- Fragmented NAL unit payload (without original NAL header)
- Proper sequence numbering across fragments
- Marker bit handling on final fragment

### Reference Implementation Analysis
The original C implementation in `gst-plugins-good/gst/rtp/gstrtph264pay.c` around lines 800-1200 shows fragmentation logic. Key patterns include:
- Fragment size calculation based on MTU
- Proper FU-A header construction
- Sequence number management across fragments
- Marker bit setting for fragment completion

**Existing Patterns to Follow:**
- Study `net/rtp/src/av1/pay/imp.rs` for video payload fragmentation patterns
- Reference `net/rtp/src/basepay/mod.rs` for MTU handling and packet construction
- Look at `net/rtp/src/mp2t/pay/imp.rs` for multi-packet payload handling

## Implementation Plan

### Target Architecture
Create `net/rtp/src/h264/common/fragmentation.rs` with FU-A packet generation and reassembly functionality.

### Core Components to Implement

1. **FU-A Header Construction**
   - Implement FU indicator byte generation
   - Handle FU header with start/end/reserved bits
   - Preserve NAL unit type in fragmented packets
   - Validate fragment header structure

2. **Fragmentation Logic**
   - Calculate optimal fragment sizes based on MTU constraints
   - Handle RTP header overhead in size calculations
   - Split large NAL units across multiple fragments
   - Preserve NAL unit boundaries and integrity

3. **Fragment Sequencing**
   - Maintain proper RTP sequence numbers across fragments
   - Handle fragment ordering and dependencies
   - Implement marker bit logic for final fragments
   - Track fragmentation state across multiple NAL units

4. **Reassembly Support** (for depayloader)
   - Detect FU-A packets and extract fragments
   - Reconstruct original NAL units from fragments
   - Handle missing fragment detection and recovery
   - Validate reassembled NAL unit integrity

5. **Memory Management**
   - Efficient fragment buffer allocation and reuse
   - Zero-copy fragmentation where possible
   - Handle partial fragment cleanup on errors
   - Bounded memory usage for fragmentation state

### Integration Requirements
- Use NAL unit parsing infrastructure from previous PRP
- Integrate with RtpBasePay2 packet queue system
- Support MTU discovery and dynamic adjustment
- Handle edge cases like very small MTU sizes

## Validation

### Functional Testing
```bash
# Syntax/Style validation
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit tests for fragmentation logic
cargo test h264::common::fragmentation --all-features -- --nocapture

# Integration with payloader framework
cargo test h264::pay::fragmentation --all-features -- --nocapture

# End-to-end fragmentation/reassembly testing
cargo test h264::integration::fragmentation --all-features -- --nocapture
```

### Test Scenarios
- Large IDR frames requiring multiple fragments
- Small NAL units that don't require fragmentation
- Edge cases: MTU barely larger than RTP header
- Fragment loss and recovery scenarios
- Mixed fragmented and non-fragmented streams

### Performance Validation
- Benchmark fragmentation overhead vs C implementation
- Memory allocation profiling during fragmentation
- Throughput testing with various MTU sizes
- Latency impact of fragmentation processing

## Dependencies

### External Documentation
- **RFC 6184 Section 5.8**: Fragmentation Units (FUs)
- **RFC 3984**: Previous H.264 RTP specification (deprecated but useful reference)
- **H.264 bitstream examples**: For testing with real video content

### Codebase References
- Requires H.264 NAL unit parsing infrastructure
- Uses RtpBasePay2 MTU and packet handling
- Follows fragmentation patterns from existing video payloaders

### Test Dependencies
- H.264 test vectors with large NAL units
- RTP packet analysis tools for validation
- Network simulation for fragment loss testing

## Success Criteria

1. **RFC Compliance**
   - Generate valid FU-A packets according to RFC 6184
   - Proper fragment header construction and sequencing
   - Correct marker bit and sequence number handling
   - Interoperability with standard H.264 RTP receivers

2. **Performance**
   - Fragmentation overhead <5% compared to unfragmented transmission
   - Memory usage bounded and predictable
   - Support for high-bitrate video streams (4K/8K)

3. **Robustness**
   - Handle various MTU sizes gracefully
   - Recover from fragment processing errors
   - Maintain stream integrity under network conditions

4. **Integration**
   - Seamless integration with H.264 payloader architecture
   - Support for dynamic MTU changes
   - Clean separation of fragmentation and payload logic

## Risk Assessment

**Medium Risk** - Complex packet boundary calculations and sequence management, but well-defined by RFC specification.

## Estimated Effort

**4 hours** - Focused implementation of FU-A fragmentation with comprehensive validation testing.

## Implementation Notes

### Critical Implementation Details
- Fragment size calculations must account for all RTP header overhead
- FU-A headers must preserve NAL unit type information correctly
- Sequence number continuity across fragments is essential
- Marker bit logic determines fragment completion detection

### Performance Considerations  
- Minimize memory copies during fragmentation process
- Use buffer slicing for zero-copy fragmentation where possible
- Consider SIMD optimization for high-throughput scenarios
- Implement efficient fragment buffer management

### Edge Case Handling
- Handle MTU sizes smaller than minimum H.264 requirements
- Manage fragmentation of very large NAL units (>1MB)
- Support streams mixing fragmented and non-fragmented content
- Graceful degradation when fragmentation resources exhausted

This PRP implements the critical fragmentation capability required for real-world H.264 video streaming over RTP networks.

## Confidence Score: 7/10

Good confidence due to clear RFC specification and existing video fragmentation patterns in codebase. Some complexity in sequence management and performance optimization, but scope is well-defined.