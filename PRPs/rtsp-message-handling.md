# PRP: RTSP Message API Migration

## Overview
Migrate from rtsp-types crate to the native RTSPMessage API provided by gstreamer-rtsp bindings for all RTSP message construction, parsing, and manipulation.

## Background
Currently using the rtsp-types crate for RTSP protocol messages. The gstreamer-rtsp bindings provide RTSPMessage with deeper GStreamer integration, better memory management, and additional protocol features.

## Requirements
- Replace rtsp-types message types with RTSPMessage
- Maintain all current RTSP methods (OPTIONS, DESCRIBE, SETUP, PLAY, etc.)
- Preserve header manipulation capabilities
- Keep SDP parsing integration

## Technical Context
RTSPMessage API features:
- Message types: Request, Response, Data
- Header management with RTSPHeaderField enum
- Body handling for SDP and other content
- Integration with RTSPConnection send/receive

Current usage of rtsp-types:
- Request building in `imp.rs`
- Response parsing throughout
- Header manipulation for authentication
- SDP body extraction from DESCRIBE

## Implementation Tasks
1. Replace rtsp_types::Message with RTSPMessage
2. Update request building to use RTSPMessage methods
3. Convert response handling to RTSPMessage API
4. Migrate header field access to RTSPHeaderField
5. Update authentication header handling
6. Adapt SDP extraction from message body
7. Update data message handling for interleaved mode
8. Convert message logging/debugging output

## Testing Approach
- Unit tests for message construction
- Verify all RTSP methods work correctly
- Test header manipulation edge cases
- Validate SDP parsing from messages

## Validation Gates
```bash
# Build and test
cargo build --package gst-plugin-rtsp --all-features
cargo clippy --package gst-plugin-rtsp -- -D warnings

# Message handling tests
cargo test --package gst-plugin-rtsp message

# Protocol compliance tests
cargo test --package gst-plugin-rtsp rtsp_protocol
```

## Success Metrics
- All RTSP methods work as before
- Correct header handling including custom headers
- SDP parsing works without changes
- No memory leaks in message handling

## Dependencies
- RTSPConnection for send/receive integration
- Existing SDP parsing code

## Risk Mitigation
- Create compatibility shim for gradual migration
- Extensive logging of message content
- Validate against RTSP protocol test suite
- Keep rtsp-types as dev-dependency for comparison

## References
- RTSPMessage API in gstreamer-rtsp bindings
- RTSP methods in RFC 2326
- Current usage: Throughout `net/rtsp/src/rtspsrc/imp.rs`

## Confidence Score: 8/10
Clear API migration with good documentation. Main challenge is extensive usage throughout codebase.