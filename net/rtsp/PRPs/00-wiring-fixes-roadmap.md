# RTSP Wiring Fixes Roadmap

## Problem Statement
The RTSP codebase has extensive functionality implemented but not connected to the actual execution flow. Components exist in isolation without being invoked or integrated.

## Identified Wiring Gaps

### Critical (Blocking Functionality)
1. **Retry Logic Disconnected** - RetryCalculator exists but doesn't receive connection results
2. **HTTP Tunneling Unused** - Complete implementation never instantiated
3. **Auto Mode Decisions Ignored** - Computes strategies but doesn't apply them

### Important (Degraded Experience)
4. **No Telemetry for Retry** - Can't monitor or debug retry behavior
5. **Adaptive Learning Not Persisting** - Learns but loses knowledge on restart
6. **Static Racing Configuration** - Can't adapt to detected patterns

### Quality of Life
7. **No Debug Visibility** - Decisions happen silently
8. **No Integration Testing** - Can't verify wiring is correct

## Implementation Order

### Phase 1: Core Wiring (Week 1)
1. **PRP-29**: Wire Retry to Connections (3 hours)
   - Most critical - enables all retry functionality
   - Unblocks auto mode and adaptive learning
   
2. **PRP-33**: Dynamic Racing Strategy (2 hours)
   - Depends on PRP-29
   - Enables network adaptation

3. **PRP-34**: Debug Observability (2 hours)
   - Essential for debugging other PRPs
   - Low risk, high value

### Phase 2: Advanced Features (Week 2)
4. **PRP-30**: HTTP Tunneling (3 hours)
   - Standalone feature
   - Enables firewall traversal
   
5. **PRP-32**: Adaptive Persistence (2 hours)
   - Depends on PRP-29
   - Enables learning across sessions

6. **PRP-31**: Telemetry Integration (2 hours)
   - Depends on PRP-29
   - Enables monitoring

### Phase 3: Validation (Week 3)
7. **PRP-35**: Integration Tests (4 hours)
   - Validates all wiring
   - Prevents regressions

## Success Metrics
- All retry strategies affect actual connection behavior
- Auto mode switches strategies based on network patterns
- HTTP tunneling activates when needed
- Adaptive learning persists across sessions
- Debug logs show all decisions
- Integration tests pass

## Risk Mitigation
- Start with debug observability to make issues visible
- Test each PRP in isolation before integration
- Keep existing behavior as fallback
- Add feature flags for risky changes

## Testing Strategy
Each PRP includes specific validation gates. After all PRPs:
```bash
# Full integration test
cargo test --all-features --release

# Stress test with mock server
cargo test integration::stress -- --nocapture

# Real-world test
gst-launch-1.0 rtspsrc2 location=rtsp://wowzaec2demo.streamlock.net/vod/mp4:BigBuckBunny_115k.mp4 retry-strategy=auto ! decodebin ! autovideosink
```

## Estimated Total Effort
18 hours across 7 PRPs

## Confidence Score
Overall: 7.5/10
- Clear wiring points identified
- Existing code quality is good
- Main risk is async complexity