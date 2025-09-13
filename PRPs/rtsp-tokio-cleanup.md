# PRP: Complete Tokio Dependency Removal

## Overview
Final cleanup phase to completely remove Tokio dependencies when the tokio-runtime feature is not enabled, ensuring a pure GStreamer/GIO implementation.

## Background
After implementing GIO-based alternatives for all async operations, we need to clean up the codebase to remove all Tokio imports, dependencies, and related code when running in pure GStreamer mode.

## Requirements
- Remove all Tokio imports behind feature gates
- Clean up Cargo.toml dependencies
- Remove Tokio-specific error types and conversions
- Eliminate futures/async-trait when not needed
- Ensure zero Tokio code in default build

## Technical Context
Items to remove/gate:
- tokio crate dependency (make optional)
- futures crate (if only used with Tokio)
- async-trait (replace with GIO callbacks)
- tokio-util for codecs
- Any remaining async/await syntax
- Tokio-specific error conversions

Files with heavy Tokio usage:
- imp.rs - Runtime and task spawning
- tcp_message.rs - Tokio codecs
- connection_pool.rs - Async connection management
- All test files using Tokio runtime

## Implementation Tasks
1. Audit all Cargo.toml dependencies
2. Make tokio, futures, async-trait optional
3. Add #[cfg(feature = "tokio-runtime")] to all Tokio imports
4. Remove/replace Tokio error type conversions
5. Clean up async fn signatures when not using Tokio
6. Update test infrastructure for both modes
7. Remove Tokio macros usage (#[tokio::test])
8. Clean up Tokio-specific utility functions
9. Update documentation to reflect dual-mode
10. Create migration guide for downstream users

## Testing Approach
- Verify clean build without Tokio
- Check no Tokio symbols in binary
- Run full test suite in both modes
- Benchmark both implementations

## Validation Gates
```bash
# Ensure no Tokio in default build
cargo clean
cargo build --package gst-plugin-rtsp
cargo tree --package gst-plugin-rtsp | grep -E "tokio|futures|async-trait" && exit 1

# Verify both configurations work
cargo test --package gst-plugin-rtsp
cargo test --package gst-plugin-rtsp --features tokio-runtime

# Check binary size reduction
ls -la target/release/*.so

# Ensure no async/await in non-Tokio build
grep -r "async fn\|\.await" src/ --include="*.rs" | grep -v "cfg.*tokio"
```

## Success Metrics
- Zero Tokio dependencies in default build
- Binary size reduction of 20-30%
- Reduced compile time without Tokio
- Both modes pass identical test suites

## Dependencies
- Completion of all GIO implementation PRPs
- Feature gate architecture in place

## Risk Mitigation
- Incremental removal with testing at each step
- Maintain compatibility layer temporarily
- Clear compile-time errors for missing implementations
- Comprehensive CI testing for both modes

## References
- Cargo optional dependencies: https://doc.rust-lang.org/cargo/reference/features.html#optional-dependencies
- Similar cleanup in other gst-plugins-rs crates
- Current Tokio usage analysis from previous PRPs

## Confidence Score: 9/10
Mechanical cleanup with clear success criteria. Main effort is thoroughness in finding all dependencies.