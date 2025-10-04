# Frame Monitoring in rtspsrc_cleanup Example

## Overview

The `rtspsrc_cleanup` example now includes frame monitoring to verify that video frames continue to flow through the pipeline during reconnections and source rebuilds.

## Features

### 1. Frame Counter (Default Mode)

When running with `fakesink` (default), the example:
- Counts every frame that reaches the sink
- Reports statistics every 5 seconds:
  - **Total frames**: Cumulative frame count since startup
  - **FPS (last 5s)**: Frame rate over the most recent 5-second period
  - **FPS (avg)**: Average frame rate since the pipeline started

Example output:
```
Frame stats: 150 total frames, 30.0 fps (last 5s), 30.0 fps (avg)
Frame stats: 300 total frames, 30.0 fps (last 5s), 30.0 fps (avg)
Frame stats: 450 total frames, 30.0 fps (last 5s), 30.0 fps (avg)
```

This allows you to:
- Verify frames are flowing continuously
- Detect stalls or drops during reconnections
- Monitor pipeline health over time

### 2. Visual Verification Mode

Use the `--autovideosink` flag to display the video stream in a window instead of just counting frames.

```bash
cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
  --url rtsp://192.168.12.38:8554/test-h264 \
  --autovideosink
```

This mode:
- Opens a video window showing the live stream
- Lets you visually verify frame continuity during reconnections
- Useful for debugging visual artifacts or frozen frames

## Usage Examples

### Default: Frame counter with periodic restarts
```bash
cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
  --url rtsp://192.168.12.38:8554/test-h264 \
  --restart-interval 15
```

### Visual mode with reconnection testing
```bash
cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
  --url rtsp://192.168.12.38:8554/test-h264 \
  --restart-interval 25 \
  --autovideosink
```

### Using the helper script
```bash
# Default: frame counter mode
./net/rtsp/test_reconnection_cleanup.sh rtsp://192.168.12.38:8554/test-h264 15

# To use visual mode, modify the script or run directly:
cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
  --url rtsp://192.168.12.38:8554/test-h264 \
  --restart-interval 15 \
  --autovideosink
```

## Implementation Details

### Frame Counting

- Uses `AtomicU64` for thread-safe frame counting
- Pad probe on the fakesink's sink pad increments the counter for each buffer
- Periodic timer (5-second intervals) reads the counter and calculates statistics
- Tracks time using `Instant` for accurate FPS measurements

### Statistics Calculation

```rust
fps_last_5s = (current_count - last_count) / elapsed_time
fps_avg = total_count / total_elapsed_time
```

## Troubleshooting

### No frames reported

If you see:
```
Frame stats: 0 total frames, 0.0 fps (last 5s), 0.0 fps (avg)
```

Check:
1. RTSP URL is correct and reachable
2. `GST_PLUGIN_PATH` includes the rtspsrc2 plugin
3. Network connectivity to the RTSP server
4. Check for error messages in the output

### Low or unstable FPS

Possible causes:
- Network issues (check for "Connection retry" messages)
- Server-side performance problems
- CPU bottlenecks on the client
- Periodic restarts (expected brief drops during rebuilds)

### Visual mode shows nothing

If `--autovideosink` shows a blank window:
- Ensure you have a display server (X11/Wayland)
- Check that GStreamer video sink plugins are installed
- Try running without `--autovideosink` to see if frames are being received
