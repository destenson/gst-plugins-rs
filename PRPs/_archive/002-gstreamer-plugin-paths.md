# PRP-002: GStreamer Plugin Installation Path Configuration

## Overview
Configure the Debian package to install the gstfallbackswitch library in the correct GStreamer plugin directory where it will be automatically discovered by GStreamer.

## Context
- GStreamer plugin paths on Debian/Ubuntu:
  - 64-bit: /usr/lib/x86_64-linux-gnu/gstreamer-1.0/
  - 32-bit: /usr/lib/i386-linux-gnu/gstreamer-1.0/
  - ARM64: /usr/lib/aarch64-linux-gnu/gstreamer-1.0/
- Library file: libgstfallbackswitch.so
- Must be discoverable by gst-inspect-1.0

## References
- GStreamer plugin installation docs: https://gstreamer.freedesktop.org/documentation/installing/on-linux.html
- Debian library paths: https://wiki.debian.org/Multiarch/LibraryPathOverview
- cargo-c metadata: utils/fallbackswitch/Cargo.toml (lines 47-59)

## Implementation Tasks
1. Add assets section to [package.metadata.deb] in Cargo.toml
2. Configure library installation with architecture-aware path
3. Use cargo-deb's built-in variable substitution for multiarch paths
4. Set correct permissions (644) for the shared library
5. Add ldconfig trigger for library cache update
6. Configure stripping of debug symbols for release build
7. Test installation path resolution for different architectures
8. Verify GStreamer plugin discovery post-installation

## Validation Gates
```bash
# Build the library with cargo-c first
cargo cbuild -p gst-plugin-fallbackswitch --release

# Check the built library
file target/release/libgstfallbackswitch.so

# Generate deb and extract to check paths
cd utils/fallbackswitch && cargo deb
dpkg-deb -c target/debian/*.deb | grep gstreamer

# Test installation (in container/VM)
dpkg -i target/debian/*.deb
gst-inspect-1.0 fallbackswitch
```

## Success Criteria
- Library installs to /usr/lib/{arch}/gstreamer-1.0/
- gst-inspect-1.0 finds the plugin after installation
- Both fallbacksrc and fallbackswitch elements are registered
- ldconfig processes the library correctly

## Dependencies
- cargo-c for library building
- Architecture detection in cargo-deb
- GStreamer 1.0 development files

## Notes
- Must handle multiarch paths correctly
- Library name must match GStreamer naming convention
- Consider using [package.metadata.deb.variants] for architecture-specific builds

## Confidence Score: 8/10
Path configuration is well-documented but multiarch handling requires careful testing.