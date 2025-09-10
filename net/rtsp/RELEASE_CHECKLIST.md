# Release Checklist for gst-plugin-rtsp

This document provides a checklist for creating releases of the gst-plugin-rtsp Debian package.

## Pre-Release Validation

### Code Quality
- [ ] All tests pass: `cargo test -p gst-plugin-rtsp`
- [ ] No clippy warnings: `cargo clippy -p gst-plugin-rtsp`
- [ ] Documentation builds: `cargo doc -p gst-plugin-rtsp --no-deps`
- [ ] Code formatting is consistent: `cargo fmt --check`

### Package Building
- [ ] Package builds successfully: `./scripts/build-deb.sh`
- [ ] Package passes lintian: `lintian target/debian/*.deb`
- [ ] Package contents are correct: `dpkg-deb -c target/debian/*.deb`
- [ ] Package installs cleanly: `./scripts/test-deb-package.sh`

### Functionality Testing
- [ ] Plugin loads in GStreamer: `gst-inspect-1.0 rtspsrc`
- [ ] Basic RTSP pipeline works
- [ ] Authentication mechanisms tested
- [ ] Transport protocols (TCP/UDP) tested
- [ ] Error handling and retry logic tested

### Compatibility Testing
- [ ] Test on Debian bookworm
- [ ] Test on Ubuntu 22.04
- [ ] Test on Ubuntu 24.04
- [ ] Verify with common IP cameras (if available)

## Version Management

### Update Version Numbers
- [ ] Update version in `Cargo.toml`
- [ ] Update version in `debian/changelog`
- [ ] Ensure version follows semantic versioning

### Update Documentation
- [ ] Update CHANGELOG.md with new features/fixes
- [ ] Update README.md if new features added
- [ ] Update debian/README.Debian if installation changes
- [ ] Verify all documentation is current

## Release Process

### Git Operations
- [ ] Create release branch: `git checkout -b release/v<VERSION>`
- [ ] Commit all changes
- [ ] Tag release: `git tag -a v<VERSION> -m "Release v<VERSION>"`
- [ ] Push branch and tags: `git push origin release/v<VERSION> --tags`

### Package Publishing
- [ ] Build final package: `./scripts/build-deb.sh`
- [ ] Run final validation: `./scripts/test-deb-package.sh`
- [ ] Upload package to releases (manual or via CI)
- [ ] Update package repository (if applicable)

### CI/CD Verification
- [ ] GitHub Actions pipeline completes successfully
- [ ] GitLab CI pipeline completes successfully
- [ ] All distribution tests pass
- [ ] Package artifacts are generated correctly

## Post-Release

### Verification
- [ ] Package is downloadable from release page
- [ ] Installation instructions work for new users
- [ ] Plugin appears in GStreamer registry after installation

### Communication
- [ ] Update project documentation
- [ ] Announce release on relevant channels
- [ ] Close related issues in issue tracker

### Cleanup
- [ ] Merge release branch to main
- [ ] Delete release branch if no longer needed
- [ ] Archive old release artifacts (if space limited)

## Rollback Plan

In case of critical issues discovered post-release:

1. **Immediate Response**
   - [ ] Remove package from download locations
   - [ ] Post notice about issues
   - [ ] Identify root cause

2. **Fix and Re-release**
   - [ ] Create hotfix branch from release tag
   - [ ] Fix critical issues
   - [ ] Increment patch version
   - [ ] Follow abbreviated release process
   - [ ] Test specifically for reported issues

3. **Communication**
   - [ ] Notify users of issue and resolution
   - [ ] Update documentation with any workarounds
   - [ ] Document lessons learned

## Release Notes Template

```markdown
# Release v<VERSION>

## New Features
- Feature descriptions here

## Bug Fixes  
- Bug fix descriptions here

## Improvements
- Improvement descriptions here

## Breaking Changes
- Any breaking changes here

## Installation
- Download: [gst-plugin-rtsp_<VERSION>.deb](link)
- Installation: `sudo dpkg -i gst-plugin-rtsp_<VERSION>.deb`

## Compatibility
- Debian: bookworm (12)
- Ubuntu: 22.04, 24.04
- GStreamer: 1.16+

## Known Issues
- Any known issues here

## Contributors
- Thank contributors here
```

## Validation Commands

Quick commands for common validation tasks:

```bash
# Build and test package
./scripts/build-deb.sh
./scripts/test-deb-package.sh

# Quick smoke test
cargo test -p gst-plugin-rtsp
sudo dpkg -i target/debian/*.deb
gst-inspect-1.0 rtspsrc
gst-launch-1.0 rtspsrc location=rtsp://test.url ! fakesink

# Clean up test installation
sudo dpkg -r gst-plugin-rtsp
```

---

**Release Manager**: Update this checklist as processes evolve.
**Last Updated**: Initial version for v0.14.0 series