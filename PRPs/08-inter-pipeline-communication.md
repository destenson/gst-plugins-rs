# PRP-08: Inter-Pipeline Communication Setup

## Overview
Implement inter-pipeline communication using intersink/intersrc elements for decoupled inference and processing pipelines.

## Context
- Inference should run in separate pipeline for isolation
- Need zero-copy transfer between pipelines
- Must support dynamic pipeline connection
- Should handle pipeline lifecycle independently

## Requirements
1. Setup intersink elements in main pipeline
2. Create corresponding intersrc consumers
3. Implement pipeline coupling logic
4. Handle connection state management
5. Support multiple consumers per producer

## Implementation Tasks
1. Create src/pipeline/inter.rs module
2. Define InterConnection struct:
   - Producer ID (intersink name)
   - Consumer pipeline references
   - Connection state
3. Implement producer setup:
   - Add intersink to branch
   - Configure unique producer ID
   - Set producer properties
4. Create consumer pipeline builder:
   - Create new pipeline
   - Add intersrc with matching ID
   - Configure consumer properties
   - Return pipeline reference
5. Add connection management:
   - Register producer/consumer pairs
   - Track connection state
   - Handle disconnection
6. Implement cleanup handling:
   - Detect consumer pipeline shutdown
   - Clean up orphaned producers
   - Handle producer removal
7. Add connection monitoring

## Validation Gates
```bash
# Test inter connection
cargo test --package stream-manager pipeline::inter::tests

# Verify producer/consumer linking
cargo test inter_connection

# Check multiple consumers
cargo test multiple_consumers
```

## Dependencies
- PRP-04: Pipeline abstraction
- PRP-06: Branch management for intersink placement

## References
- intersink/intersrc: generic/inter/src/
- Examples: generic/inter/examples/
- StreamProducer API: Search for "streamproducer"

## Success Metrics
- Producer/consumer pipelines connected
- Data flows between pipelines
- Dynamic connection/disconnection works
- Multiple consumers supported

**Confidence Score: 7/10**