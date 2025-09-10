# PRP-001: cargo-deb Setup and Basic Configuration

## Overview
Configure cargo-deb for the gst-plugin-fallbackswitch package to enable Debian package generation. This establishes the foundation for building .deb packages from the Rust plugin.

## Context
- Location: utils/fallbackswitch/
- Package name: gst-plugin-fallbackswitch
- Library name: gstfallbackswitch
- Contains: fallbacksrc and fallbackswitch GStreamer elements
- Build system: cargo-c for C library generation
- Target: Debian/Ubuntu systems with GStreamer 1.0

## References
- cargo-deb documentation: https://github.com/kornelski/cargo-deb
- cargo-deb crate docs: https://docs.rs/cargo-deb
- Example from stream-manager: apps/stream-manager/scripts/install.sh

## Implementation Tasks
1. Install cargo-deb tool if not present
2. Add [package.metadata.deb] section to utils/fallbackswitch/Cargo.toml
3. Configure basic metadata fields (maintainer, copyright, license-file)
4. Set up extended description for the package
5. Define package section as "libs" for GStreamer plugins
6. Configure priority as "optional"
7. Test basic deb generation with cargo deb command
8. Verify package metadata with dpkg-deb --info

## Validation Gates
```bash
# Build the plugin first
cargo build -p gst-plugin-fallbackswitch --release

# Generate the deb package
cd utils/fallbackswitch && cargo deb

# Verify package was created
ls target/debian/*.deb

# Check package info
dpkg-deb --info target/debian/*.deb
```

## Success Criteria
- cargo deb command runs without errors
- .deb file is generated in target/debian/
- Package metadata displays correctly
- No lintian errors for basic package structure

## Dependencies
- Rust toolchain (1.76+)
- cargo-deb installed
- dpkg-dev for package inspection

## Notes
- Use MPL-2.0 license as specified in the plugin
- Maintainer should follow Debian standards format
- Package naming should follow Debian conventions

## Confidence Score: 9/10
Basic cargo-deb setup is straightforward with clear documentation and examples available.