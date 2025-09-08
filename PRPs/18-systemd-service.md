# PRP-18: Systemd Service Integration

## Overview
Implement systemd service integration with proper daemon behavior, status reporting, and watchdog support.

## Context
- Must run as system service
- Need proper startup/shutdown sequencing
- Should report status to systemd
- Must support systemd watchdog

## Requirements
1. Create systemd service unit file
2. Implement sd_notify protocol
3. Add watchdog support
4. Handle systemd signals
5. Setup logging integration

## Implementation Tasks
1. Create systemd/stream-manager.service file:
   - Type=notify for sd_notify
   - Restart=always for resilience
   - WatchdogSec for monitoring
   - User/Group configuration
   - Environment variables
2. Implement sd_notify in src/service/mod.rs:
   - READY=1 when initialized
   - STATUS= updates
   - WATCHDOG=1 heartbeats
   - STOPPING=1 on shutdown
3. Add watchdog handler:
   - Get interval from WATCHDOG_USEC
   - Send periodic heartbeats
   - Include health check
4. Handle systemd signals:
   - SIGTERM for graceful shutdown
   - SIGHUP for reload
   - SIGUSR1 for status dump
5. Setup journal logging:
   - Use tracing-journald
   - Structured logging
   - Priority levels
6. Add install/uninstall scripts:
   - Copy service file
   - Run systemctl daemon-reload
   - Enable/start service
7. Create systemd socket activation support

## Validation Gates
```bash
# Test service file validity
systemd-analyze verify systemd/stream-manager.service

# Check sd_notify integration
cargo test --package stream-manager service::tests

# Verify signal handling
cargo test service_signals
```

## Dependencies
- PRP-09: StreamManager for service core
- PRP-15: Config reload for SIGHUP

## References
- sd_notify: https://www.freedesktop.org/software/systemd/man/sd_notify.html
- systemd-rs: https://github.com/systemd/systemd-rs
- Service examples: Standard systemd service patterns

## Success Metrics
- Service starts via systemd
- Status reported correctly
- Watchdog prevents hangs
- Clean shutdown on stop

**Confidence Score: 8/10**