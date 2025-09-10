# PRP-003: Debian Package Dependencies Configuration

## Overview
Configure proper runtime and build dependencies for the gst-plugin-fallbackswitch Debian package to ensure it works correctly on target systems.

## Context
- Plugin requires GStreamer 1.0 runtime libraries
- Dependencies from Cargo.toml: gst, gst-base, gst-audio, gst-video
- Must use dpkg-shlibdeps for automatic dependency detection
- Optional features: gtk4 support (should be separate variant)

## References
- Debian shlibs documentation: https://www.debian.org/doc/debian-policy/ch-sharedlibs.html
- cargo-deb dependencies: https://github.com/kornelski/cargo-deb#dependencies
- GStreamer package names: Search "gstreamer1.0 debian package names"

## Implementation Tasks
1. Configure depends field in [package.metadata.deb]
2. Add gstreamer1.0-plugins-base as runtime dependency
3. Configure auto-depends for shared library detection
4. Set up separate-debug-symbols option
5. Add Recommends for commonly used companion plugins
6. Configure Suggests for optional features
7. Test dependency resolution with dpkg-shlibdeps
8. Validate minimal installation requirements

## Validation Gates
```bash
# Check shared library dependencies
ldd target/release/libgstfallbackswitch.so

# Generate package and check dependencies
cd utils/fallbackswitch && cargo deb
dpkg-deb --info target/debian/*.deb | grep Depends

# Test in minimal container
docker run --rm -it debian:bookworm bash
# Inside container:
# apt-get update && apt-get install -y gstreamer1.0-tools
# dpkg -i gst-plugin-fallbackswitch*.deb
# gst-inspect-1.0 fallbackswitch
```

## Success Criteria
- Package declares all required GStreamer dependencies
- Installation pulls in necessary runtime libraries
- No unresolved symbols when loading the plugin
- Works on fresh Debian/Ubuntu installation

## Dependencies
- libgstreamer1.0-0 (>= 1.20.0)
- libgstreamer-plugins-base1.0-0
- libc6
- Automatic detection via dpkg-shlibdeps

## Notes
- Version constraints should match GStreamer features used
- Consider using ${shlibs:Depends} variable
- May need different dependencies for different Debian versions

## Confidence Score: 8/10
Dependency detection is automated but requires validation across distributions.