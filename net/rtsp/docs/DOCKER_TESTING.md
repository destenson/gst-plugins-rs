# Docker-Based RTSP Testing

This document describes how to use the Docker-based testing environment for the RTSP plugin. The Docker approach provides:

- ✅ **No sudo required** - All operations run as non-root with capabilities
- ✅ **Isolated environment** - Clean, reproducible test environment
- ✅ **All dependencies included** - GStreamer, FFmpeg, mediamtx, etc.
- ✅ **Cross-platform** - Works on any system with Docker
- ✅ **CI/CD ready** - Easy integration into automated pipelines

## Quick Start

### 1. Build the plugin and Docker image (first time only)

```bash
cd net/rtsp

# Build plugin in release mode + create Docker image with plugin pre-installed
./docker-test.sh build
```

This creates a Docker image with:
- Rust toolchain  
- GStreamer 1.0 (runtime + development)
- FFmpeg (for test streams)
- mediamtx (RTSP server)
- Network utilities (iptables, iproute2, netcat)
- **Pre-built RTSP plugin (libgstrsrtsp.so) installed system-wide**

The build process:
1. Compiles `target/release/libgstrsrtsp.so` on host (release mode, ~26MB)
2. Creates Docker image with plugin copied to `/usr/lib/x86_64-linux-gnu/gstreamer-1.0/`
3. Plugin is available immediately when container starts (no build step during tests!)

### 2. Run tests

```bash
# Quick smoke test (1 minute)
./docker-test.sh run suite:smoke

# Full test suite (10 minutes)
./docker-test.sh run suite:full

# Individual test
./docker-test.sh run test:basic-udp

# Custom duration
./docker-test.sh run suite:smoke --duration 60
```

### 3. Interactive debugging

```bash
# Open shell in container
./docker-test.sh shell

# Then run commands manually
./run_tests_docker.sh suite:smoke
cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup
```

## Architecture

### Docker Components

```
┌─────────────────────────────────────────┐
│  Host System                            │
│  ┌───────────────────────────────────┐  │
│  │ net/rtsp/                         │  │
│  │   ├── docker-test.sh (wrapper)   │  │
│  │   ├── Dockerfile.test            │  │
│  │   └── run_tests_docker.sh        │  │
│  └───────────────────────────────────┘  │
│            ▼                             │
│  ┌───────────────────────────────────┐  │
│  │ Docker Container                  │  │
│  │   ├── Rust + Cargo               │  │
│  │   ├── GStreamer 1.0               │  │
│  │   ├── FFmpeg                      │  │
│  │   ├── mediamtx                    │  │
│  │   └── Network tools               │  │
│  │                                   │  │
│  │   Capabilities:                   │  │
│  │   - CAP_NET_ADMIN (ip commands)   │  │
│  │   - CAP_NET_RAW (iptables)        │  │
│  │                                   │  │
│  │   User: tester (non-root)         │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### Key Differences from Host Testing

| Aspect | Host (`run_tests.sh`) | Docker (`docker-test.sh`) |
|--------|----------------------|---------------------------|
| **sudo** | Required for ip/iptables | Not required (capabilities) |
| **Dependencies** | Must install manually | All included in image |
| **Isolation** | Affects host system | Isolated container |
| **Cleanup** | Manual sudo cleanup | Automatic on container exit |
| **Portability** | Host-dependent | Works anywhere with Docker |
| **CI/CD** | Complex setup | Simple docker run |

## Available Commands

### docker-test.sh (wrapper)

```bash
./docker-test.sh build              # Build Docker image
./docker-test.sh run <test>         # Run tests in container
./docker-test.sh shell              # Interactive shell
./docker-test.sh clean              # Remove image/containers
./docker-test.sh logs               # Show recent test results
```

### Test Suites (inside container)

All the same tests as `run_tests.sh`:

```bash
# Quick validation (1 min)
./docker-test.sh run suite:smoke

# Transport tests (1.5 min)
./docker-test.sh run suite:transport

