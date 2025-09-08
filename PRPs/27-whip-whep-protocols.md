# PRP-27: WHIP/WHEP Protocol Support

## Overview
Implement WHIP (WebRTC-HTTP Ingestion Protocol) and WHEP (WebRTC-HTTP Egress Protocol) for standardized WebRTC streaming.

## Context
- WHIP/WHEP are emerging standards
- Simplify WebRTC integration
- HTTP-based signaling
- Already have webrtchttp plugin

## Requirements
1. Implement WHIP ingestion endpoint
2. Implement WHEP playback endpoint
3. Handle SDP negotiation via HTTP
4. Support authentication
5. Integrate with stream management

## Implementation Tasks
1. Create src/webrtc/whip_whep.rs module
2. Define WHIP handler:
   - POST endpoint for ingestion
   - SDP offer processing
   - Resource allocation
   - Bearer token auth
3. Implement WHIP ingestion:
   - Parse SDP offer
   - Create webrtcbin sink
   - Generate SDP answer
   - Return 201 Created
   - Provide resource URL
4. Define WHEP handler:
   - POST endpoint for playback
   - Stream selection
   - SDP offer processing
   - Viewer authentication
5. Implement WHEP playback:
   - Parse SDP offer
   - Create webrtcbin source
   - Connect to stream tee
   - Generate SDP answer
   - Return 201 Created
6. Add HTTP endpoints:
   - /whip/{stream_id} for ingestion
   - /whep/{stream_id} for playback
   - DELETE for teardown
   - PATCH for ICE trickle
7. Integrate with existing:
   - Register as stream source
   - Connect to recording
   - Apply to inference

## Validation Gates
```bash
# Test WHIP ingestion
cargo test --package stream-manager webrtc::whip_whep::tests

# Test WHEP playback
cargo test whep_playback

# Test with GStreamer clients
gst-launch-1.0 webrtcsink signaller=whip-client whip-endpoint=http://localhost:8080/whip/test
```

## Dependencies
- PRP-26: WebRTC infrastructure
- PRP-11: REST API for endpoints

## References
- WHIP spec: https://datatracker.ietf.org/doc/draft-ietf-wish-whip/
- WHEP spec: https://datatracker.ietf.org/doc/draft-murillo-whep/
- webrtchttp plugin: net/webrtchttp in gst-plugins-rs

## Success Metrics
- WHIP ingestion works
- WHEP playback works
- Standard clients compatible
- Clean HTTP API

**Confidence Score: 7/10**