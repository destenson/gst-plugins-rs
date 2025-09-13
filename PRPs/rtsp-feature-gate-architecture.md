# PRP: Feature-Gate Architecture for Tokio Migration

## Overview
Establish a clean feature-gate system that allows the codebase to run either with Tokio (legacy mode) or pure GStreamer/GIO, enabling gradual migration and A/B testing.

## Background
To ensure stability during migration, we need to maintain both implementations side-by-side. The Tokio implementation will be behind a "tokio-runtime" feature flag, with the default being pure GStreamer/GIO.

## Requirements
- Create "tokio-runtime" feature flag (opt-in, not default)
- Abstract connection and async interfaces
- Ensure clean separation with no Tokio imports when feature is disabled
- Support runtime selection without recompilation for testing
- Maintain identical external API regardless of backend

## Technical Context
Feature gate structure:
- Default: Pure GStreamer/GIO implementation
- With tokio-runtime: Current Tokio-based implementation
- Cargo.toml feature configuration
- Conditional compilation with #[cfg(feature = "tokio-runtime")]

Key abstraction points:
- Connection management trait
- Async runtime abstraction
- Socket operations interface
- Timer/timeout abstraction
- Task spawning interface

## Implementation Tasks
1. Add tokio-runtime feature to Cargo.toml (non-default)
2. Create abstraction traits for runtime operations
3. Implement GIO backend for abstractions
4. Implement Tokio backend behind feature gate
5. Create runtime factory based on feature/config
6. Move Tokio imports behind cfg(feature) blocks
7. Update build scripts for feature combinations
8. Add runtime selection property (for testing)
9. Create compatibility shims for smooth transition

## Testing Approach
- Build tests for both feature configurations
- A/B comparison tests between implementations
- Performance benchmarks for both backends
- Feature flag combination testing

## Validation Gates
```bash
# Build without Tokio (default)
cargo build --package gst-plugin-rtsp
cargo test --package gst-plugin-rtsp

# Build with Tokio feature
cargo build --package gst-plugin-rtsp --features tokio-runtime
cargo test --package gst-plugin-rtsp --features tokio-runtime

# Ensure no Tokio deps without feature
cargo tree --package gst-plugin-rtsp | grep -v tokio

# Test both implementations
cargo test --package gst-plugin-rtsp --all-features
```

## Success Metrics
- Clean compilation without Tokio dependencies by default
- Both implementations pass identical test suite
- No performance regression in either mode
- Clear separation of implementation code

## Dependencies
- No external dependencies for abstraction layer
- Conditional dependencies in Cargo.toml

## Risk Mitigation
- Start with Tokio as default, switch after validation
- Comprehensive abstraction trait design
- Parallel testing infrastructure
- Clear documentation of feature flags

## References
- Cargo feature flags: https://doc.rust-lang.org/cargo/reference/features.html
- Similar pattern in gstreamer-rs optional features
- Current Tokio usage throughout net/rtsp/src/rtspsrc/

## Confidence Score: 9/10
Well-established pattern in Rust. Main complexity is designing clean abstractions.