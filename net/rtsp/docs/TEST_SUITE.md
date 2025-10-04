# RTSP Test Suite

Automated validation framework for rtspsrc2 with high-level commands for running comprehensive tests.

## Quick Start

```bash
# Run smoke tests (fastest validation)
./run_tests.sh suite:smoke

# Run full test suite
./run_tests.sh suite:full

# Run specific test
./run_tests.sh test:reconnection

# Clean up when done
./run_tests.sh cleanup
```

## Test Suites

### `suite:smoke` - Quick Validation (~1 minute)
Fastest way to validate basic functionality:
- ✅ Basic UDP transport
- ✅ Dual independent streams

**Use when:** Quick PR validation, pre-commit checks

### `suite:transport` - Transport Tests (~1.5 minutes)
Validates different transport protocols:
- ✅ UDP transport
- ✅ TCP transport

**Use when:** Testing protocol changes, transport layer updates

### `suite:resilience` - Resilience Tests (~3 minutes)
Tests recovery and reconnection:
- ✅ Stream reconnection after failure
- ✅ Periodic source restart
- ✅ Stream isolation (one fails, others continue)

**Use when:** Testing error handling, reconnection logic

### `suite:multistream` - Multi-Stream Tests (~2 minutes)
Tests multiple concurrent streams:
- ✅ Independent dual streams
- ✅ Synchronized dual streams (compositor)
- ✅ Stream isolation

**Use when:** Testing multi-camera scenarios, synchronization

### `suite:full` - Complete Test Suite (~10 minutes)
All tests including long-running stability:
- All of the above
- ✅ 5-minute stability test

**Use when:** Pre-release validation, comprehensive regression testing

## Individual Tests

| Test | Duration | Description |
|------|----------|-------------|
| `test:basic-udp` | 30s | Basic UDP transport with single stream |
| `test:basic-tcp` | 30s | Basic TCP transport with single stream |
| `test:reconnection` | 30s | Stream interruption and recovery |
| `test:periodic-restart` | 45s | Automatic periodic source restart (3 cycles) |
| `test:dual-independent` | 30s | Two independent streams, separate sinks |
| `test:dual-synced` | 30s | Two synchronized streams with compositor |
| `test:stream-isolation` | 30s | Verify one stream failure doesn't affect others |
| `test:long-running` | 5min | Long-term stability test |

## Command Options

```bash
# Override test duration
./run_tests.sh test:basic-udp --duration 60

# Keep environment running after test (no cleanup)
./run_tests.sh suite:smoke --no-cleanup

# Custom results directory
./run_tests.sh suite:full --results-dir /tmp/rtsp-tests
```

## Utilities

```bash
# Setup test environment (manual control)
./run_tests.sh setup
# ... run tests manually ...
./run_tests.sh cleanup

# Just build the plugin
./run_tests.sh build

# View recent test results
./run_tests.sh results
```

## Test Results

All test outputs are saved to `test-results/` with timestamps:

```
test-results/
├── basic-udp-20251004-143022.log
├── dual-independent-20251004-143055.log
├── mediamtx-20251004-143020.log
├── ffmpeg-test-h264-20251004-143021.log
└── ...
```

Each test log contains:
- Full GStreamer debug output
- Frame statistics
- Error and warning messages
- Reconnection events

## Result Analysis

Tests automatically analyze results and report:
- ✅ **PASSED** - No errors, expected frame count
- ⚠️ **FAILED** - Errors detected in logs
- Frame counts and average FPS
- Error/warning counts
- Reconnection attempts

Example output:
```
=== Test Results: basic-udp ===
  Total frames: 899
  Errors: 0
  Warnings: 2
  Status: PASSED
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: RTSP Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y ffmpeg
          wget https://github.com/bluenviron/mediamtx/releases/download/v1.3.0/mediamtx_v1.3.0_linux_amd64.tar.gz
          tar -xzf mediamtx_v1.3.0_linux_amd64.tar.gz
          sudo mv mediamtx /usr/local/bin/
      
      - name: Run smoke tests
        run: ./net/rtsp/run_tests.sh suite:smoke
      
      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: test-results
          path: net/rtsp/test-results/
```

### Jenkins Example

