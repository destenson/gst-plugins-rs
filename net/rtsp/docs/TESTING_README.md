# RTSP Plugin Testing - Quick Start

## Docker Testing (Recommended)

**No sudo required! Includes all dependencies and pre-built plugin.**

### Setup (first time)
```bash
# Build plugin + Docker image
./scripts/docker-test.sh build
```

### Run Tests
```bash
# Quick validation (1 min)
./scripts/docker-test.sh run suite:smoke

# Full test suite (10 min)
./scripts/docker-test.sh run suite:full

# Individual test
./scripts/docker-test.sh run test:basic-udp

# Interactive shell for debugging
./scripts/docker-test.sh shell
```

### After Code Changes
```bash
# Rebuild plugin + Docker image
./scripts/docker-test.sh build

# Run tests with updated plugin
./scripts/docker-test.sh run suite:smoke
```

## Host Testing (Alternative)

**Requires: sudo, mediamtx, ffmpeg, cargo**

```bash
# Build and run all tests
./scripts/run_tests.sh suite:full

# Run specific test
./scripts/run_tests.sh test:reconnection
```

## Documentation

- **[Docker Testing Guide](docs/DOCKER_TESTING.md)** - Complete Docker setup and usage
- **[Docker Build Notes](docs/DOCKER_BUILD_NOTES.md)** - Technical details about Docker build
- **[Test Suite Reference](docs/TEST_SUITE.md)** - All available tests and CI/CD examples
- **[Testing Quick Reference](docs/TESTING_QUICKREF.md)** - Command cheat sheet
- **[Dual Stream Testing](docs/DUAL_STREAM_TESTING.md)** - Multi-stream examples and architecture

## Examples

```bash
# Run dual-stream tests
./scripts/docker-test.sh run test:dual-synced
./scripts/docker-test.sh run test:dual-independent

# Run resilience tests (reconnection, restart, isolation)
./scripts/docker-test.sh run suite:resilience

# View test results
./scripts/docker-test.sh logs
cat test-results/basic-udp-*.log
```

## Test Results

All test logs are saved to `test-results/` with timestamps:
- `basic-udp-20231004-120000.log` - Test output
- `mediamtx-20231004-120000.log` - RTSP server logs
- `ffmpeg-test-h264-20231004-120000.log` - Stream generator logs

## CI/CD Integration

See [TEST_SUITE.md](docs/TEST_SUITE.md) for GitHub Actions, GitLab CI, and Jenkins examples.

Quick GitHub Actions example:
```yaml
- name: Run RTSP Tests
  run: |
    cd net/rtsp
    ./docker-test.sh build
    ./docker-test.sh run suite:smoke
```
