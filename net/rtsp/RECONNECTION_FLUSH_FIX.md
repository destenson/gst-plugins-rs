# RTSP Reconnection Pipeline Flush Fix

## Problem

When the RTSP connection was lost and reconnected, buffers would stop flowing for extended periods (10+ seconds) even after the RTSP session was successfully re-established. 

### Root Cause Analysis

From captured logs (`recon-test.txt`), the issue was identified:

1. **Initial connection** fails at ~34 seconds
2. **Reconnection takes 5+ seconds** (5086ms) to re-establish RTSP session
3. **After reconnection success**, jitter buffer detects **1857 lost packets** (sequence #249->#2105)
4. **Jitter buffer creates lost-packet timers** for all missing packets, each with 200ms offset
5. **Buffer flow stops** for ~14 seconds while timers expire
6. **Problem repeats** on subsequent reconnections

The core issue: **The RTP pipeline state (jitter buffers, sequence numbers, SSRCs) was not being reset when reconnecting**, causing the new session to inherit stale state from the old session.

## Solution

Added pipeline flushing before reconnection attempts:

### 1. New `flush_rtp_pipeline()` Method

```rust
fn flush_rtp_pipeline(&self) {
    // Sends FLUSH_START and FLUSH_STOP events through all RTP/RTCP appsrc elements
    // This clears jitter buffer state, sequence numbers, and timing information
}
```

**Location**: `net/rtsp/src/rtspsrc/imp.rs` ~line 1075

**What it does**:
- Finds all `rtp_appsrc_*` and `rtcp_appsrc_*` elements in the bin
- Sends `FLUSH_START` event to each appsrc (clears buffers, stops flow)
- Sends `FLUSH_STOP` event to each appsrc (resets state, allows new data)

### 2. Call Flush Before Reconnection

```rust
// In the reconnection error handling loop (~line 3767):
gst::warning!(CAT, "Connection lost, attempting reconnection...");

// NEW: Flush the RTP pipeline to clear jitter buffer state
task_src.flush_rtp_pipeline();

// Post reconnection attempt message...
```

**Why this works**:
- GStreamer's FLUSH_START/STOP events propagate through the pipeline
- Jitter buffers receive flush events and clear their state:
  - Pending timers are cancelled
  - Sequence number tracking resets
  - SSRC mappings clear
  - Timing/synchronization state resets
- When new packets arrive after reconnection, they're treated as a fresh stream

## Benefits

1. **Fast reconnection recovery**: Buffers flow immediately after RTSP PLAY response
2. **No accumulated state**: Each reconnection starts with clean pipeline state
3. **SSRC change handling**: Works correctly when server changes SSRC (seen in logs: 3133120158 → 2772523032)
4. **Sequence number gaps**: No phantom "lost packet" timers from pre-disconnection state

## Testing

Test with simulated packet loss:

```bash
# Run with automatic reconnection
cargo run --example rtspsrc_cleanup -- \
  --url rtsp://your-server:8554/stream \
  --restart-interval 60

# Monitor frame flow during reconnections
# Look for:
# - "Flushing RTP pipeline before reconnection" message
# - Quick buffer flow after "Reconnection successful"
# - Consistent FPS after reconnection (not 0.0 fps for extended periods)
```

## Technical Details

### Flush Event Propagation

```
appsrc (FLUSH_START) → rtpbin → rtpssrcdemux → rtpjitterbuffer → rtpptdemux
                                      ↓
                                 Clear state
                                      ↓
appsrc (FLUSH_STOP)  → rtpbin → rtpssrcdemux → rtpjitterbuffer → rtpptdemux
```

### Why Not Full Pipeline Rebuild?

Alternative considered: Tear down and recreate entire RTP infrastructure on each reconnection.

**Rejected because**:
- More complex (requires pad management, ghost pad re-linking)
- Slower (element creation/configuration overhead)
- Unnecessary (flush events achieve the same goal)
- Higher risk of race conditions during rebuild

**Flush events are sufficient** because:
- GStreamer designed for this use case (live stream discontinuities)
- Lightweight operation (no allocations, just state reset)
- Thread-safe (GStreamer event handling is atomic)
- Standard practice for RTSP/RTP reconnection scenarios

## Related Changes

This fix complements earlier changes:
- **Pad unlink guards** (prevents unlinking already-disconnected pads)
- **Frame monitoring** (helps detect when buffers stop flowing)
- **Reconnection success timing** (posts message only after buffers flow)
- **Reconnect counter reset** (resets to 0 after successful reconnection)
- **Elapsed time tracking** (uses GStreamer clock for consistency)

## Future Improvements

Potential enhancements:
1. Make flush behavior configurable (property: `flush-on-reconnect`)
2. Add telemetry for flush operations
3. Expose jitter buffer statistics before/after flush
4. Add flush to other disconnect scenarios (TEARDOWN, EOS with reconnect flag)

## References

- GStreamer Flush Events: https://gstreamer.freedesktop.org/documentation/additional/design/events.html#flush-start
- RTP Jitter Buffer: https://gstreamer.freedesktop.org/documentation/rtpmanager/rtpjitterbuffer.html
- Original Issue: Buffers stop flowing after reconnection with packet loss
