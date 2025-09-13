# PRP-RTSP-34: Add Debug Observability for Retry and Connection Decisions

## Overview
Add comprehensive debug logging and observability to make retry decisions, strategy changes, and connection patterns visible for debugging wiring issues.

## Current State
- Minimal logging of retry decisions
- No visibility into auto mode pattern detection
- Strategy changes happen silently
- Can't debug why certain decisions are made

## Success Criteria
- [ ] Debug logs show all retry decisions with reasons
- [ ] Pattern detection logic is visible
- [ ] Strategy changes logged with context
- [ ] New debug category for retry logic
- [ ] Property to query decision history
- [ ] Tests verify logging output

## Technical Details

### Debug Categories
- `rtspsrc2-retry` - Retry decisions and delays
- `rtspsrc2-auto` - Auto mode pattern detection
- `rtspsrc2-adaptive` - Learning and confidence scores
- `rtspsrc2-racing` - Connection racing decisions

### Key Decision Points to Log
1. Pattern detection (why detected as lossy/limited/stable)
2. Strategy selection (why chose this strategy)
3. Racing updates (why changed racing mode)
4. Retry delays (calculated delay and reason)
5. Learning updates (confidence changes)

## Implementation Blueprint
1. Add debug categories with LazyLock
2. Add structured logging at decision points
3. Create decision history buffer (last 20 decisions)
4. Add property to query decision history
5. Add trace spans for connection attempts
6. Create debug visualization tool
7. Add environment variable for verbose retry logging

## Resources
- GStreamer debug categories: https://gstreamer.freedesktop.org/documentation/gstreamer/gstinfo.html
- Structured logging with tracing: https://docs.rs/tracing/latest/tracing/
- Ring buffer for history: std::collections::VecDeque
- Debug visualization examples from other GStreamer elements

## Validation Gates
```bash
# Test debug output
GST_DEBUG=rtspsrc2-retry:7,rtspsrc2-auto:7 cargo test retry_debug -- --nocapture

# Verify all decision points logged
cargo test decision_logging -- --nocapture | grep -c "decision:"

# Check structured output
GST_DEBUG_FORMAT=json cargo test | jq '.message'
```

## Dependencies
- All retry-related modules
- GStreamer debug infrastructure
- Optional: tracing crate for structured logging

## Estimated Effort
2 hours

## Risk Assessment
- Very low risk - only adds logging
- No functional changes
- Minimal performance impact when disabled

## Success Confidence Score
9/10 - Straightforward logging additions