# Resilience tests (3 min)
./docker-test.sh run suite:resilience

# Multi-stream tests (2 min)
./docker-test.sh run suite:multistream

# Complete suite (10 min)
./docker-test.sh run suite:full
```

### Individual Tests

```bash
./docker-test.sh run test:basic-udp
./docker-test.sh run test:basic-tcp
./docker-test.sh run test:reconnection
./docker-test.sh run test:periodic-restart
./docker-test.sh run test:dual-independent
./docker-test.sh run test:dual-synced
./docker-test.sh run test:stream-isolation
./docker-test.sh run test:long-running
```

## How It Works

### 1. Image Building

The `Dockerfile.test` creates a minimal image with:

```dockerfile
# Base: Rust official image (Debian Bookworm)
FROM rust:1.75-bookworm

# Install all system dependencies
RUN apt-get install -y \
    libgstreamer1.0-dev \
    gstreamer1.0-plugins-* \
    ffmpeg \
    network-tools \
    ...

# Install mediamtx from GitHub releases
RUN wget mediamtx && mv /usr/local/bin/

# Create non-root user
RUN useradd -m tester

# Grant network capabilities (no sudo needed)
RUN setcap cap_net_admin,cap_net_raw+eip /usr/sbin/ip
RUN setcap cap_net_admin,cap_net_raw+eip /usr/sbin/iptables
```

### 2. Running Tests

```bash
docker run --rm \
    --cap-add=NET_ADMIN \      # Allow ip commands
    --cap-add=NET_RAW \        # Allow iptables
    -v $PROJECT_ROOT:/workspace \  # Mount source code
    gst-rtsp-test \
    ./run_tests_docker.sh suite:smoke
```

**Key points:**
- Volume mount preserves source code and test results
- Capabilities grant network permissions without sudo
- Non-root user (tester) runs all commands
- Container auto-removes after tests complete

### 3. No sudo Required

The Docker version replaces:

```bash
# Host version (requires sudo)
sudo ip addr add 127.0.0.2/8 dev lo
sudo iptables -A INPUT -s 127.0.0.2 -j DROP
```

With:

```bash
# Docker version (no sudo)
ip addr add 127.0.0.2/8 dev lo
iptables -A INPUT -s 127.0.0.2 -j DROP
```

This works because:
1. Container has `CAP_NET_ADMIN` and `CAP_NET_RAW` capabilities
2. Binaries have `setcap` applied at build time
3. User `tester` can execute network commands without root

## CI/CD Integration

### GitHub Actions

```yaml
name: RTSP Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Build test image
        run: |
          cd net/rtsp
          ./docker-test.sh build
      
      - name: Run smoke tests
        run: |
          cd net/rtsp
          ./docker-test.sh run suite:smoke
      
      - name: Run full suite
        if: github.event_name == 'push' && github.ref == 'refs/heads/main'
        run: |
          cd net/rtsp
          ./docker-test.sh run suite:full
      
      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: test-results
          path: net/rtsp/test-results/
```

### GitLab CI

```yaml
rtsp-tests:
  image: docker:latest
  services:
    - docker:dind
  script:
    - cd net/rtsp
    - ./docker-test.sh build
    - ./docker-test.sh run suite:smoke
  artifacts:
    paths:
      - net/rtsp/test-results/
    when: always
```

### Jenkins

```groovy
pipeline {
    agent any
    stages {
        stage('Build') {
            steps {
                dir('net/rtsp') {
                    sh './docker-test.sh build'
                }
            }
        }
        stage('Test') {
            steps {
                dir('net/rtsp') {
                    sh './docker-test.sh run suite:full'
                }
            }
        }
    }
    post {
        always {
            archiveArtifacts artifacts: 'net/rtsp/test-results/**'
        }
    }
}
```

## Troubleshooting

### Build fails with permission errors

```bash
# Ensure Docker daemon is running
sudo systemctl start docker

