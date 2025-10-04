# Docker Build Notes

## Overview

The Docker testing environment now includes the **pre-built RTSP plugin** installed system-wide in the container, eliminating the need to rebuild during test runs.

## Architecture

### Build Process

```
Host Machine:
  1. Build plugin in release mode
     cargo build --release -p gst-plugin-rtsp
     → target/release/libgstrsrtsp.so
  
  2. Build Docker image with plugin
     docker build (from PROJECT_ROOT)
     → Copies libgstrsrtsp.so to container
     → Installs to /usr/lib/x86_64-linux-gnu/gstreamer-1.0/

Container Runtime:
  3. Plugin is available system-wide
     gst-inspect-1.0 rsrtsp ✅
     No build step needed!
```

### File Layout

```
PROJECT_ROOT/
├── target/release/
│   └── libgstrsrtsp.so          # Built on host before Docker build
│
├── net/rtsp/
│   ├── Dockerfile.test           # COPY plugin → system location
│   ├── docker-test.sh            # Builds plugin, then Docker image
│   └── run_tests_docker.sh       # No build step (plugin pre-installed)
```

## Key Changes

### 1. Dockerfile.test

**Added:**
```dockerfile
# Copy the pre-built RTSP plugin to system GStreamer plugin directory
COPY target/release/libgstrsrtsp.so /usr/lib/x86_64-linux-gnu/gstreamer-1.0/libgstrsrtsp.so
RUN chmod 644 /usr/lib/x86_64-linux-gnu/gstreamer-1.0/libgstrsrtsp.so
```

**Removed:**
```dockerfile
# No longer needed - plugin installed system-wide
ENV GST_PLUGIN_PATH=/workspace/target/debug
```

**Why:**
- Plugin is installed to standard GStreamer plugin directory
- No need to override `GST_PLUGIN_PATH`
- GStreamer automatically discovers plugins in system locations

### 2. docker-test.sh

**Added build_image():**
```bash
# Build the plugin first (release mode for Docker)
cargo build --release -p gst-plugin-rtsp

# Build Docker image with the plugin
docker build -t gst-rtsp-test -f Dockerfile.test $PROJECT_ROOT
```

**Why:**
- Ensures plugin is built before Docker image
- Uses release mode (optimized, smaller binary)
- Build context is PROJECT_ROOT (to access target/release/)

### 3. run_tests_docker.sh

**Removed:**
```bash
build_plugin() {
    cargo build -p gst-plugin-rtsp ...
}
```

**Added verification:**
```bash
setup_environment() {
    # Verify plugin is installed
    if ! gst-inspect-1.0 rsrtsp >/dev/null 2>&1; then
        log_error "RTSP plugin not found"
        exit 1
    fi
    ...
}
```

**Why:**
- No build step needed inside container
- Plugin already installed during image build
- Faster test startup (no compilation overhead)

## Benefits

### Performance
- ✅ **Faster test runs** - No compilation during tests
- ✅ **Optimized plugin** - Built in release mode
- ✅ **Smaller image** - No debug symbols in plugin

### Reliability
- ✅ **Consistent environment** - Same plugin binary every test
- ✅ **No build failures** - Build happens once during image creation
- ✅ **Plugin always available** - Installed system-wide

### Simplicity
- ✅ **No cargo/rust needed at runtime** - Container could be slimmed further
- ✅ **Standard GStreamer setup** - Plugin in expected location
- ✅ **Easy verification** - `gst-inspect-1.0 rsrtsp` works

## Usage

### First Time Setup

```bash
cd net/rtsp

# Build plugin + Docker image (includes plugin)
./docker-test.sh build
```

This will:
1. Compile `libgstrsrtsp.so` in release mode
2. Build Docker image with plugin pre-installed
3. Verify plugin is accessible via `gst-inspect-1.0`

### Running Tests

```bash
# Run smoke tests (plugin already installed, no build step!)
./docker-test.sh run suite:smoke

# Run all tests
./docker-test.sh run suite:full
```

Inside the container:
- Plugin is at `/usr/lib/x86_64-linux-gnu/gstreamer-1.0/libgstrsrtsp.so`
- GStreamer finds it automatically
- `gst-inspect-1.0 rsrtsp` shows plugin info
- Examples work immediately (no build wait)

### Rebuilding After Changes

If you modify the RTSP plugin source code:

