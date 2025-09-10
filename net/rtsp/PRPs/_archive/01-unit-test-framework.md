# PRP-RTSP-01: Unit Test Framework Setup for RTSP Plugin

## Overview
Establish a comprehensive unit testing framework for the net/rtsp plugin, following GStreamer Rust plugin testing patterns. This is foundational work that will enable test-driven development for all future improvements.

## Current State
- No tests exist in net/rtsp/tests/
- Other net plugins have established testing patterns (net/reqwest/tests/, net/quinn/tests/)
- The RTSP implementation has 2000+ lines of untested code

## Success Criteria
- [ ] Basic test harness created following GStreamer patterns
- [ ] At least 3 basic unit tests passing
- [ ] CI integration verified with cargo test
- [ ] Test coverage measurement enabled

## Technical Details

### Reference Patterns
Study these existing test implementations:
- net/reqwest/tests/reqwesthttpsrc.rs - HTTP source testing patterns with harness
- net/quinn/tests/quinnquic.rs - Network protocol testing
- net/rtp/tests/rtpbin2.rs - RTP-specific testing patterns

### Testing Infrastructure Components
1. Test initialization function with gst::init()
2. Plugin registration for tests
3. Basic element creation and property tests
4. State change testing
5. Mock server preparation (stub for next PRP)

### Key Testing Areas
- Element registration and instantiation
- Property getter/setter validation
- State transitions (NULL -> READY -> PAUSED -> PLAYING)
- Signal emission testing
- Basic pipeline construction

## Implementation Blueprint
1. Create net/rtsp/tests/ directory
2. Add test dependencies to Cargo.toml (gst-check, serial_test)
3. Create tests/rtspsrc.rs with init() function
4. Implement element creation tests
5. Add property validation tests
6. Add state change tests
7. Run cargo test and ensure all pass
8. Add #[cfg(test)] modules in source files where appropriate

## Resources
- GStreamer testing guide: https://gstreamer.freedesktop.org/documentation/tutorials/basic/debugging-tools.html
- Rust testing best practices: https://doc.rust-lang.org/book/ch11-00-testing.html
- gst-check documentation: https://gstreamer.freedesktop.org/documentation/check/

## Validation Gates
```bash
# Ensure tests compile and run
cargo test -p gst-plugin-rtsp --all-features

# Check test coverage (if grcov is available)
cargo tarpaulin -p gst-plugin-rtsp --out Html

# Verify no warnings
cargo clippy -p gst-plugin-rtsp --all-targets --all-features -- -D warnings
```

## Dependencies
- None (foundational PRP)

## Estimated Effort
3 hours

## Risk Assessment
- Low risk - purely additive, no changes to existing functionality
- Main challenge: Understanding GStreamer test patterns

## Success Confidence Score
8/10 - Well-established patterns exist in other plugins to follow