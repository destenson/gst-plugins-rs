# PRP-RTSP-27: Simple Auto Retry Mode

## Overview
Implement a simple "auto" retry mode that uses basic heuristics to quickly select an appropriate retry strategy without requiring user configuration or learning.

## Current State
- Users must manually select retry strategy
- No automatic fallback between strategies
- Requires network knowledge to configure
- One-size-fits-all defaults don't work everywhere

## Success Criteria
- [ ] Automatic strategy selection based on simple rules
- [ ] Quick decisions (within 3 attempts)
- [ ] No configuration required
- [ ] Handles common network patterns
- [ ] Tests verify auto selection logic

## Technical Details

### Design Principles
- **Keep It Simple**: Use proven heuristics
- **Fast Decisions**: Don't make users wait
- **No State**: Stateless between sessions
- **Good Enough**: 80% optimal is fine

### Decision Logic

```
START
  ↓
Use exponential-jitter (safe default for most networks)
  ↓
Monitor first 3 connection attempts
  ↓
Decision Rules:
- IF connection succeeds but drops < 30 seconds
  → Switch to last-wins (device dropping old connections)
  
- ELSE IF > 50% connection failures
  → Switch to first-wins (network packet loss)
  
- ELSE IF current strategy working
  → Keep current strategy
  
- ELSE IF total failure
  → Try next in fallback list
```

### Fallback Strategy List
```rust
const FALLBACK_STRATEGIES: [Strategy; 4] = [
    Strategy::ExponentialJitter,  // Good default
    Strategy::FirstWins,          // For packet loss
    Strategy::LastWins,           // For connection limits
    Strategy::Linear,             // Conservative fallback
];
```

### Detection Heuristics

1. **Connection-Limited Device Detection**
   - Connection succeeds initially
   - Drops within 30 seconds
   - Happens repeatedly
   - → Use last-wins strategy

2. **Packet Loss Detection**
   - High failure rate (>50%)
   - Timeouts common
   - Some attempts succeed
   - → Use first-wins racing

3. **Stable Network Detection**
   - Consistent success
   - Rare failures
   - Quick recovery
   - → Keep exponential-jitter

### Properties
```rust
retry-strategy: auto | manual strategies...
(default: auto)

# Auto mode specific
auto-detection-attempts: 3 (how many attempts before deciding)
auto-fallback-enabled: true (try other strategies on failure)
```

## Implementation Blueprint
1. Add "auto" to retry strategy enum
2. Create auto_selector module
3. Implement detection heuristics
4. Add fallback list iterator
5. Track attempt outcomes (last 3)
6. Implement strategy switching
7. Add logging for decisions
8. Test with network scenarios

## Example Behavior

### Scenario: Normal Network
```
Attempt 1: exponential-jitter → Success
Attempt 2: exponential-jitter → Success
Decision: Keep exponential-jitter
```

### Scenario: IP Camera
```
Attempt 1: exponential-jitter → Success (drops at 15s)
Attempt 2: exponential-jitter → Success (drops at 20s)
Attempt 3: exponential-jitter → Success (drops at 18s)
Decision: Switch to last-wins
```

### Scenario: Lossy WiFi
```
Attempt 1: exponential-jitter → Timeout
Attempt 2: exponential-jitter → Timeout
Attempt 3: exponential-jitter → Success
Decision: Switch to first-wins racing
```

## Resources
- Common network patterns documentation
- GStreamer rtspsrc retry behavior
- Industry best practices for retry logic

## Validation Gates
```bash
# Test auto detection logic
cargo test -p gst-plugin-rtsp auto_detection -- --nocapture

# Test strategy switching
cargo test -p gst-plugin-rtsp auto_switching -- --nocapture

# Test fallback list
cargo test -p gst-plugin-rtsp auto_fallback -- --nocapture

# Test with various networks
cargo test -p gst-plugin-rtsp auto_scenarios -- --nocapture
```

## Dependencies
- PRP-RTSP-06 (Retry strategies) - uses all strategies
- PRP-RTSP-17 (Connection racing) - for first/last-wins

## Estimated Effort
2 hours

## Risk Assessment
- Low risk - simple rule-based system
- Challenge: Tuning detection thresholds
- Benefit: Zero configuration for most users

## Success Confidence Score
9/10 - Simple heuristics work well in practice

## Why This Approach Works

1. **Most networks fall into common patterns**
   - Home/office: stable, exponential-jitter works
   - Mobile: packet loss, first-wins helps
   - IP cameras: connection limits, last-wins needed

2. **Quick decisions prevent user frustration**
   - 3 attempts = ~5-10 seconds max
   - Better to be fast than perfect

3. **Fallback list covers edge cases**
   - If detection fails, try other strategies
   - Eventually something will work

4. **No state means predictable behavior**
   - Same network always gets same treatment
   - Easy to debug and understand