```bash
# Rebuild Docker image (will rebuild plugin first)
./docker-test.sh build

# Then run tests with updated plugin
./docker-test.sh run suite:smoke
```

## Troubleshooting

### Plugin not found in container

```bash
# Check if plugin binary exists on host
ls -lh target/release/libgstrsrtsp.so

# If missing, build manually
cargo build --release -p gst-plugin-rtsp

# Then rebuild Docker image
./docker-test.sh build
```

### Plugin version mismatch

```bash
# Verify plugin in container
./docker-test.sh shell
gst-inspect-1.0 rsrtsp | grep Version

# Check plugin file
ls -lh /usr/lib/x86_64-linux-gnu/gstreamer-1.0/libgstrsrtsp.so
```

### Build context errors

```bash
# Dockerfile.test must be in net/rtsp/
# But build context is PROJECT_ROOT
docker build -f net/rtsp/Dockerfile.test PROJECT_ROOT

# This allows: COPY target/release/libgstrsrtsp.so ...
```

## Technical Details

### GStreamer Plugin Discovery

GStreamer searches for plugins in:
1. `/usr/lib/x86_64-linux-gnu/gstreamer-1.0/` ← We install here
2. `$GST_PLUGIN_PATH` (if set)
3. `~/.gstreamer-1.0/plugins/`

By installing to system location, we get:
- Automatic discovery
- Standard installation
- No environment variable needed

### Build Context

```bash
docker build -f net/rtsp/Dockerfile.test PROJECT_ROOT
              ├─ Dockerfile location
              └─ Build context (for COPY commands)
```

The build context must be PROJECT_ROOT because:
- `COPY target/release/libgstrsrtsp.so` is relative to build context
- Plugin is built at `PROJECT_ROOT/target/release/`
- Can't copy files outside build context

### Release vs Debug

| Build Type | Size | Performance | Use Case |
|------------|------|-------------|----------|
| Debug | ~50MB | Slow | Development |
| Release | ~5MB | Fast | Docker/Production |

We use **release mode** for Docker because:
- Tests run faster
- Smaller image size
- No debugging inside container (use host for that)

## Migration from Previous Version

### Old Workflow
```bash
./docker-test.sh run suite:smoke
  → Container starts
  → Runs cargo build (5-10 seconds)
  → Runs tests
```

### New Workflow
```bash
./docker-test.sh build              # Once, includes plugin
./docker-test.sh run suite:smoke    # Fast, no build step!
  → Container starts
  → Verifies plugin exists
  → Runs tests immediately
```

### Time Savings

Per test run:
- Old: 5-10s build + test time
- New: 0s build + test time

For 10 test runs:
- Old: ~60-100s wasted on rebuilds
- New: ~0s (built once in image)

## Future Improvements

### Multi-arch Support

The Dockerfile currently hardcodes `x86_64-linux-gnu`. For ARM support:

```dockerfile
# Auto-detect architecture
RUN ARCH=$(dpkg --print-architecture) && \
    if [ "$ARCH" = "amd64" ]; then \
        PLUGIN_DIR="/usr/lib/x86_64-linux-gnu/gstreamer-1.0"; \
    elif [ "$ARCH" = "arm64" ]; then \
        PLUGIN_DIR="/usr/lib/aarch64-linux-gnu/gstreamer-1.0"; \
    fi && \
    cp /tmp/libgstrsrtsp.so $PLUGIN_DIR/
```

### Multi-stage Build

Separate build and runtime stages:

```dockerfile
# Stage 1: Build
FROM rust:1.75 AS builder
COPY . /src
RUN cargo build --release -p gst-plugin-rtsp

# Stage 2: Runtime
FROM debian:bookworm-slim
COPY --from=builder /src/target/release/libgstrsrtsp.so /usr/lib/.../
```

Benefits:
- Smaller image (~500MB vs ~2GB)
- No Rust toolchain in runtime
- Faster container startup

### CI/CD Caching

```yaml
# GitHub Actions
- name: Cache plugin binary
  uses: actions/cache@v3
  with:
    path: target/release/libgstrsrtsp.so
    key: ${{ runner.os }}-plugin-${{ hashFiles('net/rtsp/src/**') }}
```

## Summary

The Docker testing environment now:
1. **Builds plugin on host** (release mode, optimized)
2. **Includes plugin in image** (system-wide installation)
3. **Skips build during tests** (instant startup)

This provides:
- Faster test runs
- More reliable environment
- Standard GStreamer setup
- Better CI/CD integration
