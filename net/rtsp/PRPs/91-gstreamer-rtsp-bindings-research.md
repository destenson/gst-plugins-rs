# PRP-RTSP-91: Research GStreamer RTSP Library Bindings

## Overview
Research and document the available GStreamer RTSP client library bindings in gstreamer-rs to understand what APIs are available for replacing Tokio.

## Context
GStreamer provides a complete RTSP client library (gst-rtsp) that handles:
- RTSP connection management
- TCP/UDP transport negotiation
- RTCP handling
- Keep-alive management
- Async I/O through GIO
- TLS/SSL support

Need to understand what's available in the Rust bindings.

## Research Context
- C library docs: https://gstreamer.freedesktop.org/documentation/gstreamer-rtsp/
- Rust bindings: https://docs.rs/gstreamer-rtsp/
- Original implementation: `~/repos/gstreamer/subprojects/gst-plugins-base/gst-libs/gst/rtsp/`
- Example usage: `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`

## Scope
This PRP ONLY covers:
1. Document available RTSP types in gstreamer-rs
2. Check if RTSPConnection is exposed
3. Document available methods and callbacks
4. Identify any missing bindings
5. Research GIO async patterns in Rust

Does NOT include:
- Implementation code
- Modifications to existing code
- Creating new bindings

## Implementation Tasks
1. Search gstreamer-rs for RTSP-related types
2. Document RTSPConnection equivalent (if available)
3. Check for RTSPMessage, RTSPUrl, RTSPTransport bindings
4. Document GIO MainLoop integration patterns
5. Research how to handle async callbacks in gstreamer-rs
6. Check if manual bindings are needed for missing APIs
7. Document TLS support through GIO

## Research Areas
- `gstreamer-rtsp` crate structure and exports
- `gstreamer-rtsp-sys` FFI bindings
- GIO async I/O patterns in gtk-rs
- MainContext and MainLoop usage
- Callback registration patterns

## Documentation Output
Create `GSTREAMER_RTSP_API.md` with:
- Available RTSP types and methods
- Missing bindings that need creation
- GIO async patterns for Rust
- Example patterns from other GStreamer elements
- TLS/SSL configuration approach

## Validation Gates
```bash
# Verify documentation created
test -f GSTREAMER_RTSP_API.md

# Check key sections exist
grep -q "RTSPConnection" GSTREAMER_RTSP_API.md || echo "Note: RTSPConnection section needed"
grep -q "Async Patterns" GSTREAMER_RTSP_API.md
grep -q "Missing Bindings" GSTREAMER_RTSP_API.md
```

## Expected Output
Complete API documentation showing:
- What RTSP functionality is available
- How to use GIO async in Rust
- What bindings need to be created
- Migration path from Tokio patterns

## Success Criteria
- [ ] All RTSP bindings documented
- [ ] GIO async patterns understood
- [ ] Missing bindings identified
- [ ] Clear API usage examples found
- [ ] TLS configuration approach documented

## Risk Assessment
**LOW RISK** - Research and documentation only.

## Estimated Effort
2-3 hours

## Confidence Score
9/10 - Research task with clear objectives