# FallbackSrc Architecture and Implementation

## Overview

`fallbacksrc` is a sophisticated GStreamer bin element designed to handle unreliable media sources with automatic reconnection, retries, and fallback capabilities. It's implemented in Rust in the `utils/fallbackswitch` plugin.

## Source Code Location

The implementation is located at:
- **Main implementation:** `utils/fallbackswitch/src/fallbacksrc/imp.rs`
- **Module definition:** `utils/fallbackswitch/src/fallbacksrc/mod.rs`
- **Custom source wrapper:** `utils/fallbackswitch/src/fallbacksrc/custom_source/`

## Core Architecture

### 1. Dual Source Management

The element manages two independent sources:
```rust
struct State {
    source: SourceBin,           // Primary source
    fallback_source: Option<SourceBin>, // Backup source
    // ...
}
```

Each `SourceBin` contains:
- The actual source element (typically `uridecodebin3`)
- Timeout management for failure detection
- Restart/retry logic
- Stream state tracking

### 2. Source Types Supported

**Primary Source:**
- URI-based sources (using `uridecodebin3`)
- Custom source elements (any GStreamer element)

**Fallback Source:**
- URI to alternative content
- Can be a test pattern, cached content, or alternative stream

### 3. Key Configuration Parameters

```rust
struct Settings {
    uri: Option<String>,              // Primary URI
    source: Option<gst::Element>,    // Custom source element
    fallback_uri: Option<String>,    // Backup URI
    timeout: gst::ClockTime,          // Detection timeout (default: 5s)
    restart_timeout: gst::ClockTime, // Delay between restarts (default: 5s)
    retry_timeout: gst::ClockTime,   // Total retry duration (default: 60s)
    restart_on_eos: bool,            // Auto-restart on stream end
    immediate_fallback: bool,        // Start with fallback immediately
    manual_unblock: bool,            // Manual control mode
    // ...
}
```

## How It Works

### 1. Source Creation and Setup

When starting, `fallbacksrc`:
1. Creates a bin containing the source element (usually `uridecodebin3`)
2. Sets up monitoring for pad additions/removals
3. Configures timeout handlers
4. Optionally creates a fallback source bin

### 2. Failure Detection Mechanisms

The element detects failures through multiple mechanisms:

**Timeout Detection:**
- Monitors if source doesn't produce data within `timeout` period
- Triggers source restart when timeout expires

**Error Handling:**
- Captures error messages from source elements
- Categorizes errors by reason (Error, EOS, StateChangeFailure, Timeout)

**State Monitoring:**
- Tracks state changes of internal elements
- Detects failed state transitions

### 3. Retry Logic

When a source fails:

```rust
fn handle_source_error(&self, state: &mut State, reason: RetryReason, fallback_source: bool) {
    // 1. Update retry statistics
    if fallback_source {
        state.stats.num_fallback_retry += 1;
        state.stats.last_fallback_retry_reason = reason;
    } else {
        state.stats.num_retry += 1;
        state.stats.last_retry_reason = reason;
    }
    
    // 2. Mark source for restart
    source.pending_restart = true;
    
    // 3. Remove blocking probes to prevent deadlock
    // 4. Schedule asynchronous restart
    // 5. Start retry timeout counter
}
```

### 4. Source Switching

The element seamlessly switches between sources using:
- **Pad probes** to block/unblock data flow
- **Queue elements** for buffering during transitions
- **Fallbackswitch elements** for gapless switching

### 5. Statistics Tracking

Real-time statistics are available:
```rust
struct Stats {
    num_retry: u64,                      // Primary source retry count
    num_fallback_retry: u64,             // Fallback source retry count
    last_retry_reason: RetryReason,      // Why last retry occurred
    buffering_percent: i32,              // Current buffering state
    // ...
}
```

## Stream Management

### Stream-Aware vs Non-Stream-Aware Sources

**Stream-Aware Sources** (e.g., modern elements):
- Provide `StreamCollection` messages
- Handle stream selection natively
- `fallbacksrc` acts as passthrough

**Non-Stream-Aware Sources** (legacy elements):
- `fallbacksrc` creates synthetic streams
- Manages stream selection internally
- Uses `CustomSource` wrapper for compatibility

### Dynamic Stream Handling

The element handles:
- Dynamic pad addition/removal
- Stream format changes
- Multiple audio/video streams
- Stream synchronization

## Integration with uridecodebin3

By default, `fallbacksrc` uses `uridecodebin3` internally, which:
1. Automatically selects appropriate source element based on URI scheme
2. For RTSP, this means using `rtspsrc` (not `rtspsrc2` by default)
3. Handles demuxing and decoding

To use a custom source (like `rtspsrc2`):
```rust
// Set the 'source' property instead of 'uri'
fallbacksrc.set_property("source", &custom_element);
```

## State Machine

The element maintains several states:
```rust
enum Status {
    Stopped,     // Not running
    Buffering,   // Accumulating data
    Retrying,    // Attempting reconnection
    Running,     // Normal operation
}
```

## Advanced Features

### 1. Manual Unblock Mode
Allows application to control when to switch sources

### 2. Immediate Fallback
Starts with fallback source while primary is starting up

### 3. Buffer Duration Control
Configurable buffering for smooth transitions

### 4. Live Source Detection
Automatically adapts behavior for live vs non-live sources

## Usage Example

```bash
# Basic usage with automatic retry
gst-launch-1.0 \
    fallbacksrc \
        uri=rtsp://unreliable-camera:554/stream \
        fallback-uri=file:///backup-video.mp4 \
        timeout=3000000000 \
        retry-timeout=30000000000 \
        restart-timeout=2000000000 \
    ! decodebin ! autovideosink

# With custom source element
gst-launch-1.0 \
    fallbacksrc name=fsrc \
        timeout=5000000000 \
    ! decodebin ! autovideosink \
    \
    rtspsrc2 location=rtsp://camera:554/stream ! fsrc.source
```

## Limitations and Considerations

1. **Not a drop-in replacement** - Requires pipeline restructuring
2. **Memory overhead** - Maintains multiple source bins
3. **Complexity** - More complex than simple source elements
4. **Latency** - May introduce additional latency during switching

## Conclusion

`fallbacksrc` is a production-ready solution for handling unreliable media sources. Rather than creating custom reconnection logic, it provides:
- Proven retry mechanisms
- Statistical monitoring
- Seamless fallback capabilities
- Extensive configurability

For RTSP over intermittent radio links, combining `fallbacksrc` with appropriate timeout tuning provides a robust solution without requiring custom element development.