# PRP-RTSP-29: Wire Retry Logic to Connection Attempts

## Overview
The retry system (auto mode and adaptive learning) is fully implemented but not actually recording connection results or influencing connection decisions. This PRP wires the retry logic into the actual connection flow.

## Current State
- RetryCalculator is created and used for delays
- Auto mode selector exists but never receives connection results
- Adaptive learning can't learn because it never gets data
- Connection racing strategy recommendations are ignored

## Success Criteria
- [ ] Connection attempts marked and recorded in retry system
- [ ] Auto mode receives connection results and adapts strategy
- [ ] Adaptive mode learns from actual connection patterns
- [ ] Racing strategy dynamically updated based on retry decisions
- [ ] Tests verify retry integration works end-to-end

## Technical Details

### Connection Points to Wire
1. In `imp.rs` connection logic around line 3436
2. Before `racer.connect()` - mark connection start
3. After success/failure - record result
4. Update racing config based on auto mode recommendations
5. Handle connection drops separately from initial failures

### Data Flow
```
Connection Attempt -> Mark Start -> Try Connect -> Record Result -> Update Strategy -> Apply to Next Attempt
```

## Implementation Blueprint
1. Add retry calculator to task state
2. Call `mark_connection_start()` before attempts
3. Call `record_connection_result()` after attempts
4. Query `get_racing_strategy()` and update racer config
5. Add connection drop detection (track how long connections stay alive)
6. Log strategy changes for debugging
7. Add property to query current auto mode state

## Resources
- GStreamer state management: https://gstreamer.freedesktop.org/documentation/plugin-development/basics/states.html
- Tokio select for connection racing: https://docs.rs/tokio/latest/tokio/macro.select.html
- Connection lifecycle patterns in existing code at `connection_pool.rs`

## Validation Gates
```bash
# Build and test
cargo test -p gst-plugin-rtsp retry -- --nocapture
cargo test -p gst-plugin-rtsp auto -- --nocapture

# Integration test with mock server
cargo test -p gst-plugin-rtsp integration::retry -- --nocapture

# Verify retry decisions are logged
GST_DEBUG=rtspsrc2:7 gst-launch-1.0 rtspsrc2 location=rtsp://test.local retry-strategy=auto 2>&1 | grep -i "strategy"
```

## Dependencies
- Existing retry.rs implementation
- Existing auto_selector.rs implementation
- Connection racer at connection_racer.rs

## Estimated Effort
3 hours

## Risk Assessment
- Medium complexity - threading connection state through async code
- Need to handle connection lifecycle properly
- Must not break existing retry behavior

## Success Confidence Score
7/10 - Clear integration points but async complexity