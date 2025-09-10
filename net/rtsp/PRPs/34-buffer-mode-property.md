# PRP-RTSP-34: Buffer Mode Property Implementation

## Overview
Implement the `buffer-mode` enumeration property to match original rtspsrc buffering algorithm control. This property allows selection between different buffering strategies.

## Context
The original rtspsrc provides a `buffer-mode` property with 5 different buffering algorithms: none (RTP timestamps only), slave (sync to sender), buffer (watermark-based), auto (adaptive), and synced (synchronized clocks). This is a critical missing feature for advanced buffering control.

## Research Context
- Original rtspsrc buffer modes in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- GStreamer RTP jitterbuffer modes documentation
- Buffer mode enum values: none(0), slave(1), buffer(2), auto(3), synced(4)
- Relationship to jitterbuffer configuration and clock synchronization

## Scope
This PRP implements ONLY the property infrastructure:
1. Define BufferMode enumeration with all 5 modes
2. Add `buffer-mode` property with enum type
3. Add property validation and state change restrictions
4. Prepare enum values for future jitterbuffer mode configuration

Does NOT implement:
- Actual buffering algorithm logic
- Jitterbuffer mode switching
- Clock synchronization mechanisms
- Watermark-based buffering logic

## Implementation Tasks
1. Define BufferMode enum: None, Slave, Buffer, Auto, Synced
2. Add buffer_mode field to RtspSrcSettings struct
3. Implement enum-to-string and string-to-enum conversion
4. Register `buffer-mode` property with enum type and default value (Auto)
5. Add property change validation (changeable only in NULL or READY state)
6. Implement getter/setter methods for buffer mode property
7. Add proper enum value documentation for each buffering strategy

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Enum definition and property implementation
- May need separate types module for BufferMode enum

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_buffer_mode_property --all-targets --all-features -- --nocapture

# Enum Conversion Test
cargo test test_buffer_mode_enum_conversions --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
buffer-mode         : Control the buffering algorithm in use
                      flags: readable, writable, changeable only in NULL or READY state
                      Enum "BufferMode" Default: 3, "auto"
                         (0): none             - Only use RTP timestamps
                         (1): slave            - Slave receiver to sender clock
                         (2): buffer           - Do low/high watermark buffering  
                         (3): auto             - Choose mode depending on stream live
                         (4): synced           - Synchronized sender and receiver clocks
```

## Buffer Mode Descriptions
- **none**: Use only RTP timestamps, minimal buffering
- **slave**: Synchronize receiver clock to sender clock
- **buffer**: Use low/high watermark buffering algorithm
- **auto**: Automatically choose mode based on stream characteristics (live vs. recorded)
- **synced**: Use synchronized sender and receiver clocks for precise timing

## Dependencies
- **May require**: Custom enum type implementation for GStreamer property system integration

## Success Criteria
- [ ] BufferMode enum properly defined with all 5 values
- [ ] Property visible in gst-inspect with correct enum values
- [ ] Property accepts string and integer enum values
- [ ] Property restricted to NULL/READY state changes
- [ ] Enum conversion (string ↔ integer) works correctly
- [ ] Default value is "auto" (matching original rtspsrc)

## Risk Assessment
**LOW RISK** - Enum property definition, no complex logic.

## Estimated Effort
2-3 hours

## Confidence Score
8/10 - Enum properties require GStreamer-specific implementation but follow established patterns.