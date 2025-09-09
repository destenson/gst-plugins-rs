# TODO - GStreamer Fallback Switch Debian Packaging

## Overview
This project implements Debian packaging for the gst-plugin-fallbackswitch, containing the fallbacksrc and fallbackswitch GStreamer elements. The goal is to enable single-command installation via `apt install gst-plugin-fallbackswitch` with immediate availability in GStreamer.

## PRP Implementation Status

### Completed (PRP-000)
- ✅ Initial project setup and overview
- ✅ cargo-deb configuration in utils/fallbackswitch/Cargo.toml
- ✅ Basic Debian package metadata structure (debian/ directory)
- ✅ Project README with installation and usage instructions

### Phase 1: Foundation
- **PRP-001: cargo-deb Setup** ⏳
  - Basic configuration added to Cargo.toml
  - Next: Refine asset paths for multiarch support
  
- **PRP-002: GStreamer Plugin Paths** 📋
  - Configure multiarch-aware installation paths
  - Ensure GStreamer discovers plugin automatically
  
- **PRP-003: Dependencies Configuration** 📋
  - Set up runtime dependencies for GStreamer
  - Configure automatic dependency detection

### Phase 2: Automation
- **PRP-004: Build Script** 📋
  - Create automated build script for Debian packages
  - Handle cross-compilation scenarios
  
- **PRP-005: Testing Framework** 📋
  - Implement comprehensive testing across distributions
  - Validate installation and functionality

### Phase 3: Integration
- **PRP-006: Post-Installation** 📋
  - Implement maintainer scripts for system integration
  - Handle GStreamer registry updates
  
- **PRP-007: CI/CD Pipeline** 📋
  - Integrate with existing CI infrastructure
  - Automate package building and publishing

### Phase 4: Release
- **PRP-008: Documentation** 📋
  - Create user-facing documentation
  - Establish release workflow

## Technical Details

### Package Structure
```
utils/fallbackswitch/
├── Cargo.toml              # Rust package with [package.metadata.deb]
├── debian/                  # Traditional Debian packaging files
│   ├── control             # Package metadata and dependencies
│   ├── rules               # Build instructions
│   ├── changelog           # Version history
│   ├── copyright           # License information
│   ├── compat              # Debhelper compatibility level
│   └── source/format       # Source package format
├── src/                    # Rust source code
│   ├── fallbackswitch/     # Switch element
│   └── fallbacksrc/        # Source element
└── README.md               # User documentation
```

### Build Requirements
- Rust 1.83+
- cargo-deb (installed via `cargo install cargo-deb`)
- GStreamer 1.18+ development libraries
- Debian build tools (dpkg-dev, debhelper)

### Build Commands
```bash
# Using cargo-deb (recommended)
cd utils/fallbackswitch
cargo deb

# Traditional Debian build
cd utils/fallbackswitch
dpkg-buildpackage -b -uc -us
```

### Installation Paths
- Plugin: `/usr/lib/{arch}/gstreamer-1.0/libgstfallbackswitch.so`
- Documentation: `/usr/share/doc/gst-plugin-fallbackswitch/`

## Next Steps

1. **Immediate**: Test cargo-deb package generation
2. **Short-term**: Implement multiarch support (PRP-002)
3. **Medium-term**: Create automated build scripts (PRP-004)
4. **Long-term**: Set up CI/CD pipeline (PRP-007)

## Known Issues

- Build compatibility: The codebase may have API compatibility issues with the latest gstreamer-rs bindings that need to be resolved for successful compilation.
- Multiarch paths: The asset paths in cargo-deb configuration need to be adjusted to support multiple architectures dynamically.

## Testing Checklist

- [ ] Package builds successfully with cargo-deb
- [ ] Package installs without errors
- [ ] Plugin is discoverable via `gst-inspect-1.0 fallbackswitch`
- [ ] Plugin is discoverable via `gst-inspect-1.0 fallbacksrc`
- [ ] Example pipelines work correctly
- [ ] Package removes cleanly
- [ ] Package upgrades work properly

## Resources

- cargo-deb documentation: https://github.com/kornelski/cargo-deb
- Debian Policy: https://www.debian.org/doc/debian-policy/
- GStreamer Plugin Guide: https://gstreamer.freedesktop.org/documentation/plugin-development/
- Project repository: https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs