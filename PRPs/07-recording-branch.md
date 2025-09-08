# PRP-07: Recording Branch Implementation

## Overview
Implement the recording branch using togglerecord and splitmuxsink for segmented, controllable recording.

## Context
- Must support clean start/stop of recording
- Need segmented files for manageable sizes
- Should handle keyframe alignment
- Must track recording state and segments

## Requirements
1. Create recording branch pipeline
2. Configure togglerecord element
3. Setup splitmuxsink for segmentation
4. Implement recording control methods
5. Track recorded segments

## Implementation Tasks
1. Create src/recording/branch.rs module
2. Define RecordingBranch struct:
   - Branch bin containing elements
   - togglerecord element reference
   - splitmuxsink element reference
   - Recording state tracking
   - Segment counter
3. Implement branch creation:
   - Create bin with queue
   - Add togglerecord element
   - Add splitmuxsink element
   - Configure segment duration
   - Setup file naming pattern
4. Configure togglerecord:
   - Set initial record=false
   - Configure is-live based on source
   - Setup property notifications
5. Configure splitmuxsink:
   - Set location pattern with placeholders
   - Configure max-size-time
   - Enable send-keyframe-requests
   - Set muxer (mp4mux or matroskamux)
6. Add control methods:
   - start_recording()
   - stop_recording()
   - is_recording()
   - get_current_segment()
7. Setup signal handlers for segment completion

## Validation Gates
```bash
# Test recording branch creation
cargo test --package stream-manager recording::branch::tests

# Verify togglerecord control
cargo test recording_start_stop

# Check segment creation
cargo test recording_segments
```

## Dependencies
- PRP-06: Branch management for connection

## References
- togglerecord: utils/togglerecord/src/
- splitmuxsink: Search for "splitmuxsink" in examples
- Patterns: https://gstreamer.freedesktop.org/documentation/multifile/splitmuxsink.html

## Success Metrics
- Recording starts/stops cleanly
- Segments created at configured intervals
- File naming follows pattern
- State tracked accurately

**Confidence Score: 9/10**