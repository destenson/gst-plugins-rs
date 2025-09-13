# rtspsrc2

Rust rewrite of rtspsrc, with the purpose of fixing the fundamentally broken
architecture of rtspsrc. There are some major problems with rtspsrc:

1. Element states are linked to RTSP states, which causes unfixable glitching
   and issues, especially in shared RTSP media
2. The command loop is fundamentally broken and buggy, which can cause RTSP
   commands such as `SET_PARAMETER` and `GET_PARAMETER` to be lost
3. The combination of the above two causes unfixable deadlocks when doing state
   changes due to external factors such as server state, or when seeking
4. Parsing of untrusted RTSP messages from the network was done in C with the
   `GstRTSPMessage` API.
5. Parsing of untrusted SDP from the network was done in C with the
   `GstSDPMessage` API

## Implemented features

* RTSP 1.0 support
* Lower transports: TCP, UDP, UDP-Multicast
* RTCP SR and RTCP RR
* RTCP-based A/V sync
* Lower transport selection and priority (NEW!)
  - Also supports different lower transports for each SETUP
* Connection retry logic with configurable backoff strategies (NEW!)
  - Multiple retry strategies: auto, adaptive, none, immediate, linear, exponential, exponential-jitter
  - Configurable retry parameters and limits
* Session timeout handling with automatic keep-alive (NEW!)
  - Parses session timeout from RTSP headers
  - Sends keep-alive messages (GET_PARAMETER) before timeout
  - Prevents session expiration during streaming
* GET_PARAMETER and SET_PARAMETER support (NEW!)
  - Used for keep-alive and camera control
  - Essential for PTZ cameras and ONVIF devices
* Performance optimizations (NEW!)
  - Buffer pool management for reduced allocations
  - TCP connection pooling for multiple streams from same server
  - Zero-copy buffer operations where possible
* Enhanced RTCP support (NEW!)
  - Extended RTCP statistics collection
  - RTCP XR (Extended Reports) support (RFC 3611)
  - Feedback message handling (RFC 4585)
  - VoIP quality metrics (R-factor, MOS score)
* Telemetry and observability (NEW!)
  - Structured logging with tracing
  - Metrics collection for monitoring
  - Connection and performance statistics
  - tokio-console support for async debugging
* Authentication support (NEW!)
  - HTTP Basic Authentication (RFC 7617)
  - HTTP Digest Authentication (RFC 7616)
  - Credentials from URL or properties
  - Automatic retry on 401 Unauthorized
