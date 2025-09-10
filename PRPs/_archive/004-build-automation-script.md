# PRP-004: Automated Build Script for Debian Package Generation

## Overview
Create a build script that automates the complete process of building the gst-plugin-fallbackswitch Debian package with proper error handling and cross-architecture support.

## Context
- Need to coordinate cargo build, cargo-c, and cargo-deb
- Support multiple architectures (amd64, arm64, armhf)
- Handle build dependencies installation
- Provide clean build environment setup

## References
- Example build script: apps/stream-manager/scripts/install.sh
- cargo cross-compilation: https://doc.rust-lang.org/cargo/reference/config.html
- Debian packaging helpers: https://www.debian.org/doc/manuals/maint-guide/build.en.html

## Implementation Tasks
1. Create utils/fallbackswitch/scripts/build-deb.sh
2. Add architecture detection and validation
3. Implement build dependency checking
4. Add cargo and cargo-c build steps with error handling
5. Configure release build optimizations
6. Implement package generation with cargo-deb
7. Add package validation and testing steps
8. Create cleanup function for build artifacts

## Validation Gates
```bash
# Make script executable and test
chmod +x utils/fallbackswitch/scripts/build-deb.sh

# Test basic build
./utils/fallbackswitch/scripts/build-deb.sh

# Test with specific architecture
./utils/fallbackswitch/scripts/build-deb.sh --arch arm64

# Verify output
ls -la utils/fallbackswitch/target/debian/*.deb
file utils/fallbackswitch/target/debian/*.deb
```

## Success Criteria
- Script completes successfully on multiple architectures
- Proper error messages for missing dependencies
- Generated .deb passes validation checks
- Build artifacts are properly cleaned up
- Script is idempotent (can run multiple times)

## Dependencies
- Bash shell
- Build essentials (gcc, make, pkg-config)
- Rust toolchain with target architectures
- cargo-c and cargo-deb tools

## Notes
- Use set -e for fail-fast behavior
- Add verbose mode for debugging
- Consider Docker/podman for isolated builds
- Include version detection from git tags

## Script Structure
```
#!/bin/bash
# 1. Setup and validation
# 2. Dependency checks
# 3. Architecture configuration
# 4. Build process
# 5. Package generation
# 6. Validation
# 7. Cleanup
```

## Confidence Score: 9/10
Build automation follows established patterns with clear examples available.