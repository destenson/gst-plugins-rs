# PRP-RTSP-90: Analysis and Inventory of Tokio Usage

## Overview
Document and analyze all Tokio/async usage in the current RTSP implementation to prepare for replacement with GStreamer's RTSP client library. This is a research and documentation task only.

## Context
The current implementation uses Tokio for async networking instead of GStreamer's built-in RTSP client library (gst-rtsp). This is architecturally wrong because:
- GStreamer has its own threading model and streaming threads
- The gst-rtsp library provides complete RTSP client functionality
- Manual async/await adds unnecessary complexity and thread-safety issues
- Duplicates functionality that already exists in GStreamer

## Research Context
- Original rtspsrc: `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- Uses `GstRTSPConnection` from gst-libs/gst/rtsp/
- No manual thread creation - uses GStreamer's streaming threads
- Async I/O handled by GIO internally

## Scope
This PRP ONLY covers:
1. Document all files using Tokio
2. Identify all async functions and their purposes
3. Map Tokio functionality to GStreamer equivalents
4. Create dependency graph of async code
5. Document data flow through async tasks

Does NOT include:
- Any code changes
- Implementation of replacements
- Removal of Tokio code

## Implementation Tasks
1. Create comprehensive list of all Tokio imports and usage
2. Document each RUNTIME.spawn location and purpose
3. Map each async task to its GStreamer equivalent
4. Document all mpsc channels and their communication patterns
5. Identify all UDP/TCP socket handling code
6. Document the command queue pattern
7. Create visual flow diagram of async task communication

## Files to Analyze
- `net/rtsp/src/rtspsrc/imp.rs` - Main implementation with RUNTIME usage
- `net/rtsp/src/rtspsrc/transport.rs` - UDP socket handling
- `net/rtsp/src/rtspsrc/connection_racer.rs` - TCP connection racing
- `net/rtsp/src/rtspsrc/tcp_message.rs` - Async TCP I/O
- `net/rtsp/src/rtspsrc/session_manager.rs` - Session management with channels

## Documentation Output
Create `TOKIO_ANALYSIS.md` with:
- Complete inventory of async code
- Mapping table: Tokio component -> GStreamer equivalent
- Dependency graph showing task relationships
- Risk assessment for each component removal

## Validation Gates
```bash
# Verify documentation created
test -f TOKIO_ANALYSIS.md

# Check documentation completeness
grep -q "transport.rs" TOKIO_ANALYSIS.md
grep -q "connection_racer.rs" TOKIO_ANALYSIS.md
grep -q "tcp_message.rs" TOKIO_ANALYSIS.md
grep -q "session_manager.rs" TOKIO_ANALYSIS.md
```

## Expected Output
A complete technical document that can guide the systematic removal of Tokio, showing:
- Every async entry point
- Data flow between components
- GStreamer API equivalents for each Tokio component
- Order of removal to maintain functionality

## Success Criteria
- [ ] All Tokio usage documented
- [ ] All async tasks mapped to purposes
- [ ] GStreamer equivalents identified for each component
- [ ] Clear removal strategy documented
- [ ] No code changes made

## Risk Assessment
**LOW RISK** - Pure documentation task with no code changes.

## Estimated Effort
2-3 hours

## Confidence Score
9/10 - Straightforward documentation task