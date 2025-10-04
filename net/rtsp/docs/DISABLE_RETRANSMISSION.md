# Live Feed Configuration: Disabling Retransmissions

## Change Summary

**Modified**: `net/rtsp/src/rtspsrc/imp.rs`
```rust
const DEFAULT_DO_RETRANSMISSION: bool = false; // Changed from true
```

## Why This Matters for Live Feeds

### The Problem with Retransmissions

When `do-retransmission` is enabled (the old default):

1. Jitter buffer detects missing packets (sequence gaps)
2. Creates timers for each missing packet
3. Sends RTCP NACK requests asking server to resend them
4. Waits for retransmissions before releasing buffers
5. **Result**: Long delays and stalls, especially after reconnection

**Example from logs (with retransmission enabled)**:
```
lost event for 2179 packet(s) (#1259->#3437) for duration 0:00:04.943501722
```

This creates 2179 timers, causing **5+ seconds of delay** waiting for packets that will never arrive (they were lost before reconnection).

### The Solution for Live Feeds

When `do-retransmission` is disabled:

1. Jitter buffer detects missing packets
2. Immediately generates "lost packet" events
3. **Discards the missing packets** - doesn't wait
4. Continues with next available packet
5. **Result**: Smooth playback with occasional minor artifacts

## Impact on Different Scenarios

### Live Camera Feeds ✅ (Our Use Case)

**Characteristics**:
- Real-time streaming
- Latency-sensitive
- Missing frames acceptable (skip to live)
- Network may be unstable (radio links)

**Best Configuration**:
```rust
do-retransmission: false  // Don't wait for lost packets
drop-on-latency: false     // Allow jitter buffer to handle timing
latency: 2000              // 2 second buffer for network jitter
```

**Behavior**:
- Packet loss → small visual artifact → continues streaming
- Reconnection → immediate resume (no waiting for old packets)
- Lower latency
- Continuous playback

### VOD/Playback Scenarios ❌ (Not Our Use Case)

**Characteristics**:
- Pre-recorded content
- Not latency-sensitive
- Perfect quality desired
- Network usually stable

**Best Configuration**:
```rust
do-retransmission: true   // Request re-send of lost packets
drop-on-latency: false    // Wait as long as needed
latency: 5000+            // Large buffer for retransmissions
```

**Behavior**:
- Packet loss → pause → wait for retransmission → perfect playback
- Higher latency
- Potential stalls

## Technical Details

### RTCP NACK (Negative Acknowledgement)

With `do-retransmission: true`:
```
[Client] → [Server]: RTCP NACK packet #1234 missing
[Server] → [Client]: Retransmit RTP packet #1234
[Client]: Receives packet, fills gap, continues
```

**Problem**: If server can't retransmit (packet lost upstream, or publisher died), client waits forever (until timer expires).

With `do-retransmission: false`:
```
[Client]: Packet #1234 missing → Log it → Discard → Continue
```

**Benefit**: No waiting, no RTCP overhead, immediate continuation.

### Sequence Number Gaps After Reconnection

**Scenario**: Publisher disconnects and reconnects

**Before (with retransmission)**:
```
Last packet before disconnect: #1000
First packet after reconnect:  #5000

Jitter buffer behavior:
- Creates timers for packets #1001-#4999 (3999 timers!)
- Sends RTCP NACK for all missing packets
- Waits for retransmissions (server ignores, packets don't exist)
- Eventually times out after several seconds
- Finally continues
```

**After (without retransmission)**:
```
Last packet before disconnect: #1000
First packet after reconnect:  #5000

Jitter buffer behavior:
- Detects gap, logs lost event for #1001-#4999
- Immediately discards missing range
- Continues with #5000
- Instant resume!
```

## Configuration Matrix

| Use Case | do-retransmission | latency | drop-on-latency | Notes |
|----------|-------------------|---------|-----------------|-------|
| **Live Camera** (stable network) | false | 1000-2000ms | false | Low latency, tolerates minor loss |
| **Live Camera** (unstable/radio) | false | 2000-3000ms | false | Higher buffer for jitter, no retrans |
| **VOD/Playback** (stable) | true | 5000ms+ | false | Perfect quality, can wait |
| **Low Latency Gaming** | false | 500-1000ms | true | Ultra-low latency, drop old packets |

## Testing the Change

### Before: With Retransmission Enabled

```bash
# Old behavior - you saw this in your tests
grep "lost event" recon-test.txt
# Output: lost event for 2179 packet(s) (#1259->#3437) for duration 0:00:04.943501722
```

**Frame flow**:
```
0:00:11 - Reconnection successful
0:00:11-0:00:16 - Jitter buffer waiting for retransmissions (0.0 fps)
0:00:16 - Timers expire, playback resumes
```

### After: With Retransmission Disabled

```bash
# Rebuild with new default
cd net/rtsp
cargo build -p gst-plugin-rtsp --example rtspsrc_cleanup

# Run test
GST_DEBUG='rt*:4' ./test_reconnection_cleanup.sh 2>&1 | tee recon-test-no-retrans.txt

# Check for lost packets
grep "lost event" recon-test-no-retrans.txt
```

**Expected frame flow**:
```
0:00:11 - Reconnection successful
0:00:11 - Lost packets discarded immediately
0:00:11 - Playback resumes (30 fps)
```

**Expected**: Frames resume flowing **immediately** after reconnection, no multi-second delays!

## When to Override the Default

If you need retransmissions for a specific use case, you can still enable it:

```rust
// In your application
rtspsrc2.set_property("do-retransmission", true);
```

Or in gst-launch:
```bash
gst-launch-1.0 rtspsrc2 location=rtsp://... do-retransmission=true ! ...
```

## Related Changes

This works in combination with:

1. **Pipeline Flush** (already implemented)
   - Clears jitter buffer state before reconnection
   - Prevents accumulation of stale timers

2. **Fast Reconnection** (already implemented)
   - 3-second backoff
   - 5 retry attempts
   - Frame monitoring

3. **MediaMTX Configuration** (updated)
   - 10s timeouts (fast failure detection)
   - 2048 packet write queue (absorbs transient loss)
   - Auto-restart publisher

Together, these create a **resilient live streaming system** that handles:
- ✅ Network packet loss
- ✅ Publisher disconnections
- ✅ Radio link instability  
- ✅ Fast recovery
- ✅ Continuous playback

## Performance Impact

### CPU/Network

- **CPU**: Slightly lower (no RTCP NACK processing)
- **Network**: Lower bandwidth (no RTCP NACK packets)
- **Jitter Buffer**: Lower memory (no retransmission timers)

### Latency

- **Before**: 2000ms (buffer) + 0-5000ms (retransmission waits)
- **After**: 2000ms (buffer) only

### Quality

- **Before**: Perfect quality (if retransmissions work), long stalls (if they don't)
- **After**: Occasional minor artifacts, continuous playback

For **live camera feeds**, the trade-off is clear: slight quality reduction for much better reliability and lower latency!

## References

- GStreamer rtpbin documentation: `do-retransmission` property
- RFC 4585: Extended RTP Profile for Real-time Transport Control Protocol (RTCP)-Based Feedback (NACK)
- Your test results: 2179 lost packet timers causing 5+ second delays

## Conclusion

Changing `DEFAULT_DO_RETRANSMISSION` from `true` to `false` is the **correct default for live streaming use cases**, which is what rtspsrc2 is primarily designed for. This eliminates the massive delays you were seeing after reconnection!
