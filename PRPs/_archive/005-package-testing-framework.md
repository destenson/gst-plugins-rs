# PRP-005: Debian Package Testing and Validation Framework

## Overview
Implement comprehensive testing for the gst-plugin-fallbackswitch Debian package to ensure it installs correctly and functions properly across different Debian/Ubuntu versions.

## Context
- Need to test installation, removal, and upgrade scenarios
- Validate plugin functionality post-installation
- Test across multiple Debian/Ubuntu versions
- Ensure no file conflicts or missing dependencies

## References
- Debian piuparts testing: https://piuparts.debian.org/
- GStreamer testing: utils/fallbackswitch/tests/fallbackswitch.rs
- Docker for testing: https://docs.docker.com/engine/reference/builder/
- lintian checks: https://lintian.debian.org/

## Implementation Tasks
1. Create utils/fallbackswitch/scripts/test-deb-package.sh
2. Implement Docker/Podman based testing for multiple distros
3. Add lintian checks for package compliance
4. Create GStreamer pipeline tests for installed plugin
5. Implement installation/upgrade/removal test scenarios
6. Add dependency resolution testing
7. Create test matrix for different architectures
8. Generate test report with results

## Validation Gates
```bash
# Run lintian checks
lintian utils/fallbackswitch/target/debian/*.deb

# Test installation in container
docker run --rm -v $(pwd):/workspace debian:bookworm bash -c "
  apt-get update && 
  apt-get install -y gstreamer1.0-tools &&
  dpkg -i /workspace/utils/fallbackswitch/target/debian/*.deb &&
  gst-inspect-1.0 fallbackswitch &&
  gst-launch-1.0 --gst-version
"

# Run package tests
./utils/fallbackswitch/scripts/test-deb-package.sh
```

## Success Criteria
- Package passes all lintian checks (or has justified overrides)
- Installs cleanly on Debian 11, 12 and Ubuntu 22.04, 24.04
- Plugin loads successfully in GStreamer
- Basic pipeline with fallbackswitch works
- Clean removal with no leftover files
- Upgrade from older version works correctly

## Test Scenarios
1. Fresh installation
2. Upgrade from previous version
3. Removal and purge
4. Installation with missing dependencies
5. Plugin functionality tests
6. Multi-architecture installation

## Dependencies
- Docker or Podman for isolated testing
- lintian for Debian policy checks
- GStreamer tools for functionality testing

## Notes
- Consider using autopkgtest framework
- Test both with and without recommended packages
- Validate against different GStreamer versions
- Document any known issues or limitations

## Confidence Score: 7/10
Testing framework requires careful setup but patterns are well-established.