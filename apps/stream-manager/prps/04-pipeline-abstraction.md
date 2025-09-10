# PRP-04: Pipeline Abstraction Layer

## Overview
Create abstractions for managing GStreamer pipelines with proper state management, error handling, and message bus monitoring.

## Context
- Need to manage multiple independent pipelines
- Must handle pipeline state changes gracefully
- Should abstract common pipeline operations
- Need proper cleanup on shutdown

## Requirements
1. Create Pipeline wrapper struct
2. Implement state change management
3. Setup message bus handling
4. Add pipeline lifecycle methods
5. Create pipeline registry for tracking

## Implementation Tasks
1. Create src/pipeline/mod.rs with Pipeline struct
2. Define PipelineWrapper containing:
   - gst::Pipeline
   - State tracking enum
   - Message bus watch
   - Unique identifier
3. Implement Pipeline lifecycle methods:
   - new() with name
   - start() to Playing state
   - pause() to Paused state
   - stop() to Null state
   - add_element() helper
4. Setup message bus handling:
   - Error message handling
   - EOS handling
   - State change notifications
   - Buffering messages
5. Create PipelineManager:
   - HashMap of active pipelines
   - Add/remove pipeline methods
   - Shutdown all pipelines method
6. Add error types for pipeline failures
7. Implement Drop trait for cleanup

## Validation Gates
```bash
# Test pipeline creation
cargo test --package stream-manager pipeline::tests

# Verify state management
cargo test pipeline_state_changes

# Check cleanup on drop
cargo test pipeline_cleanup
```

## Dependencies
- PRP-03: GStreamer must be initialized

## References
- Pipeline patterns: utils/fallbackswitch/src/fallbacksrc/imp.rs
- Message handling: Search for "message" in test files
- State management: https://gstreamer.freedesktop.org/documentation/additional/design/states.html

## Success Metrics
- Pipelines created and destroyed cleanly
- State changes handled properly
- Message bus events processed
- No pipeline leaks on shutdown

**Confidence Score: 8/10**