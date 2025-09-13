# PRP: RTSP Transport Negotiation with Native Bindings

## Overview
Replace the custom transport parsing and negotiation logic with the new RTSPTransport bindings, providing robust transport configuration and negotiation for SETUP requests.

## Background
The current implementation has custom transport parsing in `transport.rs` that manually constructs transport headers and parses server responses. The new RTSPTransport bindings provide complete transport negotiation with proper RFC compliance and edge case handling.

## Requirements
- Use RTSPTransport for all transport configuration
- Support UDP, TCP, and multicast transports as currently implemented
- Maintain transport selection priority system
- Preserve per-stream transport configuration capability

## Technical Context
The RTSPTransport API provides:
- Transport creation: `RTSPTransport::new()` 
- Parsing: `RTSPTransport::parse(transport_str)`
- Configuration: `set_lower_transport()`, `set_profile()`, `set_client_port()`
- Serialization: `transport.as_text()`
- Builder pattern: `RTSPTransportBuilder` for convenient setup

Current implementation to replace:
- `net/rtsp/src/rtspsrc/transport.rs` - RtspTransportInfo enum
- Transport header construction in `imp.rs`
- Transport parsing from SETUP responses

## Implementation Tasks
1. Replace RtspTransportInfo with RTSPTransport wrapper
2. Use RTSPTransportBuilder for creating transport requests
3. Parse server transport responses with RTSPTransport::parse()
4. Update transport priority selection to work with RTSPTransport
5. Implement transport fallback using RTSPTransport configuration
6. Update UDP socket binding based on RTSPTransport ports
7. Handle interleaved channel assignment for TCP transport
8. Update multicast configuration from RTSPTransport

## Testing Approach
- Unit tests for transport building and parsing
- Test all transport combinations (UDP, TCP, multicast)
- Verify transport fallback scenarios
- Test with servers that modify transport parameters

## Validation Gates
```bash
# Build and lint
cargo build --package gst-plugin-rtsp --all-features
cargo clippy --package gst-plugin-rtsp --all-features -- -D warnings

# Transport-specific tests
cargo test --package gst-plugin-rtsp transport

# Integration tests for different transports
cargo test --package gst-plugin-rtsp --features integration -- transport_
```

## Success Metrics
- Correctly negotiates transport with various RTSP servers
- Handles transport parameter modifications by server
- Transport fallback works as expected
- No regression in transport selection logic

## Dependencies
- RTSPConnection foundation (previous PRP)
- RTSPTransport bindings from gstreamer-rs

## Risk Mitigation
- Maintain mapping between old and new transport representations
- Add extensive logging for transport negotiation
- Keep fallback to manual parsing if RTSPTransport::parse fails

## References
- RTSPTransport API: Local gstreamer-rs/gstreamer-rtsp/src/rtsp_transport.rs
- RFC 2326 Section 12.39 (Transport header)
- Current implementation: `net/rtsp/src/rtspsrc/transport.rs`

## Confidence Score: 9/10
Well-defined API with clear mapping. RTSPTransport handles complexity that we currently do manually.