# PRP-RTSP-33: Wire Dynamic Connection Racing Strategy Updates

## Overview
Auto mode can recommend connection racing strategies (first-wins, last-wins) based on network patterns, but these recommendations are never applied to the ConnectionRacer configuration.

## Current State
- Auto selector has `get_racing_strategy()` method
- ConnectionRacer is created once with static config
- No mechanism to update racing strategy dynamically
- Recommendations are computed but ignored

## Success Criteria
- [ ] Racing strategy updates based on auto mode detection
- [ ] Smooth transitions between racing strategies
- [ ] Property reflects current racing strategy
- [ ] Logs show strategy changes with reasons
- [ ] Tests verify dynamic strategy switching

## Technical Details

### Update Flow
1. Auto mode detects pattern (connection-limited, lossy, stable)
2. Recommends racing strategy change
3. Update ConnectionRacer config
4. Apply to next connection attempt
5. Log the change with reason

### Strategy Mapping
- ConnectionLimited → LastWins (replace old connections)
- HighPacketLoss → FirstWins (parallel attempts)
- Stable → None (single connection)
- Unknown → None (conservative default)

## Implementation Blueprint
1. Make ConnectionRacer config mutable
2. Add update method to ConnectionRacer
3. Query auto selector after pattern detection
4. Apply recommended strategy before next attempt
5. Add property for current effective strategy
6. Emit signal on strategy change
7. Add integration test with pattern simulation

## Resources
- Connection racing patterns: Look at Happy Eyeballs RFC 8305
- Tokio select! for racing: https://docs.rs/tokio/latest/tokio/macro.select.html
- Dynamic reconfiguration patterns in Rust
- GStreamer property notifications

## Validation Gates
```bash
# Test strategy updates
cargo test -p gst-plugin-rtsp racing_strategy_update -- --nocapture

# Verify with debug logs
GST_DEBUG=rtspsrc2:7 gst-launch-1.0 rtspsrc2 location=rtsp://test.local retry-strategy=auto 2>&1 | grep -i "racing"

# Test transitions
cargo test racing_transitions -- --nocapture
```

## Dependencies
- auto_selector.rs with get_racing_strategy()
- connection_racer.rs
- Retry integration from PRP-29

## Estimated Effort
2 hours

## Risk Assessment
- Low risk - additive change to existing racing
- Must handle strategy changes mid-connection gracefully
- Thread safety for config updates

## Success Confidence Score
8/10 - Clear integration path with existing components