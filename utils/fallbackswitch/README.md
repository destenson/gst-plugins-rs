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

### Testing the Package

The package includes comprehensive testing scripts:

```bash
# Test the package across multiple distributions using Docker
./scripts/test-deb-package.sh target/debian/gst-plugin-fallbackswitch_*.deb

# Test on specific distribution
./scripts/test-deb-package.sh --distro debian:bookworm target/debian/*.deb

# Run tests locally without Docker (requires sudo)
./scripts/test-deb-package.sh --no-docker target/debian/*.deb
```

### Cross-Architecture Building

The build script supports cross-compilation for different architectures:

```bash
# Build for ARM64
./scripts/build-deb.sh --arch arm64

# Build for ARMhf (32-bit ARM)
./scripts/build-deb.sh --arch armhf

# Clean build with verbose output
./scripts/build-deb.sh --clean --verbose
```

### Debian Packaging Implementation

The Debian packaging has been implemented through a series of PRPs:

1. **PRP-001**: cargo-deb Setup and Basic Configuration ✓
2. **PRP-002**: GStreamer Plugin Installation Path Configuration ✓
3. **PRP-003**: Debian Package Dependencies Configuration ✓
   - Configured automatic dependency detection with $auto
   - Added GStreamer runtime dependencies
   - Set up recommends and suggests for companion packages
4. **PRP-004**: Automated Build Script ✓
   - Created build-deb.sh with cross-architecture support
   - Added command-line options for flexibility
   - Implemented proper error handling and validation
5. **PRP-005**: Package Testing Framework ✓
   - Created test-deb-package.sh for comprehensive testing
   - Docker-based testing across multiple distributions
   - Lintian policy compliance checking
6. **PRP-006**: Post-Installation Configuration ✓
   - Created maintainer scripts (postinst, prerm, postrm)
   - Automatic GStreamer registry updates
   - Clean removal and purge handling
7. **PRP-008**: Documentation and Release Management ✓
   - Created comprehensive README.Debian
   - Updated main README with installation instructions
   - Added troubleshooting and usage examples

## License

This plugin is licensed under the Mozilla Public License Version 2.0. See LICENSE-MPL-2.0 for details.

## Contributing

Contributions are welcome! Please submit merge requests to the [GStreamer Rust plugins repository](https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs).

## Support

For issues and questions:
- File issues at: https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs/-/issues
- Mailing list: gstreamer-rust@lists.freedesktop.org
- IRC: #gstreamer on OFTC