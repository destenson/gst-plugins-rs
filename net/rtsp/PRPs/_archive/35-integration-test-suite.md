# PRP-RTSP-35: Create Integration Test Suite for Wired Components

## Overview
Create comprehensive integration tests that verify all wired components work together correctly, exposing any remaining wiring gaps.

## Current State
- Unit tests exist for individual components
- No end-to-end tests for retry with real connections
- Can't verify auto mode actually changes behavior
- No tests for HTTP tunneling or adaptive persistence

## Success Criteria
- [ ] Mock RTSP server with configurable behavior
- [ ] Tests verify retry strategies affect connections
- [ ] Auto mode pattern detection works end-to-end
- [ ] HTTP tunneling activates when needed
- [ ] Adaptive learning persists across restarts
- [ ] All tests pass in CI

## Technical Details

### Test Scenarios
1. **Connection-Limited Device**
   - Server drops connections after 20 seconds
   - Verify auto mode switches to last-wins racing
   
2. **Lossy Network**
   - 50% connection failure rate
   - Verify switches to first-wins racing
   
3. **HTTP Tunneling**
   - Block port 554, allow port 80
   - Verify tunneling activates automatically
   
4. **Adaptive Learning**
   - Run multiple sessions
   - Verify learns optimal strategy
   - Verify persistence across restarts

## Implementation Blueprint
1. Create mock RTSP server with behavior modes
2. Add network simulation (packet loss, delays)
3. Create test harness for element lifecycle
4. Write scenario-based integration tests
5. Add performance benchmarks
6. Create CI job for integration tests
7. Add flaky test detection and retry

## Resources
- Mock server example: tests/mock_server.rs
- tokio-test for async testing: https://docs.rs/tokio-test/
- Network simulation with netem
- GStreamer test utilities: gst-check
- CI integration: GitHub Actions examples

## Validation Gates
```bash
# Run all integration tests
cargo test -p gst-plugin-rtsp --features integration-tests integration -- --nocapture

# Run specific scenario
cargo test connection_limited_scenario -- --nocapture

# Run with network simulation
sudo cargo test --features network-sim lossy_network -- --nocapture

# Benchmark performance
cargo bench retry_performance
```

## Dependencies
- All retry and connection components
- Mock RTSP server infrastructure
- Test utilities from gst-check

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - async test coordination
- Network simulation requires privileges
- Tests may be flaky due to timing

## Success Confidence Score
7/10 - Complex but necessary for validation