# PRP-RTSP-32: Wire Adaptive Learning Persistence

## Overview
Adaptive retry has persistence code but it's only triggered on individual attempts. Need to wire proper lifecycle management for loading and saving learned patterns.

## Current State
- Persistence methods exist in adaptive_retry.rs
- Cache directory logic implemented
- Loading only happens on creation
- Saving only on individual attempts (may not persist on shutdown)
- No cleanup of old cache files

## Success Criteria
- [ ] Load cached patterns on element creation
- [ ] Save patterns on element destruction
- [ ] Periodic saves during long sessions
- [ ] Cleanup old cache entries (>7 days)
- [ ] Handle cache corruption gracefully
- [ ] Tests verify persistence across sessions

## Technical Details

### Lifecycle Integration
1. On element start → load cache
2. Every N attempts → save checkpoint
3. On element stop → final save
4. On crash → rely on periodic saves
5. Background task for cache cleanup

### Cache Management
- Location: `~/.cache/gstreamer/rtspsrc2/`
- Format: JSON with server hash as filename
- TTL: 7 days (configurable)
- Size limit: 100 servers max

## Implementation Blueprint
1. Add cache loading to element initialization
2. Implement periodic save timer (every 100 attempts)
3. Add cleanup task on startup
4. Handle element shutdown properly
5. Add cache statistics property
6. Implement cache export/import commands
7. Add tests with temp directories

## Resources
- dirs crate for cache paths: https://docs.rs/dirs/latest/dirs/
- serde_json for persistence: https://docs.rs/serde_json/latest/serde_json/
- GStreamer element lifecycle: https://gstreamer.freedesktop.org/documentation/plugin-development/basics/states.html
- tokio intervals for periodic saves: https://docs.rs/tokio/latest/tokio/time/struct.Interval.html

## Validation Gates
```bash
# Test persistence
cargo test -p gst-plugin-rtsp --features adaptive persistence -- --nocapture

# Verify cache files created
ls ~/.cache/gstreamer/rtspsrc2/*.json

# Test cache loading
cargo test adaptive_cache_load -- --nocapture

# Verify old files cleaned up
cargo test cache_cleanup -- --nocapture
```

## Dependencies
- adaptive_retry.rs with AdaptiveRetryManager
- Feature flag: adaptive
- serde and serde_json dependencies

## Estimated Effort
2 hours

## Risk Assessment
- Medium - filesystem operations can fail
- Need proper error handling for corrupt cache
- Must handle concurrent access to cache files

## Success Confidence Score
7/10 - Clear requirements but filesystem complexity