# PRP-RTSP-06: Connection Retry and Recovery Logic

## Overview
Implement robust connection retry logic with configurable backoff strategies (none, immediate, linear, exponential) for handling network interruptions and server failures gracefully.

## Current State
- Basic connection establishment exists
- No automatic retry on connection failure
- No exponential backoff implementation
- Connection drops cause permanent failure

## Success Criteria
- [ ] Automatic retry on connection failures
- [ ] Multiple backoff strategies (none, immediate, linear, exponential)
- [ ] Optional jitter for exponential backoff
- [ ] Configurable retry parameters
- [ ] Proper error state reporting
- [ ] Tests verify each retry strategy

## Technical Details

### Retry Strategy Components
1. Initial connection attempts
2. Reconnection after unexpected disconnect
3. Configurable backoff strategies
4. Optional jitter to prevent thundering herd
5. Max retry limit configuration

### Properties to Add
- retry-strategy (enum): auto, adaptive, none, immediate, linear, exponential, exponential-jitter (default: auto)
- max-reconnection-attempts (default: 5, -1 for infinite)
- reconnection-timeout (max backoff, default: 30s)
- initial-retry-delay (default: 1s)
- linear-retry-step (default: 2s, for linear strategy)

### Backoff Strategies
- **auto**: Simple heuristic-based automatic selection (see PRP-27)
- **adaptive**: Learning-based optimization over time (see PRP-28)
- **none**: No retry, fail immediately
- **immediate**: Retry immediately without delay
- **linear**: Fixed increment (1s, 3s, 5s, 7s...)
- **exponential**: Power of 2 (1s, 2s, 4s, 8s...)
- **exponential-jitter**: Exponential with random jitter ±25%

### State Machine Updates
- Add RECONNECTING state
- Track retry attempt count
- Emit signals on retry attempts
- Clear retry count on successful connection

## Implementation Blueprint
1. Add retry configuration properties including strategy enum
2. Create retry module with strategy pattern for backoff
3. Implement each backoff calculator (none, immediate, linear, exponential)
4. Add optional jitter for exponential strategies
5. Wrap connection logic in retry loop
6. Update state machine for reconnection
7. Emit messages on retry attempts with strategy info
8. Add connection-lost signal
9. Test each strategy with flaky mock server

## Resources
- Exponential backoff best practices: https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/
- tokio retry patterns: https://docs.rs/tokio-retry/
- Local ref: ~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c (search for "retry")

## Validation Gates
```bash
# Test each retry strategy
cargo test -p gst-plugin-rtsp retry_strategies -- --nocapture

# Test no-retry strategy
cargo test -p gst-plugin-rtsp retry_none -- --nocapture

# Test linear backoff
cargo test -p gst-plugin-rtsp retry_linear -- --nocapture

# Test with network interruption simulation
cargo test -p gst-plugin-rtsp connection_recovery -- --nocapture

# Verify backoff timing for each strategy
cargo test -p gst-plugin-rtsp backoff_timing -- --nocapture
```

## Dependencies
- None (can be tested with existing mock server)

## Estimated Effort
3 hours

## Risk Assessment
- Low risk - improves resilience without breaking changes
- Challenge: Testing time-based retry logic

## Success Confidence Score
8/10 - Common pattern with established best practices