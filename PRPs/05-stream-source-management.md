# PRP-05: Stream Source Management

## Overview
Implement the core stream source management using fallbacksrc for robust handling of intermittent RTSP and other sources.

## Context
- Each stream needs independent source management
- Must use fallbacksrc for automatic reconnection
- Need to track source health and statistics
- Should support both URI and custom element sources

## Requirements
1. Create StreamSource abstraction
2. Configure fallbacksrc with appropriate timeouts
3. Setup source statistics monitoring
4. Implement source health tracking
5. Add decodebin3 for format handling

## Implementation Tasks
1. Create src/stream/source.rs module
2. Define StreamSource struct with:
   - Unique stream ID
   - Source bin containing fallbacksrc
   - Health statistics
   - Configuration parameters
3. Implement source bin creation:
   - Create bin with fallbacksrc element
   - Configure timeouts from config
   - Add decodebin3 for decoding
   - Setup ghost pads for output
4. Add statistics monitoring:
   - Poll fallbacksrc statistics property
   - Track retry counts
   - Monitor buffering percentage
   - Record last frame timestamp
5. Implement health checking:
   - Define health thresholds
   - Check frame timeouts
   - Monitor retry patterns
   - Generate health status enum
6. Add pad-added signal handling for dynamic pads
7. Create source configuration methods

## Validation Gates
```bash
# Test source creation
cargo test --package stream-manager stream::source::tests

# Verify fallbacksrc configuration
cargo test source_timeout_config

# Check statistics collection
cargo test source_statistics
```

## Dependencies
- PRP-04: Pipeline abstraction for bin management
- PRP-02: Configuration for timeout values

## References
- fallbacksrc usage: utils/fallbackswitch/src/fallbacksrc/
- Statistics property: fallbacksrc documentation
- Bin patterns: Look for Bin::new() usage in codebase

## Success Metrics
- Sources created with proper fallback configuration
- Statistics correctly extracted
- Health status accurately reflects stream state
- Pad connections handled dynamically

**Confidence Score: 8/10**