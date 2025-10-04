# RTSP Test Suite - Quick Reference

## 🚀 Most Common Commands

```bash
# Quick validation (1 min)
./run_tests.sh suite:smoke

# Full validation (10 min)
./run_tests.sh suite:full

# Single specific test
./run_tests.sh test:reconnection

# View results
./run_tests.sh results
```

## 📋 Test Suites

| Command | Time | Purpose |
|---------|------|---------|
| `suite:smoke` | 1m | Quick PR validation |
| `suite:transport` | 1.5m | UDP/TCP testing |
| `suite:resilience` | 3m | Reconnection & recovery |
| `suite:multistream` | 2m | Multi-camera scenarios |
| `suite:full` | 10m | Complete regression |

## 🔬 Individual Tests

| Command | What it tests |
|---------|---------------|
| `test:basic-udp` | UDP transport |
| `test:basic-tcp` | TCP transport |
| `test:reconnection` | Stream recovery |
| `test:periodic-restart` | Auto-restart |
| `test:dual-independent` | 2 independent streams |
| `test:dual-synced` | 2 synced streams (compositor) |
| `test:stream-isolation` | Failure isolation |
| `test:long-running` | 5-min stability |

## 🛠️ Utilities

```bash
./run_tests.sh setup      # Start test environment
./run_tests.sh cleanup    # Stop everything
./run_tests.sh build      # Build plugin only
./run_tests.sh results    # Show recent logs
```

## 📊 Reading Results

```
=== Test Results: basic-udp ===
  Total frames: 899        ← Frames received
  Errors: 0               ← Any GStreamer errors
  Warnings: 2             ← Warnings (usually OK)
  Status: PASSED          ← Overall result
```

- ✅ **PASSED** = No errors
- ❌ **FAILED** = Errors detected (check logs)

## 🔧 Options

```bash
# Custom duration
./run_tests.sh test:basic-udp --duration 60

# Keep environment running
./run_tests.sh suite:smoke --no-cleanup

# Custom output dir
./run_tests.sh suite:full --results-dir /tmp/tests
```

## 🐛 Troubleshooting

```bash
# Something stuck?
./run_tests.sh cleanup

# Check what's running
pgrep -a mediamtx
pgrep -a ffmpeg

# View latest log
ls -t test-results/*.log | head -1 | xargs cat
```

## 📁 Files

- `run_tests.sh` - Main test runner
- `test-results/` - Test outputs (auto-created)
- `TEST_SUITE.md` - Full documentation
- `mediamtx.yml` - RTSP server config

## ⚡ CI/CD Integration

```bash
# In CI pipeline
./run_tests.sh suite:smoke || exit 1
```

## 🎯 Typical Workflow

### Development
```bash
# Make changes to rtspsrc
vim net/rtsp/src/rtspsrc/imp.rs

# Quick test
./run_tests.sh suite:smoke

# If passed, run full suite
./run_tests.sh suite:full
```

### Pre-commit
```bash
./run_tests.sh suite:smoke
```

### Pre-release
```bash
./run_tests.sh suite:full
```

## 💡 Tips

- Run `suite:smoke` frequently during development
- Use `--no-cleanup` to debug test failures
- Check `test-results/` logs for detailed output
- Tests need sudo for iptables (stream-isolation)
- All tests are idempotent (safe to re-run)

## 📞 Getting Help

```bash
./run_tests.sh help    # Full help
./run_tests.sh         # Same as help
```

For detailed documentation, see `TEST_SUITE.md`
