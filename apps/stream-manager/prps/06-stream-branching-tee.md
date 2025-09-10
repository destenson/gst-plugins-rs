# PRP-06: Stream Branching with Tee Element

## Overview
Implement stream branching using the tee element to split decoded streams for multiple consumers (recording, inference, preview).

## Context
- Single decode must feed multiple outputs
- Need queue elements for buffering between branches
- Must handle dynamic branch addition/removal
- Should prevent one slow branch from affecting others

## Requirements
1. Add tee element after decoder
2. Implement branch management system
3. Create queue configuration for each branch
4. Handle request pad management
5. Support dynamic branch connection

## Implementation Tasks
1. Create src/stream/branching.rs module
2. Define StreamBranch enum:
   - Recording branch
   - Inference branch
   - Preview branch
   - Custom branch
3. Implement BranchManager struct:
   - Tee element reference
   - Active branches map
   - Queue configurations
4. Add branch creation methods:
   - create_branch() with branch type
   - Configure queue for branch isolation
   - Get request pad from tee
   - Link queue to tee pad
5. Implement branch removal:
   - Unlink from tee
   - Release request pad
   - Clean up queue element
6. Add queue configuration:
   - Max size bytes/buffers/time
   - Leaky behavior for live sources
   - Different configs per branch type
7. Handle tee property configuration

## Validation Gates
```bash
# Test branch creation
cargo test --package stream-manager stream::branching::tests

# Verify multiple branches
cargo test multiple_branches

# Check branch removal cleanup
cargo test branch_cleanup
```

## Dependencies
- PRP-05: Stream source provides decoded output

## References
- Tee element: Search for "tee" in test files
- Queue configuration: Search for "queue" property settings
- Request pads: https://gstreamer.freedesktop.org/documentation/additional/design/request-pads.html

## Success Metrics
- Multiple branches created from single source
- Branches isolated via queues
- Dynamic addition/removal works
- No impact between branches

**Confidence Score: 8/10**