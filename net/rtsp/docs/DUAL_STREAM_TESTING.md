# Dual-Stream RTSP Testing

This directory contains tools for testing multiple independent RTSP streams in the same GStreamer pipeline, with both independent and synchronized (muxed) modes.

## Files

- `rtspsrc_cleanup_dual_stream.rs` - Independent streams (each with own sink)
- `rtspsrc_synced_dual_stream.rs` - Synchronized streams using compositor (like nvstreammux)
- `mediamtx-dual.yml` - MediaMTX configuration for hosting test streams
- `test_dual_stream.sh` - Helper script for setup and testing

## Two Approaches

### 1. Independent Streams (`test-dual`)

Each stream runs completely independently with its own sink:
- Separate processing chains
- Independent frame rates
- One stream can fail without affecting the other
- **Use case:** Multi-camera monitoring, different frame rates OK

### 2. Synchronized Streams (`test-synced`)

Streams are muxed together using compositor to keep frames synchronized:
- Frames are time-aligned (similar to nvstreammux behavior)
- Single output with frames from all streams in sync
- If one stream stalls, output waits for it
- **Use case:** Multi-camera inference, frame sync required

## Quick Start

### 1. Setup the loopback IP (one-time setup)

```bash
./net/rtsp/test_dual_stream.sh setup-ip
```

This adds `127.0.0.2` as a loopback alias so you can have two "different" servers for testing.

### 2. Start MediaMTX (Terminal 1)

```bash
./net/rtsp/test_dual_stream.sh start-mediamtx
```

### 3. Publish test streams (Terminal 2)

```bash
./net/rtsp/test_dual_stream.sh publish-test-streams
```

This uses ffmpeg to create two different test patterns and publish them to the RTSP server.

### 4a. Run independent streams (Terminal 3)

```bash
./net/rtsp/test_dual_stream.sh test-dual
```

You should see both streams receiving frames independently:

```
[Stats] Stream 1: 60 frames (30.0 fps) | Stream 2: 58 frames (29.0 fps)
[Stats] Stream 1: 120 frames (30.0 fps) | Stream 2: 118 frames (29.5 fps)
```

### 4b. OR run synchronized streams (Terminal 3)

```bash
./net/rtsp/test_dual_stream.sh test-synced
```

You should see synchronized output:

```
[Stats] Stream 1: 60 frames (30.0 fps) | Stream 2: 60 frames (30.0 fps) | Synced Output: 60 frames (30.0 fps)
```

Note: The compositor waits for frames from BOTH streams, so they stay in sync!

## Testing Stream Independence vs Synchronization

### Independent Mode (`test-dual`)

To verify that one stream can fail/reconnect without affecting the other:

```bash
./net/rtsp/test_dual_stream.sh block-stream2
```

**Expected behavior:**
- Stream 2 stops receiving frames
- **Stream 1 continues unaffected** ✅
- Both have independent output

### Synchronized Mode (`test-synced`)

```bash
./net/rtsp/test_dual_stream.sh block-stream2
```

**Expected behavior:**
- Stream 2 stops receiving frames
- **Compositor output pauses** (waiting for Stream 2) ⏸️
- Stream 1 keeps receiving frames but compositor won't output them
- When Stream 2 recovers, compositor resumes with synchronized frames

This demonstrates the tradeoff:
- **Independent mode**: Resilient, streams don't affect each other
- **Synchronized mode**: Frames stay aligned, but failure of one stream pauses output

### Restore Stream 2

```bash
./net/rtsp/test_dual_stream.sh unblock-stream2
```

Stream 2 should reconnect and:
- Independent mode: Immediately starts outputting again
- Synchronized mode: Compositor catches up and resumes synced output

## Architecture

### Independent Mode
```
Pipeline:
  ┌─────────────────────────┐
  │  stream1-bin            │
  │  ┌──────────────────┐   │
  │  │ rtspsrc2-stream1 │   │
  │  └────────┬─────────┘   │
  │           │             │
  │  ┌────────▼─────────┐   │
  │  │ decodebin        │   │
  │  └────────┬─────────┘   │
  │           │             │
  │  ┌────────▼─────────┐   │
  │  │ queue → fakesink │   │
  │  └──────────────────┘   │
  └─────────────────────────┘

  ┌─────────────────────────┐
  │  stream2-bin            │
  │  ┌──────────────────┐   │
  │  │ rtspsrc2-stream2 │   │
  │  └────────┬─────────┘   │
  │           │             │
  │  ┌────────▼─────────┐   │
  │  │ decodebin        │   │
  │  └────────┬─────────┘   │
  │           │             │
  │  ┌────────▼─────────┐   │
  │  │ queue → fakesink │   │
  │  └──────────────────┘   │
  └─────────────────────────┘
```