* TLS/TCP support (NEW!)
  - Secure RTSP connections (rtsps://)
  - Default port 322 for RTSPS
  - Configurable TLS validation flags
  - Support for self-signed certificates

* HTTP tunneling support (NEW!)
  - Properties for tunnel mode and port configuration
  - Base64 encoding/decoding for RTSP messages
  - Dual HTTP connection management (GET/POST)
  - Auto-detection of tunneling need
* ONVIF backchannel preparation (NEW!)
  - SDP attribute parsing for stream direction (sendonly/recvonly/sendrecv)
  - Detection of ONVIF backchannel streams
  - Sink pad template for audio return channel
  - Signal emission for backchannel detection

## Missing features

Roughly in order of priority:
* NAT hole punching
* Allow ignoring specific streams (SDP medias)
  - Currently all available source pads must be linked
* SRTP support
* Proxy support
* Make TCP connection optional when using UDP transport
  - Or TCP reconnection if UDP has not timed out
* Parse more SDP attributes
  - extmap
  - key-mgmt
  - rid
  - rtcp-fb
  - source-filter
  - ssrc
* Clock sync support, such as RFC7273
* PAUSE support with VOD
* Seeking support with VOD
* ONVIF backchannel full implementation (prepared, data flow pending)
* ONVIF trick mode support
* RTSP 2 support (no servers exist at present)

## Testing

### Unit Tests

Unit tests have been added for the RTSP plugin. Run them with:

```bash
cargo test -p gst-plugin-rtsp rtspsrc
```

The test suite includes:
* Element registration and creation tests
* Property getter/setter validation  
* State transition testing
* Protocol parsing tests
* Signal connection tests

### Mock RTSP Server

A mock RTSP server has been implemented for testing RTSP protocol interactions:

```bash
cargo test -p gst-plugin-rtsp mock_server
```

The mock server provides:
* Basic RTSP command support (OPTIONS, DESCRIBE, SETUP, PLAY, TEARDOWN)
* Configurable SDP responses
* Session management
* TCP listener on configurable port

Note: Integration tests with the actual rtspsrc2 element are still in development

### Connection Retry Configuration

The rtspsrc2 element supports robust connection retry logic with various backoff strategies:

#### Properties

* `retry-strategy` (string): Connection retry strategy
  - `auto` (default): Automatic strategy selection based on connection conditions
  - `adaptive`: Learning-based optimization (placeholder for future enhancement)
  - `none`: No retry, fail immediately
  - `immediate`: Retry immediately without delay
  - `linear`: Fixed increment delays
  - `exponential`: Power of 2 backoff
  - `exponential-jitter`: Exponential with ±25% random jitter

* `max-reconnection-attempts` (int): Maximum retry attempts (-1 for infinite, 0 for no retry, default: 5)
* `reconnection-timeout` (nanoseconds): Maximum backoff delay (default: 30 seconds)
* `initial-retry-delay` (nanoseconds): Initial retry delay (default: 1 second)
* `linear-retry-step` (nanoseconds): Step increment for linear strategy (default: 2 seconds)

#### Example Usage

```bash
# Exponential backoff with 10 retry attempts
gst-launch-1.0 rtspsrc2 location=rtsp://camera.local/stream \
  retry-strategy=exponential \
  max-reconnection-attempts=10 \
  initial-retry-delay=500000000 ! \
  decodebin ! autovideosink

# No retry for testing
gst-launch-1.0 rtspsrc2 location=rtsp://camera.local/stream \
  retry-strategy=none ! \
  fakesink

# HTTP tunneling (automatic detection)
gst-launch-1.0 rtspsrc2 location=rtsp://camera.local/stream \
  protocols=tcp,http \
  http-tunnel-mode=auto ! \
  decodebin ! autovideosink

# Force HTTP tunneling on custom port
gst-launch-1.0 rtspsrc2 location=rtsp://camera.local/stream \
  http-tunnel-mode=always \
  tunnel-port=8080 ! \
  decodebin ! autovideosink
```

### HTTP Tunneling Configuration

The rtspsrc2 element supports HTTP tunneling for RTSP to bypass restrictive firewalls:

#### Properties

* `http-tunnel-mode` (string): HTTP tunneling mode
  - `auto` (default): Automatically detect need for tunneling
  - `never`: Never use HTTP tunneling
  - `always`: Always use HTTP tunneling

* `tunnel-port` (uint): Port to use for HTTP tunneling (default: 80)

* `protocols` (string): Include "http" to enable HTTP as a transport option

### ONVIF Backchannel Support (Preparation)

The rtspsrc2 element has initial support for detecting ONVIF backchannel streams:

* Parses SDP attributes for stream direction (sendonly/recvonly/sendrecv)
* Detects ONVIF backchannel streams in SDP
* Provides sink pad template for future audio return channel
* Emits `backchannel-detected` signal when backchannel stream is found

Note: Full backchannel data flow implementation is pending

## Camera Compatibility Testing

The plugin includes a comprehensive camera compatibility testing framework to validate rtspsrc2 against real IP cameras and RTSP servers. See [Camera Quirks Documentation](docs/CAMERA_QUIRKS.md) for known issues and workarounds.

### Running Compatibility Tests

```bash
# Run basic compatibility tests
cargo test -p gst-plugin-rtsp compat -- --nocapture

# Test with specific camera configuration
GST_DEBUG=rtspsrc2:5 cargo test -p gst-plugin-rtsp camera_hikvision

# Generate compatibility report
cargo test -p gst-plugin-rtsp --test camera_compatibility_tests
```

### Supported Camera Brands

The framework includes test configurations for:
* Axis Communications (M3045-V, P-Series)
* Hikvision (DS-2CD2132F, DS-2CD2385G1)
* Dahua Technology (IPC-HFW4431E, IPC-HDW4631C)
* ONVIF Profile S/T compatible devices

### Test Categories

1. **Basic Connectivity**: Authentication, transport negotiation
2. **Stream Formats**: H.264, H.265, MJPEG, audio codecs
3. **Features**: PTZ, ONVIF, events, backchannel
4. **Reliability**: Reconnection, timeout, error handling
5. **Performance**: Latency, throughput, stability

### Test Configuration

Camera configurations can be loaded from TOML or JSON files:

```toml
[[cameras]]
name = "Axis M3045-V"
vendor = "Axis"
model = "M3045-V"
url = "rtsp://192.168.1.100/axis-media/media.amp"
username = "root"
password = "password"
transport = "auto"
auth_type = "digest"

[cameras.features]
h264 = true
h265 = true
audio = true
onvif = true
```

## Installation

### From Debian Package

The easiest way to install the plugin is via the Debian package:

```bash
# Download and install the package
wget https://github.com/gstreamer/gst-plugins-rs/releases/latest/download/gst-plugin-rtsp_*.deb
sudo dpkg -i gst-plugin-rtsp_*.deb

# Or install dependencies if needed
sudo apt-get install -f

# Verify installation
gst-inspect-1.0 rtspsrc
```

### From Source

```bash
# Build and install from source
cargo build --release -p gst-plugin-rtsp
sudo cp target/release/libgstrsrtsp.so /usr/lib/x86_64-linux-gnu/gstreamer-1.0/

# Update GStreamer registry
gst-inspect-1.0 --gst-disable-registry-fork rtspsrc
```

## Package Information

This plugin is available as a Debian package (`gst-plugin-rtsp`) with the following features:
- Automatic GStreamer plugin registry integration
- Proper dependency management
- System-wide installation support
- Documentation and examples included

For build scripts and packaging tools, see the `scripts/` directory.

## Performance Features

### Buffer Pool Management
The plugin includes an efficient buffer pool to reduce memory allocations:
- Pre-allocated buffers for common packet sizes
- Automatic reuse of buffers
- Memory limit enforcement
- Zero-copy operations where possible

### TCP Connection Pooling
Reduces connection overhead when streaming from the same server:
- Reuses TCP connections for multiple streams
- Automatic health checking and cleanup
- Configurable pool size and idle timeout
- Thread-safe connection sharing

### RTCP Enhancements
Advanced RTCP statistics and feedback:
- Extended Reports (XR) for quality monitoring
- VoIP metrics including R-factor and MOS scores
- Feedback messages (NACK, PLI, FIR, REMB)
- Comprehensive jitter and packet loss tracking

### Telemetry
Enable telemetry features for production monitoring:
```bash
cargo build --features telemetry
```

This provides:
- Structured logging with tracing
- Prometheus metrics export (optional)
- tokio-console support for async debugging
- Performance event tracking

## Maintenance and future cleanup

* Test with market RTSP cameras
  - Camera compatibility testing framework has been implemented
  - Includes support for Axis, Hikvision, Dahua, and ONVIF devices
* Add tokio-console and tokio tracing support (COMPLETED)
