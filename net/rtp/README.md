# GStreamer Rust RTP Plugin

Advanced RTP handling capabilities for GStreamer, providing enhanced RTP send/receive functionality with custom payload handling and bandwidth estimation.

## Elements

### rtpsend - RTP Session Sender
A comprehensive RTP sender element that provides advanced control over RTP streams.

**Features:**
- Custom payload handling
- Congestion control integration
- Session management
- Multi-stream support

### rtprecv - RTP Session Receiver  
An advanced RTP receiver element with enhanced capabilities over the standard GStreamer RTP elements.

**Features:**
- Custom payload handling
- Loss detection and reporting
- Session management
- Multi-stream support

### rtpgccbwe - Google Congestion Control Bandwidth Estimator
Implements Google Congestion Control (GCC) bandwidth estimation for adaptive streaming.

**Features:**
- Real-time bandwidth estimation
- Network congestion detection
- Adaptive bitrate support
- Integration with rtpsend/rtprecv

## Payload Support

The plugin includes enhanced payloaders and depayloaders for various media formats:

**Audio:**
- AC-3 (rtpac3pay2/rtpac3depay2)
- AMR (rtpamrpay2/rtpamrdepay2) 
- Opus (rtpopuspay2/rtpopusdepay2)
- PCMA (rtppcmapay2/rtppcmadepay2)
- PCMU (rtppcmupay2/rtppcmudepay2)
- MPEG-4 Audio (rtpmp4apay2/rtpmp4adepay2)

**Video:**
- AV1 (rtpav1pay/rtpav1depay)
- VP8 (rtpvp8pay2/rtpvp8depay2)
- VP9 (rtpvp9pay2/rtpvp9depay2)
- JPEG (rtpjpegpay2/rtpjpegdepay2)

**Data:**
- MPEG-TS (rtpmp2tpay2/rtpmp2tdepay2)
- MPEG-4 Generic (rtpmp4gpay2/rtpmp4gdepay2)
- KLV Metadata (rtpklvpay2/rtpklvdepay2)

## Usage Examples

### Basic RTP Sending
```bash
gst-launch-1.0 videotestsrc ! \
  x264enc ! rtph264pay ! \
  rtpsend name=send ! \
  udpsink host=192.168.1.100 port=5004
```

### RTP Receiving with Bandwidth Estimation
```bash
gst-launch-1.0 udpsrc port=5004 ! \
  rtprecv name=recv ! \
  rtpgccbwe ! \
  rtph264depay ! avdec_h264 ! \
  autovideosink
```

### Advanced Multi-stream Setup
```bash
# Sender
gst-launch-1.0 \
  videotestsrc ! x264enc ! rtph264pay pt=96 ! rtpsend.send_rtp_sink_0 \
  audiotestsrc ! opusenc ! rtpopuspay pt=97 ! rtpsend.send_rtp_sink_1 \
  rtpsend name=rtpsend ! udpsink host=192.168.1.100 port=5004

# Receiver  
gst-launch-1.0 udpsrc port=5004 ! \
  rtprecv name=recv \
  recv.recv_rtp_src_0_96 ! rtph264depay ! avdec_h264 ! autovideosink \
  recv.recv_rtp_src_1_97 ! rtpopusdepay ! opusdec ! autoaudiosink
```

## Building

```bash
# Build the plugin
cargo build --release -p gst-plugin-rtp

# Run tests
cargo test -p gst-plugin-rtp

# Build with all features
cargo build --release -p gst-plugin-rtp --all-features
```

## Installation

### From Debian Package

```bash
# Install the plugin
sudo apt install gst-plugin-rtp

# Verify installation
gst-inspect-1.0 rtpsend
gst-inspect-1.0 rtprecv
gst-inspect-1.0 rtpgccbwe
```

### From Source

```bash
# Build and install
cargo build --release -p gst-plugin-rtp
sudo cp target/release/libgstrsrtp.so /usr/lib/x86_64-linux-gnu/gstreamer-1.0/

# Update GStreamer registry
gst-inspect-1.0 --gst-disable-registry-fork rtpsend
```

## Dependencies

- GStreamer 1.20 or later
- GStreamer Base plugins
- GStreamer RTP plugins  
- GStreamer Network plugins
- GStreamer Video plugins

## Package Information

This plugin is available as a Debian package (`gst-plugin-rtp`) with:
- Automatic GStreamer plugin registry integration
- Proper dependency management for GStreamer 1.20+
- System-wide installation support
- Complete documentation and examples

## License

MPL-2.0 - See LICENSE-MPL-2.0 for details.

## Contributing

This plugin is part of the gst-plugins-rs project. Please refer to the main repository for contribution guidelines.