### Synchronized Mode (like nvstreammux)
```
Pipeline:
  ┌─────────────────────────────────┐
  │  stream1-source                 │
  │  ┌──────────────────┐           │
  │  │ rtspsrc2-stream1 │           │
  │  └────────┬─────────┘           │
  │           │                     │
  │  ┌────────▼─────────┐           │
  │  │ decodebin        │           │
  │  └────────┬─────────┘           │
  │           │                     │
  │  ┌────────▼────────────────┐    │
  │  │ convert→scale→caps→queue│    │
  │  └────────┬────────────────┘    │
  │           │ (ghostpad)          │
  └───────────┼─────────────────────┘
              │
              ├──────────────┐
              │              │
  ┌───────────▼────────┐     │     ┌─────────────────────┐
  │  compositor        │◄────┘     │  stream2-source     │
  │  (mux/aggregator)  │           │  (similar structure)│
  │  - Keeps frames    │           └─────────────────────┘
  │    synchronized    │
  │  - Waits for all   │
  │    inputs          │
  └──────────┬─────────┘
             │
             │ Synchronized output (1280x480 side-by-side)
             │
  ┌──────────▼─────────┐
  │ queue → convert    │
  │    → fakesink      │
  └────────────────────┘
```

## Manual Testing

You can also test with real cameras or different configurations:

```bash
RTSP_URL1="rtsp://camera1.local/stream" \
RTSP_URL2="rtsp://camera2.local/stream" \
cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup_dual_stream
```

## Cleanup

Remove the loopback IP when done:

```bash
./net/rtsp/test_dual_stream.sh remove-ip
```

Remove any firewall rules:

```bash
./net/rtsp/test_dual_stream.sh unblock-stream2
```

## What This Tests

### Common to Both Modes
1. **UDP Transport Fix** - Both modes use UDP, demonstrating the `0.0.0.0` sender address fix
2. **Resource Cleanup** - Each stream can be stopped/restarted independently
3. **Stream Isolation** - Network issues on one stream are isolated to that stream's bin

### Independent Mode Tests
1. **Resilient Processing** - One stream failing doesn't stop the other
2. **Independent Frame Rates** - Streams can have different FPS
3. **Use Case:** Multi-camera monitoring, recording multiple feeds

### Synchronized Mode Tests
1. **Frame Alignment** - Compositor ensures frames from all streams are time-aligned
2. **Aggregate Behavior** - Like nvstreammux, waits for all inputs
3. **Coordinated Output** - Single output stream with synchronized frames
4. **Use Case:** Multi-camera inference, batch processing, synchronized analysis

## Comparison with DeepStream nvstreammux

The synchronized mode (`rtspsrc_synced_dual_stream.rs`) is similar to NVIDIA DeepStream's `nvstreammux`:

| Feature | nvstreammux | compositor (this example) |
|---------|-------------|---------------------------|
| **Frame Synchronization** | ✅ Yes | ✅ Yes |
| **Batching** | ✅ Hardware-accelerated | ⚠️ CPU-based |
| **Multiple inputs** | ✅ Yes | ✅ Yes |
| **Wait for all streams** | ✅ Yes | ✅ Yes |
| **Time alignment** | ✅ Yes | ✅ Yes |
| **GPU Zero-copy** | ✅ CUDA | ❌ CPU memory |
| **Format normalization** | ✅ Yes | ✅ Yes (videoconvert) |

**Key difference:** nvstreammux is optimized for NVIDIA GPUs and creates batches for inference. The compositor approach works on any platform but uses CPU memory.

## Future Enhancements

The current examples are simplified for clarity. Future versions could add:

### For Both Modes
- Dynamic stream addition/removal while pipeline is running
- Per-stream health monitoring and metrics
- Automatic reconnection logic triggered by errors
- Different transport protocols per stream (one TCP, one UDP)

### For Independent Mode
- Compositor option to visualize all streams
- Per-stream recording to separate files
- Load balancing across multiple sinks

### For Synchronized Mode
- Custom aggregator plugin for batch inference (like nvstreammux)
- Hardware-accelerated composition on GPU
- Adaptive sync that drops frames when streams desync
- Integration with inference plugins (e.g., onnx, tensorflow)
- PTS-based frame alignment (instead of compositor's implicit sync)
