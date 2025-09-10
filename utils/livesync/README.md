# GStreamer Livesync Plugin

A GStreamer plugin that provides live synchronization capabilities for media streams.

## Overview

The `livesync` element automatically keeps live sources valid and synchronized, making it essential for live streaming applications requiring high availability and proper timing.

## Features

- **Live Source Synchronization**: Automatically maintains proper timing for live sources
- **Latency Management**: Handles latency adjustments to keep streams synchronized
- **Buffer Management**: Intelligent buffering for smooth playback
- **Clock Synchronization**: Maintains proper clock relationships in live pipelines

## Installation

### From Debian Package

The easiest way to install the plugin is via the Debian package:

```bash
# Install the package
sudo dpkg -i gst-plugin-livesync_*.deb

# Or with apt to resolve dependencies
sudo apt install ./gst-plugin-livesync_*.deb

# Verify installation
gst-inspect-1.0 livesync
```

### Building from Source

#### Prerequisites

- Rust toolchain (1.83 or newer)
- GStreamer development libraries (>= 1.16)
- cargo-deb (for Debian package generation)

```bash
# Install dependencies on Debian/Ubuntu
sudo apt-get update
sudo apt-get install \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    pkg-config \
    build-essential

# Install cargo-deb
cargo install cargo-deb
```

#### Build Steps

```bash
# Clone the repository
git clone https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs.git
cd gst-plugins-rs/utils/livesync

# Build the plugin
cargo build --release

# Generate Debian package
cargo deb

# Package will be in target/debian/
ls target/debian/*.deb
```

## Usage

### Basic Pipeline

```bash
# Live source with livesync
gst-launch-1.0 \
    videotestsrc is-live=true ! \
    livesync ! \
    autovideosink
```

### RTSP Stream with Livesync

```bash
# Synchronize an RTSP stream
gst-launch-1.0 \
    rtspsrc location=rtsp://camera.local:554/stream latency=0 ! \
    rtph264depay ! h264parse ! \
    livesync ! \
    decodebin ! \
    autovideosink
```

### Multiple Live Sources

```bash
# Synchronize multiple live sources
gst-launch-1.0 \
    videotestsrc is-live=true pattern=0 ! \
    livesync name=sync1 ! \
    videomixer name=mix ! \
    autovideosink \
    \
    videotestsrc is-live=true pattern=1 ! \
    livesync name=sync2 ! \
    mix.
```

## Properties

The `livesync` element supports the following properties:

- **latency** (uint64): Additional latency to add (in nanoseconds)
- **single-segment** (boolean): Produce a single segment
- **upstream-latency** (uint64): Upstream latency (read-only)

## Architecture Support

The Debian package supports multiple architectures:

- **amd64** (x86_64)
- **arm64** (aarch64)
- **armhf** (ARMv7)

The plugin is installed to the architecture-specific GStreamer plugin directory:
- `/usr/lib/x86_64-linux-gnu/gstreamer-1.0/` (amd64)
- `/usr/lib/aarch64-linux-gnu/gstreamer-1.0/` (arm64)
- `/usr/lib/arm-linux-gnueabihf/gstreamer-1.0/` (armhf)

## Development

### Running Tests

```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --all-features

# Run with GStreamer debug output
GST_DEBUG=livesync:7 cargo test
```

### Debugging

Enable debug output to troubleshoot issues:

```bash
# Set debug level for livesync
export GST_DEBUG=livesync:6

# Run your pipeline
gst-launch-1.0 videotestsrc is-live=true ! livesync ! autovideosink
```

## Troubleshooting

### Plugin Not Found

If `gst-inspect-1.0 livesync` doesn't find the plugin:

1. Update the GStreamer plugin cache:
   ```bash
   rm -rf ~/.cache/gstreamer-1.0/
   gst-inspect-1.0 --gst-plugin-path=/usr/lib/$(dpkg-architecture -qDEB_HOST_MULTIARCH)/gstreamer-1.0
   ```

2. Check installation path:
   ```bash
   dpkg -L gst-plugin-livesync | grep .so
   ```

3. Verify library dependencies:
   ```bash
   ldd /usr/lib/*/gstreamer-1.0/libgstlivesync.so
   ```

### Latency Issues

If experiencing high latency:

1. Check pipeline latency:
   ```bash
   gst-launch-1.0 -v your-pipeline 2>&1 | grep latency
   ```

2. Adjust livesync latency property:
   ```bash
   ... ! livesync latency=100000000 ! ...  # 100ms additional latency
   ```

## License

This plugin is licensed under the Mozilla Public License Version 2.0 (MPL-2.0).
See LICENSE-MPL-2.0 for details.

## Contributing

Contributions are welcome! Please submit merge requests to:
https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs

## Support

For issues and questions:
- Issue Tracker: https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs/-/issues
- Mailing List: gstreamer-rust@lists.freedesktop.org
- Matrix: #gstreamer:matrix.org