# PRP-17: Disk Rotation and Hot-Swap Support

## Overview
Implement support for disk rotation and hot-swapping, allowing drives to be changed without stopping recording.

## Context
- Operations may swap full drives with empty ones
- Must detect drive removal/addition
- Should migrate recordings seamlessly
- Need to preserve recording continuity

## Requirements
1. Detect disk removal and addition
2. Implement recording migration
3. Handle in-flight writes
4. Support manual rotation triggers
5. Maintain recording integrity

## Implementation Tasks
1. Create src/storage/rotation.rs module
2. Define DiskRotation struct:
   - Active disk tracking
   - Rotation queue
   - Migration state
   - Write buffers
3. Implement disk monitoring:
   - Use udev/device events (Linux)
   - Poll mount points
   - Detect UUID changes
   - Track device serial numbers
4. Create rotation handler:
   - Detect pending removal
   - Buffer incoming data
   - Switch to alternate path
   - Drain buffers to new path
5. Add manual rotation API:
   - Mark disk for rotation
   - Trigger graceful migration
   - Report rotation status
6. Implement write buffering:
   - Memory buffer during rotation
   - Spillover to alternate path
   - Ensure no data loss
7. Add rotation events and logging

## Validation Gates
```bash
# Test rotation detection
cargo test --package stream-manager storage::rotation::tests

# Verify migration handling
cargo test disk_rotation_migration

# Check buffer management
cargo test rotation_buffering
```

## Dependencies
- PRP-16: Storage management foundation
- PRP-07: Recording branch for write handling

## References
- Device events: libudev bindings or polling approach
- Mount detection: /proc/mounts monitoring
- Buffer patterns: Search for buffering examples

## Success Metrics
- Disk changes detected quickly
- Recording continues during rotation
- No data loss during migration
- Smooth transition between disks

**Confidence Score: 6/10**