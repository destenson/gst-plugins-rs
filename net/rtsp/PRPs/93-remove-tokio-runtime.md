# PRP-RTSP-93: Remove Tokio Runtime and Task Spawning

## Overview
Remove the global RUNTIME and all RUNTIME.spawn calls, replacing them with GStreamer's streaming thread model and GIO async where needed.

## Context
After creating the RTSPConnection wrapper (PRP-92), remove all Tokio runtime usage:
- Remove global RUNTIME lazy_static
- Remove all spawn() calls
- Remove block_on() calls
- Convert async functions to sync or GIO async

## Prerequisites
- PRP-92 must be completed (RTSPConnection wrapper exists)
- GSTREAMER_RTSP_API.md documentation available

## Scope
This PRP ONLY covers:
1. Remove RUNTIME definition
2. Replace spawn() in state changes
3. Replace block_on() in cleanup
4. Convert command queue to GStreamer pattern
5. Update Cargo.toml dependencies

Does NOT include:
- UDP/TCP socket conversion (separate PRP)
- Full transport rewrite

## Implementation Tasks
1. Remove RUNTIME lazy_static from imp.rs
2. Replace spawn() calls in:
   - start() method - use streaming thread
   - stop() method - direct cleanup
   - seek() handling - use pad events
   - play() command - state change
3. Convert Commands enum handling:
   - Remove async from command handlers
   - Use GStreamer's bus for messaging
4. Replace block_on() in teardown:
   - Make synchronous cleanup
   - Use GCond/GMutex if needed
5. Update task_handle to non-async type
6. Remove tokio from Cargo.toml

## Code Locations to Modify
- `imp.rs:2062` - seek spawn
- `imp.rs:2086` - play spawn  
- `imp.rs:2191` - main connection spawn
- `imp.rs:2355` - teardown block_on
- `imp.rs:2369` - join handle block_on
- `imp.rs:2465` - eos handler spawn

## Validation Gates
```bash
# No tokio runtime references
! grep -q "RUNTIME" net/rtsp/src/rtspsrc/imp.rs

# No spawn calls
! grep -q "\.spawn" net/rtsp/src/rtspsrc/imp.rs

# Build succeeds
cargo build -p gst-plugin-rtsp

# State changes work
cargo test -p gst-plugin-rtsp state_changes
```

## Expected Behavior
- State changes work without async tasks
- Commands execute in streaming thread
- Cleanup is synchronous
- No runtime overhead

## Success Criteria
- [ ] All RUNTIME references removed
- [ ] All spawn calls eliminated
- [ ] State changes still function
- [ ] Tests pass
- [ ] No tokio in dependencies

## Risk Assessment
**HIGH RISK** - Core functionality change affecting state management.

## Estimated Effort
4 hours

## Confidence Score
6/10 - Significant architectural change