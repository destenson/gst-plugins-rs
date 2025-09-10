# PRP-RTSP-17: Parallel Connection Racing (Happy Eyeballs)

## Overview
Implement parallel connection racing strategies including "first-wins" (happy eyeballs) and "last-wins" (for connection-limited devices). This significantly improves connection reliability for both intermittent packet loss and devices that drop older connections.

## Current State
- Sequential connection attempts only
- Long timeout before retry
- Single connection path
- Poor experience with flaky networks

## Success Criteria
- [ ] Launch multiple parallel connections
- [ ] Use first successful connection
- [ ] Cancel pending attempts cleanly
- [ ] Configurable racing parameters
- [ ] Tests verify racing behavior

## Technical Details

### Racing Strategies

#### First-Wins (Happy Eyeballs)
1. Launch connections with staggered delays
2. Use first successful connection
3. Cancel all pending attempts
4. Best for: intermittent packet loss

#### Last-Wins (Connection Limited)
1. On connection drop, try multiple new connections
2. Use the last successful connection (newest)
3. Helps when devices drop older connections for new ones
4. Best for: devices with strict connection limits

### Configuration Properties
- connection-racing (enum): none, first-wins, last-wins, hybrid (default: none)
- max-parallel-connections (default: 3)
- racing-delay-ms (default: 250ms between starts)
- racing-timeout (default: 5s for whole race)

### Implementation Considerations
- Use tokio::select! for racing futures
- Track all connection attempts
- Clean cancellation of losers
- Avoid resource leaks
- Log racing outcomes for debugging

### Use Cases
- **first-wins**: Networks with intermittent packet loss
- **last-wins**: IP cameras with connection limits
- **last-wins**: Devices that drop older RTSP sessions
- **hybrid**: Try first-wins, fallback to last-wins
- Dual-stack IPv4/IPv6 (future)
- Multiple server endpoints

## Implementation Blueprint
1. Add racing configuration properties with strategy enum
2. Create connection_racer module
3. Implement first-wins strategy with tokio::select!
4. Implement last-wins strategy with connection replacement
5. Add hybrid strategy detection logic
6. Track all connections and handle replacements
7. Ensure proper cleanup of replaced connections
8. Integrate with retry logic
9. Add comprehensive tests for each strategy

## Resources
- RFC 8305 (Happy Eyeballs v2): https://datatracker.ietf.org/doc/html/rfc8305
- tokio select macro: https://docs.rs/tokio/latest/tokio/macro.select.html
- Similar pattern in reqwest: https://github.com/seanmonstar/reqwest/issues/1422

## Validation Gates
```bash
# Test first-wins strategy
cargo test -p gst-plugin-rtsp racing_first_wins -- --nocapture

# Test last-wins strategy
cargo test -p gst-plugin-rtsp racing_last_wins -- --nocapture

# Test connection replacement
cargo test -p gst-plugin-rtsp racing_replacement -- --nocapture

# Test with packet loss simulation
cargo test -p gst-plugin-rtsp racing_packet_loss -- --nocapture

# Verify resource cleanup
cargo test -p gst-plugin-rtsp racing_cleanup -- --nocapture
```

## Dependencies
- PRP-RTSP-06 (Retry logic) - complements retry strategies

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - concurrent connection management
- Challenge: Proper resource cleanup
- Benefit: Major UX improvement for unreliable networks

## Success Confidence Score
7/10 - Well-known pattern but requires careful async handling