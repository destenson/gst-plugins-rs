# Stream Synchronization Example Summary

## What Was Created

I've added a **synchronized dual-stream example** (`rtspsrc_synced_dual_stream.rs`) that keeps frames from multiple RTSP sources time-aligned, similar to how NVIDIA DeepStream's `nvstreammux` works.

## Key Features

### Frame Synchronization
- Uses GStreamer's **compositor** element as an aggregator/muxer
- Waits for frames from ALL input streams before outputting
- Frames are time-aligned and synchronized
- Side-by-side layout (1280x480) showing both streams

### Pipeline Architecture

```
Stream 1 Source:
  rtspsrc2 → decodebin → videoconvert → videoscale → capsfilter → queue
                                                                     ↓
                                                            ┌────────┴──────┐
                                                            │               │
Stream 2 Source:                                            │  compositor   │
  rtspsrc2 → decodebin → videoconvert → videoscale → queue─┤   (muxer)     │
                                                            │               │
                                                            └───────┬───────┘
                                                                    │
                                                       queue → videoconvert → fakesink
```

### How It Works

1. **Normalization**: Each stream is converted to consistent format (I420, 640x480)
2. **Synchronization**: Compositor waits for buffers from both streams
3. **Time Alignment**: Uses PTS (presentation timestamps) to align frames
4. **Output**: Single synchronized stream with frames from all inputs

## Testing

```bash
# Setup (one-time)
./net/rtsp/test_dual_stream.sh setup-ip
./net/rtsp/test_dual_stream.sh start-mediamtx          # Terminal 1
./net/rtsp/test_dual_stream.sh publish-test-streams    # Terminal 2

# Run synchronized example
./net/rtsp/test_dual_stream.sh test-synced             # Terminal 3

# Test what happens when one stream fails
./net/rtsp/test_dual_stream.sh block-stream2
# Compositor output PAUSES (waiting for Stream 2)
# Stream 1 keeps receiving but compositor won't output

./net/rtsp/test_dual_stream.sh unblock-stream2
# Compositor resumes with synchronized frames
```

## Comparison: Independent vs Synchronized

| Aspect | Independent Mode | Synchronized Mode |
|--------|------------------|-------------------|
| **File** | `rtspsrc_cleanup_dual_stream.rs` | `rtspsrc_synced_dual_stream.rs` |
| **Streams** | Separate sinks | Single muxed sink |
| **Sync** | No synchronization | Frames aligned |
| **Resilience** | One failure doesn't affect other | One failure pauses output |
| **Use Case** | Multi-camera monitoring | Multi-camera inference |
| **Output FPS** | Independent | Matches slowest stream |

## Similar to nvstreammux

This is conceptually similar to NVIDIA DeepStream's `nvstreammux`:

**Similarities:**
- ✅ Aggregates multiple video streams
- ✅ Keeps frames synchronized
- ✅ Waits for all inputs
- ✅ Normalizes format/resolution
- ✅ Single batched output

**Differences:**
- ❌ CPU-based (nvstreammux uses GPU)
- ❌ No hardware batching (nvstreammux optimized for inference)
- ❌ No CUDA memory (nvstreammux does zero-copy on GPU)

**But:** It works on any platform and demonstrates the same synchronization behavior!

## Next Steps for Production

To make this production-ready for inference pipelines:

1. **Replace fakesink** with inference element (onnx, tensorflow, etc.)
2. **Add batch processing** - Modify compositor to output batches
3. **GPU acceleration** - Use hardware video decode/scale if available
4. **Adaptive sync** - Drop frames when streams desync beyond threshold
5. **Dynamic streams** - Add ability to add/remove streams while running
6. **Reconnection logic** - Restart failing streams without stopping compositor

## Why This Matters

For multi-camera inference scenarios (like object detection across multiple feeds):
- Synchronized frames ensure temporal consistency
- Batch processing improves inference throughput
- Time alignment prevents processing mismatched frames
- Like nvstreammux but pure GStreamer (portable across platforms)

The synchronized mode is ideal when you need:
- Multiple cameras feeding into single AI model
- Frame-by-frame temporal analysis across streams
- Coordinated processing of multiple video sources
- Batch inference on synchronized frames
