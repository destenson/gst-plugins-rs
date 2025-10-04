# RTSP Reconnection Timeout Analysis

## Problem Statement

During testing with 2% packet loss, the RTSP client was experiencing frequent disconnections every 5-10 seconds, requiring reconnection attempts that took ~5 seconds each. This resulted in significant buffer flow interruptions (0.0 fps for extended periods).

## Investigation

### Test Observations

From `recon-test.txt` analysis:

```
Line 60:  0:00:13.386 - Connection lost (attempt 1/5)
Line 83:  0:00:18.529 - Reconnection successful (5143ms elapsed)
Line 97:  0:00:19.772 - Connection lost again (attempt 2/5)
Line 121: 0:00:24.916 - Reconnection successful (5143ms elapsed)
Line 131: 0:00:26.692 - Connection lost again (attempt 3/5)
Line 156: 0:00:31.840 - Connection lost again (attempt 4/5)
Line 182: 0:00:36.957 - Connection lost again (attempt 5/5)
```

**Pattern**: Connections were dropping every 5-10 seconds despite only 2% packet loss.

### Frame Flow Analysis

```
Lines 46-57:  30 fps (stable)
Lines 65-75:  0.0 fps (reconnection #1)
Line 95:      34.0 fps (brief recovery)
Lines 103-123: 0.0 fps (reconnection #2)
Lines 137-147: 0.0 fps (reconnection #3)
Lines 161-171: 0.0 fps (reconnection #4)
Lines 187-253: 0.0 fps (extended outage - rapid reconnections)
Lines 260+:    30 fps (recovery)
```

## Root Cause: runOnDemand Stream Lifecycle

### Discovery

After increasing the read/write timeouts to 60s, disconnections **still occurred** every 5-10 seconds. Further investigation revealed the actual root cause:

**The mediamtx stream was configured with `runOnDemand`:**

```yaml
# mediamtx.yml (problematic)
test-h264:
  runOnDemand: ffmpeg -re -f lavfi -i testsrc...
  runOnDemandRestart: yes
  # Implicit: runOnDemandCloseAfter: 10s (from pathDefaults)
```

### Why This Caused Disconnections

1. **Client disconnects** (enters reconnection flow)
2. MediaMTX sees **no readers** for the stream
3. After `runOnDemandCloseAfter: 10s`, mediamtx **kills the ffmpeg process**
4. **But** the client reconnects within ~5 seconds!
5. Client reconnects → mediamtx keeps the TCP connection open
6. ffmpeg process might be:
   - Still shutting down
   - Restarting mid-stream
   - Generating packets with discontinuous sequence numbers
7. This causes **EOF** or **massive sequence number gaps**

### Evidence from Logs

```
Line 76:  Reconnection successful at 0:00:11.529
Line 80:  EOF at 0:00:13.206 (only 1.7 seconds later!)
Line 104: lost event for 2179 packet(s) (#1259->#3437)
```

**Pattern**: Reconnection succeeds, then immediately fails because the source stream is unstable.

### Secondary Issue: Jitter Buffer Sequence Gaps

After reconnection, the jitter buffer sees huge sequence number gaps:

```
lost event for 2179 packet(s) (#1259->#3437) for duration 0:00:04.943501722
```

This creates thousands of timers waiting for "missing" packets that will never arrive (the old ffmpeg instance generated them before being killed).

## Solution

### Primary Fix: Use Persistent Streams

Change from `runOnDemand` to `runOnInit` for test streams:

```yaml
# mediamtx.yml (fixed)
test-h264:
  runOnInit: ffmpeg -re -f lavfi -i testsrc...
  runOnInitRestart: yes
```

This ensures:
- ffmpeg starts when mediamtx starts
- Stream continues running even when no clients are connected
- No source restarts during client reconnection
- Sequence numbers remain continuous

### Alternative: Increase runOnDemandCloseAfter

If you must use `runOnDemand`, increase the close timeout:

```yaml
test-h264:
  runOnDemand: ffmpeg ...
  runOnDemandCloseAfter: 120s  # Give plenty of time for reconnection
```

But this is less reliable - if reconnection takes longer than expected, the source will still be killed.

