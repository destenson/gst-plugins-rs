# PRP: Add Comprehensive Testing Infrastructure for rtspsrc2

## Problem Statement

rtspsrc2 currently has disabled integration tests and lacks comprehensive testing infrastructure to validate fixes and prevent regressions. The tests are disabled due to "GStreamer cannot be used with tokio runtime" issues.

## Context & Research

### Current Testing State
- **Integration tests disabled**: In `net/rtsp/tests/integration.rs`
- **Reason**: Async runtime conflicts with GStreamer
- **Coverage**: Minimal testing of core functionality
- **Validation**: Manual testing only

### Testing Requirements
After implementing the core fixes, need robust testing to ensure:
1. **Data Flow Validation**: Actual RTP data flows correctly
2. **State Transition Testing**: All state changes work properly  
3. **Error Handling Testing**: Graceful handling of various error conditions
4. **Performance Testing**: No memory leaks or performance regressions
5. **Regression Testing**: Prevent future breakage

## Implementation Plan

### Task 1: Fix Integration Test Runtime Issues
- **File**: `net/rtsp/tests/integration.rs`
- **Issue**: Resolve tokio runtime conflicts with GStreamer
- **Research**: How other elements in gst-plugins-rs handle async testing
- **Solution**: Proper runtime configuration or test isolation

### Task 2: Create Basic Data Flow Tests
- **Purpose**: Verify RTP data actually flows through pipeline
- **Method**: Create test pipelines with fakesink, verify buffer flow
- **Scenarios**: TCP transport, UDP transport, various codecs
- **Validation**: Count buffers, verify timing, check for data

### Task 3: Add State Transition Test Suite
- **Purpose**: Verify all state transitions work correctly
- **Coverage**: NULL→READY→PAUSED→PLAYING and reverse
- **Scenarios**: Normal operation, rapid changes, error conditions
- **Validation**: State changes complete, no hanging or crashes

### Task 4: Create Error Condition Test Suite
- **Purpose**: Verify robust error handling
- **Scenarios**: Invalid URLs, network errors, server disconnections
- **Validation**: Appropriate error reporting, clean recovery
- **Coverage**: Connection errors vs streaming errors

### Task 5: Add Performance and Memory Tests
- **Purpose**: Prevent memory leaks and performance regressions
- **Method**: Long-running tests with memory monitoring
- **Scenarios**: Extended streaming, rapid connect/disconnect cycles
- **Validation**: Bounded memory usage, consistent performance

### Task 6: Create Mock RTSP Server for Testing
- **Purpose**: Reliable test environment independent of external servers
- **Implementation**: Simple mock server that serves test streams
- **Benefits**: Consistent test conditions, no external dependencies
- **Features**: Various stream types, error injection capabilities

## Validation Gates

```bash
# Build and run all tests
cargo build -p gst-plugin-rtsp
cargo test -p gst-plugin-rtsp

# Run integration tests specifically
cargo test -p gst-plugin-rtsp --test integration

# Run performance tests
cargo test -p gst-plugin-rtsp --test performance -- --ignored

# Memory leak testing with valgrind (if available)
```

## Success Criteria

1. **Test Coverage**: >80% code coverage for critical paths
2. **Automated Validation**: All core functionality tested automatically
3. **Regression Prevention**: Tests catch future breakage
4. **Performance Baseline**: Establish performance and memory benchmarks
5. **CI Integration**: Tests run in continuous integration

## Dependencies

**Prerequisites**: Core functionality fixes should be implemented first:
1. Unlinked pad error handling
2. Ghost pad timing fixes  
3. Buffer management
4. State transition improvements

## References

- **Current Tests**: `net/rtsp/tests/integration.rs` disabled tests
- **Testing Patterns**: How other gst-plugins-rs elements implement tests
- **GStreamer Testing**: Documentation on testing GStreamer elements
- **Mock Servers**: Examples of test RTSP servers for validation

## Risk Assessment

**Low Risk**: Testing infrastructure doesn't affect runtime behavior, but proper async handling is needed.

## Estimated Effort

**5-6 hours**: Comprehensive testing setup including mock server and various test scenarios.

## Confidence Score: 8/10

High confidence - testing patterns are well established, main challenge is resolving the async runtime conflicts.