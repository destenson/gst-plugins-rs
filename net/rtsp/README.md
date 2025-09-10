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

## Missing features

Roughly in order of priority:

* Credentials support
* TLS/TCP support
* NAT hole punching
* Allow ignoring specific streams (SDP medias)
  - Currently all available source pads must be linked
* SRTP support
* HTTP tunnelling
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
* ONVIF backchannel support
* ONVIF trick mode support
* RTSP 2 support (no servers exist at present)

## Missing configuration properties

These are some misc rtspsrc props that haven't been implemented in rtspsrc2
yet:

* latency
* do-rtx
* do-rtcp
* iface
* user-agent

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
```

## Maintenance and future cleanup

* Test with market RTSP cameras
  - Currently, only live555 and gst-rtsp-server have been tested
* Add tokio-console and tokio tracing support
