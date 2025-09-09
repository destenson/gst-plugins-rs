# GStreamer Fallback Switch Plugin

This is a GStreamer plugin that provides fallback switching and source elements for high-availability streaming applications.

## Elements

### fallbackswitch
Automatically switches between multiple input sources based on their availability. When the primary source fails or becomes unavailable, it seamlessly switches to a fallback source.

### fallbacksrc
A composite source element that provides built-in fallback capabilities. It can automatically generate test patterns or switch to alternative sources when the primary source is unavailable.

## Building

### Prerequisites
- Rust 1.83 or later
- GStreamer 1.18 or later development files
- pkg-config

### Build from source
```bash
cargo build --release --package gst-plugin-fallbackswitch
```

### Run tests
```bash
cargo test --package gst-plugin-fallbackswitch
```

## Debian Package

This plugin can be packaged as a Debian package for easy installation on Debian-based systems.

### Prerequisites for Building

- Rust toolchain (installed via rustup or system package manager)
- cargo-c (`cargo install cargo-c`)
- cargo-deb (`cargo install cargo-deb`)
- GStreamer development packages:
  - libgstreamer1.0-dev (>= 1.16.0)
  - libgstreamer-plugins-base1.0-dev (>= 1.16.0)
- pkg-config

### Building the Debian package

#### Quick build with provided script
```bash
cd utils/fallbackswitch
./scripts/build-deb.sh
```

#### Manual build with cargo-deb
```bash
# Build the library with cargo-c first
cargo cbuild -p gst-plugin-fallbackswitch --release

# Generate the Debian package
cd utils/fallbackswitch
cargo deb --no-build
```

### Package Installation Details

The package installs the plugin library to the architecture-specific GStreamer plugin directory:
- 64-bit: `/usr/lib/x86_64-linux-gnu/gstreamer-1.0/`
- 32-bit: `/usr/lib/i386-linux-gnu/gstreamer-1.0/`
- ARM64: `/usr/lib/aarch64-linux-gnu/gstreamer-1.0/`

The installation triggers `ldconfig` to update the library cache, ensuring GStreamer can discover the plugin.

### Installing the package
```bash
sudo dpkg -i target/debian/gst-plugin-fallbackswitch_*.deb
# Or with apt to resolve dependencies
sudo apt install ./target/debian/gst-plugin-fallbackswitch_*.deb
```

### Verifying installation
After installation, verify the plugin is available:
```bash
gst-inspect-1.0 fallbackswitch
gst-inspect-1.0 fallbacksrc

# Check if the library is in the correct path
ls -la /usr/lib/*/gstreamer-1.0/libgstfallbackswitch.so
```

### Validation
Run the validation script to check the package meets all requirements:
```bash
cd utils/fallbackswitch
./scripts/validate-deb.sh
```

## Usage Examples

### Basic fallbackswitch pipeline
```bash
gst-launch-1.0 \
  fallbackswitch name=switch \
  videotestsrc pattern=snow ! switch.sink_0 \
  videotestsrc pattern=smpte ! switch.sink_1 \
  switch. ! autovideosink
```

### Using fallbacksrc
```bash
gst-launch-1.0 \
  fallbacksrc \
    uri=rtsp://example.com/stream \
    fallback-uri=file:///path/to/fallback.mp4 \
  ! autovideosink
```

## Development

### Project Structure
```
utils/fallbackswitch/
├── Cargo.toml          # Rust package configuration with cargo-deb metadata
├── build.rs            # Build script
├── src/
│   ├── lib.rs          # Plugin registration
│   ├── fallbackswitch/ # Fallbackswitch element implementation
│   └── fallbacksrc/    # Fallbacksrc element implementation
├── tests/              # Integration tests
├── examples/           # Example applications
└── debian/             # Debian packaging files
```

### Debian Packaging Implementation Plan

The Debian packaging is being implemented through a series of PRPs (Project Request Proposals):

1. **PRP-001**: cargo-deb Setup and Basic Configuration (Foundational) ✓
2. **PRP-002**: GStreamer Plugin Installation Path Configuration ✓
   - Configured multiarch-aware installation paths
   - Added ldconfig trigger for library cache updates
   - Set proper file permissions (644) for the shared library
   - Implemented build and validation scripts
3. **PRP-003**: Debian Package Dependencies Configuration
4. **PRP-004**: Automated Build Script for Debian Package Generation
5. **PRP-005**: Debian Package Testing and Validation Framework
6. **PRP-006**: Post-Installation Configuration and Integration
7. **PRP-007**: CI/CD Pipeline for Automated Debian Package Building
8. **PRP-008**: Documentation and Release Management

## License

This plugin is licensed under the Mozilla Public License Version 2.0. See LICENSE-MPL-2.0 for details.

## Contributing

Contributions are welcome! Please submit merge requests to the [GStreamer Rust plugins repository](https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs).

## Support

For issues and questions:
- File issues at: https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs/-/issues
- Mailing list: gstreamer-rust@lists.freedesktop.org
- IRC: #gstreamer on OFTC