## Solution

### Primary Fix: Use Persistent Streams

Change from `runOnDemand` to `runOnInit` for test streams:

```yaml
# mediamtx.yml (fixed)
test-h264:
  runOnInit: ffmpeg -re -f lavfi -i testsrc...
  runOnInitRestart: yes
```

This ensures:
- ffmpeg starts when mediamtx starts
- Stream continues running even when no clients are connected
- No source restarts during client reconnection
- Sequence numbers remain continuous

### Secondary Fix: Increase Read/Write Timeouts (Optional)

While not the root cause, increasing timeouts provides additional resilience:

```yaml
# mediamtx.yml
readTimeout: 60s   # Changed from 10s
writeTimeout: 60s  # Changed from 10s
```

### Alternative: Increase runOnDemandCloseAfter

If you must use `runOnDemand`, increase the close timeout:

```yaml
test-h264:
  runOnDemand: ffmpeg ...
  runOnDemandCloseAfter: 120s  # Give plenty of time for reconnection
```

But this is less reliable - if reconnection takes longer than expected, the source will still be killed.

## Testing Recommendations

### 1. Verify Fix

Run the same test with updated mediamtx.yml:

```bash
# Restart mediamtx with new configuration
pkill mediamtx
mediamtx mediamtx.yml &

# Run test with 2% packet loss
GST_DEBUG='rt*:4' ./net/rtsp/test_reconnection_cleanup.sh 2>&1 | tee recon-test-fixed.txt
```

Expected result: **No reconnections** (or very rare ones only on severe network outages)

### 2. Stress Test

Test with higher packet loss to verify reconnection still works when needed:

```bash
# Test with videotestsrc-bad (20% packet loss, 4s delay)
GST_DEBUG='rt*:4' ./net/rtsp/test_reconnection_cleanup.sh \
  rtsp://localhost:8554/videotestsrc-bad 60 0.25 2>&1 | tee recon-test-stress.txt
```

### 3. Monitor Metrics

Watch for:
- Connection duration (should be >> 60 seconds with 2% loss)
- Frame flow continuity (should maintain ~30 fps average)
- Reconnection attempts (should be minimal)
- Average FPS over time (should stay above 28 fps)

## Performance Impact

### Before Fix
- 5 reconnections in 42 seconds
- Extended periods of 0.0 fps
- Average FPS dropped from 30.2 to 8.8 fps
- User experience: Severe freezing and stuttering

### After Fix (Expected)
- 0-1 reconnections in 60+ seconds (only on severe outages)
- Minimal frame loss (jitter buffer handles packet loss)
- Average FPS stays near 30 fps
- User experience: Smooth playback with occasional minor artifacts

## Related Files

- `mediamtx.yml` - Server configuration (timeout settings)
- `net/rtsp/src/rtspsrc/imp.rs` - Client reconnection logic
- `net/rtsp/src/rtspsrc/session_manager.rs` - RTSP keep-alive management
- `net/rtsp/RECONNECTION_FLUSH_FIX.md` - Jitter buffer flush implementation
- `net/rtsp/test_reconnection_cleanup.sh` - Testing script

## Lessons Learned

1. **Server-side timeouts matter**: Even perfect client logic can't prevent disconnections if the server has aggressive timeouts
2. **TCP vs UDP trade-offs**: TCP reliability comes at the cost of timeout sensitivity
3. **Testing with packet loss**: Essential for discovering timeout issues that don't appear in ideal conditions
4. **Holistic debugging**: The issue wasn't in the code being modified (rtspsrc2), but in the external server configuration
5. **Default values**: MediaMTX's 10-second timeout is reasonable for low-latency LANs but too aggressive for networks with packet loss

## Future Improvements

1. **Configurable Timeouts**: Add properties to rtspsrc2 to negotiate or warn about server timeouts
2. **Timeout Detection**: Log warnings when approaching server timeout without data
3. **Proactive Keep-Alive**: Send occasional RTP-level keep-alive packets during quiet periods
4. **Adaptive Timeout**: Dynamically adjust based on network conditions
5. **Documentation**: Add troubleshooting guide for common server timeout issues