# Add user to docker group (logout/login required)
sudo usermod -aG docker $USER
```

### Container can't modify network

```bash
# Verify capabilities are granted
./docker-test.sh shell
capsh --print | grep net_admin

# Should see: cap_net_admin,cap_net_raw
```

### Tests fail but host tests work

```bash
# Check if volume mount is working
./docker-test.sh shell
ls -la /workspace
cd /workspace/net/rtsp
cargo build -p gst-plugin-rtsp
```

### mediamtx fails to start

```bash
# Check if port 8554 is available
./docker-test.sh shell
nc -z localhost 8554
netstat -tlnp | grep 8554
```

### FFmpeg streams don't start

```bash
# Test FFmpeg manually
./docker-test.sh shell
ffmpeg -f lavfi -i testsrc=size=640x480:rate=30 -t 5 test.mp4
# If this fails, FFmpeg is not properly installed
```

## Performance Considerations

### Image Size

The test image is ~2GB due to:
- Rust toolchain (~1GB)
- GStreamer plugins (~500MB)
- FFmpeg (~200MB)
- System libraries (~300MB)

To reduce size:
- Use multi-stage build (separate build/runtime images)
- Remove unnecessary plugins
- Use Alpine Linux (harder, GStreamer compatibility)

### Build Time

First build: ~5-10 minutes (downloads dependencies)
Subsequent builds: ~30 seconds (cached layers)

### Test Execution

Docker overhead: ~1-2 seconds per test suite
Network performance: Identical to host (uses host network stack)

## Advanced Usage

### Custom mediamtx config

```bash
# Edit mediamtx.yml
vim mediamtx.yml

# Run with custom config
./docker-test.sh run suite:smoke
# (automatically picks up mediamtx.yml from volume)
```

### Debugging with logs

```bash
# Run test
./docker-test.sh run test:basic-udp

# View logs
cat test-results/basic-udp-*.log
cat test-results/mediamtx-*.log
cat test-results/ffmpeg-test-h264-*.log
```

### Interactive development

```bash
# Open shell
./docker-test.sh shell

# Build and test manually
cargo build -p gst-plugin-rtsp
mediamtx mediamtx.yml &
ffmpeg -f lavfi -i testsrc=size=640x480:rate=30 -f rtsp rtsp://127.0.0.1:8554/test &
cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- --url rtsp://127.0.0.1:8554/test
```

### Run specific duration

```bash
./docker-test.sh run suite:smoke --duration 120
```

### Keep container for debugging

```bash
# Run without auto-cleanup
docker run -it \
    --cap-add=NET_ADMIN \
    --cap-add=NET_RAW \
    -v $(pwd)/../..:/workspace \
    -w /workspace/net/rtsp \
    gst-rtsp-test \
    bash

# Then run tests manually
./run_tests_docker.sh suite:smoke
```

## Comparison: Host vs Docker

### When to use host testing

- Quick local development
- Debugging specific issues
- Custom GStreamer builds
- Performance benchmarking (eliminate Docker overhead)

### When to use Docker testing

- CI/CD pipelines ✅
- Clean test environment ✅
- Reproducible results ✅
- No system contamination ✅
- Multi-platform testing ✅
- Automated regression testing ✅

## Migration Guide

If you're currently using `run_tests.sh`, switch to Docker:

```bash
# Old way (host)
./run_tests.sh suite:smoke

# New way (Docker)
./docker-test.sh build      # First time only
./docker-test.sh run suite:smoke
```

All test names and options remain the same!

## Summary

The Docker-based testing provides:

1. **Zero sudo** - Runs as non-root with capabilities
2. **Complete isolation** - No host system pollution
3. **Reproducibility** - Same environment every time
4. **Portability** - Works on any Docker-capable system
5. **CI/CD ready** - Simple integration
6. **Easy cleanup** - Container auto-removes

**Recommended workflow:**
- Use Docker for CI/CD and regression testing
- Use host for quick local development iterations
- Both produce identical test results
