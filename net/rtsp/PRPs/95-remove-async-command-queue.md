# PRP-RTSP-95: Remove Async Command Queue Pattern

## Overview
Replace the mpsc channel-based command queue with GStreamer's standard element communication patterns using signals, properties, and pad events.

## Context
Current implementation uses tokio::sync::mpsc for command passing:
- Commands enum with Play, Pause, Teardown, etc.
- Async command processing loop
- Channel for command transmission

This should use GStreamer patterns:
- State changes for Play/Pause
- Pad events for Seek
- Element signals for custom commands

## Prerequisites
- PRP-93 completed (Runtime removed)
- Understanding of GStreamer element communication

## Scope
This PRP ONLY covers:
1. Remove Commands enum
2. Remove mpsc channel usage
3. Implement state change handling
4. Convert seek to pad event
5. Remove command processing loop

Does NOT include:
- Connection handling changes
- Transport modifications

## Implementation Tasks
1. Remove command infrastructure:
   - Delete Commands enum
   - Remove cmd_queue field
   - Remove mpsc imports
2. Update state change handling:
   - Move Play logic to PAUSED_TO_PLAYING
   - Move Pause logic to PLAYING_TO_PAUSED
   - Handle teardown in PAUSED_TO_READY
3. Convert seek handling:
   - Remove seek command
   - Handle seek events on sink pad
   - Use standard segment handling
4. Remove command loop:
   - Delete command processing loop
   - Remove async command handlers
   - Clean up related functions
5. Update EOS handling:
   - Remove command sending from EOS
   - Handle EOS in streaming thread

## Code Locations
- `imp.rs:650-750` - Commands enum definition
- `imp.rs:2060-2090` - Command sending in state changes
- `imp.rs:2200-2400` - Command processing loop
- `imp.rs:2465` - EOS command sending
- `session_manager.rs` - Remove mpsc usage

## GStreamer Patterns
- State changes: change_state() virtual method
- Seek: src_event() / sink_event() handlers
- Custom commands: action signals
- Synchronization: GCond/GMutex

## Validation Gates
```bash
# No mpsc usage
! grep -q "mpsc::" net/rtsp/src/rtspsrc/

# No Commands enum
! grep -q "enum Commands" net/rtsp/src/rtspsrc/

# State changes work
cargo test -p gst-plugin-rtsp state_changes

# Seek handling works
cargo test -p gst-plugin-rtsp seek
```

## Expected Behavior
- State changes trigger appropriate actions
- Seeks handled through pad events
- No async command queue needed
- Synchronous command execution

## Success Criteria
- [ ] Commands enum removed
- [ ] mpsc channels eliminated
- [ ] State changes functional
- [ ] Seek events handled
- [ ] Tests pass

## Risk Assessment
**HIGH RISK** - Fundamental control flow change.

## Estimated Effort
3-4 hours

## Confidence Score
5/10 - Major architectural shift