```groovy
pipeline {
    agent any
    stages {
        stage('Smoke Tests') {
            steps {
                sh './net/rtsp/run_tests.sh suite:smoke'
            }
        }
        stage('Full Tests') {
            when {
                branch 'main'
            }
            steps {
                sh './net/rtsp/run_tests.sh suite:full'
            }
        }
    }
    post {
        always {
            archiveArtifacts 'net/rtsp/test-results/**/*.log'
        }
    }
}
```

## Requirements

- **mediamtx** - RTSP server for testing
- **ffmpeg** - Test stream publisher
- **cargo/rust** - Build system
- **nc** (netcat) - Port checking
- **iptables** - For network simulation tests (requires sudo)

### Installation

```bash
# Ubuntu/Debian
sudo apt-get install ffmpeg netcat-openbsd iptables

# Install mediamtx
wget https://github.com/bluenviron/mediamtx/releases/download/v1.3.0/mediamtx_v1.3.0_linux_amd64.tar.gz
tar -xzf mediamtx_v1.3.0_linux_amd64.tar.gz
sudo mv mediamtx /usr/local/bin/

# Verify
mediamtx --version
ffmpeg -version
```

## Troubleshooting

### Test hangs or doesn't start

```bash
# Check if mediamtx is already running
pgrep mediamtx

# Kill existing processes
./run_tests.sh cleanup

# Try again
./run_tests.sh test:basic-udp
```

### Port 8554 already in use

```bash
# Find what's using the port
sudo lsof -i :8554

# Kill it
sudo kill <PID>
```

### iptables permission denied

```bash
# Some tests need sudo for iptables
# Run with sudo or skip those tests:
sudo ./run_tests.sh test:stream-isolation
```

### No frames received

Check the test logs:
```bash
# View most recent test log
ls -lt test-results/*.log | head -1 | awk '{print $NF}' | xargs cat

# Check for errors
grep -i error test-results/*.log
```

## Test Development

### Adding a New Test

1. Create test function in `run_tests.sh`:
```bash
test_my_feature() {
    log_test_start "My Feature Test"
    
    local stream_pid=$(start_test_stream "test-myfeature" "testsrc")
    
    run_test_command "my-feature" ${DEFAULT_TEST_DURATION} \
        "cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
         --url rtsp://127.0.0.1:8554/test-myfeature"
    
    stop_test_stream "test-myfeature"
}
```

2. Add to suite if appropriate:
```bash
suite_full() {
    # ... existing tests ...
    test_my_feature || ((failed++))
}
```

3. Add CLI command in `main()`:
```bash
test:my-feature)
    setup_environment
    test_my_feature
    [ -z "$NO_CLEANUP" ] && cleanup
    ;;
```

### Test Best Practices

- ✅ Tests should be idempotent (can run multiple times)
- ✅ Always cleanup resources (use trap EXIT)
- ✅ Log everything to timestamped files
- ✅ Use meaningful test names
- ✅ Keep tests focused (one thing per test)
- ✅ Make tests deterministic (avoid timing races)

## Performance Benchmarks

Typical test durations on modern hardware:

| Suite | Duration | Tests |
|-------|----------|-------|
| smoke | ~1 min | 2 tests |
| transport | ~1.5 min | 2 tests |
| resilience | ~3 min | 3 tests |
| multistream | ~2 min | 3 tests |
| full | ~10 min | 8 tests |

## Continuous Testing

Run tests continuously during development:

```bash
# Watch mode (requires entr)
find . -name '*.rs' | entr -c ./run_tests.sh suite:smoke

# Cron job (nightly tests)
0 2 * * * cd /path/to/repo && ./net/rtsp/run_tests.sh suite:full > /tmp/nightly-$(date +\%Y\%m\%d).log 2>&1
```

## Coverage

What the test suite validates:
- ✅ UDP and TCP transport
- ✅ Reconnection after network failure
- ✅ Periodic source restart
- ✅ Multi-stream independence
- ✅ Frame synchronization (compositor)
- ✅ Stream isolation
- ✅ Long-term stability
- ✅ Error handling
- ✅ Resource cleanup

What's NOT yet covered (future work):
- ❌ Authentication (RTSP auth)
- ❌ TLS/RTSPS
- ❌ Multicast
- ❌ Different codecs (only H264 tested)
- ❌ Audio streams
- ❌ Metadata streams
- ❌ Performance/latency metrics
