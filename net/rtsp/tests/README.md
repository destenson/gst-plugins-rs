# RTSP Test Infrastructure

This directory contains the test infrastructure for the GStreamer RTSP plugin (rtspsrc2). It includes both mock servers for unit testing and real GStreamer-based RTSP servers for integration testing.

## Test Components

### 1. Mock Server (`mock_server.rs`)
A lightweight mock RTSP server for unit tests that simulates RTSP protocol interactions without requiring real GStreamer components.

### 2. Real RTSP Test Server (`rtsp_test_server.rs`)
A GStreamer-based RTSP test server that provides realistic testing with actual RTP/RTCP timing, seeking support, and various content types.

### 3. Test Scripts
- `scripts/run-tests.sh` - Linux/macOS test runner
- `scripts/run-tests.bat` - Windows test runner

## Running Tests

### Quick Test
```bash
# Run basic tests with mock server
cargo test -p gst-plugin-rtsp

# Run with real RTSP server (Linux/macOS)
cd net/rtsp
./scripts/run-tests.sh live

# Run with real RTSP server (Windows)
cd net\rtsp
scripts\run-tests.bat live
```

### Integration Tests
```bash
# Run integration tests with real server (requires gst-rtsp-server)
cargo test -p gst-plugin-rtsp --test gst_server_integration -- --ignored

# Run VOD/seeking tests
./scripts/run-tests.sh vod

# Run all test suites
./scripts/run-tests.sh all
```

## Test Server Types

The test infrastructure supports multiple server configurations:

1. **Live Stream** - Real-time test pattern
2. **VOD (Video on Demand)** - File-based content with seeking support
3. **Audio Only** - Audio test stream
4. **Audio+Video** - Combined streams
5. **Authenticated** - Server with authentication

## Requirements

### Basic Testing
- GStreamer 1.16+ with core plugins
- Rust toolchain

### Integration Testing (Recommended)
- gst-rtsp-server or gst-plugins-bad (for rtspsink)
- x264enc for video encoding
- opusenc for audio encoding

### Installing Dependencies

#### Linux (Ubuntu/Debian)
```bash
sudo apt-get install \
    gstreamer1.0-tools \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-rtsp \
    gstreamer1.0-libav
```

#### macOS
```bash
brew install gstreamer gst-plugins-base gst-plugins-good \
    gst-plugins-bad gst-plugins-ugly gst-rtsp-server
```

#### Windows
Download and install GStreamer from: https://gstreamer.freedesktop.org/download/

## Writing New Tests

### Using Mock Server
```rust
use crate::mock_server::MockRtspServer;

#[tokio::test]
async fn test_with_mock() {
    let server = MockRtspServer::new().await;
    let url = server.url();
    // Test with mock server...
}
```

### Using Real Server
```rust
use crate::rtsp_test_server::GstRtspTestServer;

#[test]
fn test_with_real_server() {
    let server = GstRtspTestServer::new_live()
        .expect("Failed to start server");
    let url = server.url();
    // Test with real server...
}
```

## Test Categories

### Unit Tests
Basic functionality tests using mock servers:
- Protocol handling
- State management
- Error conditions
- Retry logic

### Integration Tests
Real-world scenarios with GStreamer servers:
- Live streaming
- VOD seeking
- Authentication
- RTCP feedback
- Reconnection
- Multiple streams

### Performance Tests
Benchmarks and stress tests:
- Connection racing
- Adaptive retry
- Telemetry

## Troubleshooting

### Server Won't Start
1. Check if port 8554 is available: `netstat -an | grep 8554`
2. Verify GStreamer installation: `gst-inspect-1.0 --version`
3. Check for rtspsink: `gst-inspect-1.0 rtspsink`

### Tests Fail with "Server not found"
The integration tests require gst-rtsp-server or rtspsink. If not available, tests will use mock servers with limited functionality.

### Windows Specific Issues
- Ensure GStreamer bin directory is in PATH
- Use Command Prompt or PowerShell (not Git Bash)
- Check Windows Firewall settings for port 8554

## CI/CD Integration

Tests are automatically run in CI with:
- Mock servers for unit tests
- Real servers when available for integration tests
- Fallback to mock servers when real servers unavailable

## Contributing

When adding new RTSP features:
1. Add unit tests with mock server
2. Add integration tests with real server
3. Update test scripts if new server configurations needed
4. Document any new dependencies or requirements