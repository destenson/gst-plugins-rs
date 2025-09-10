# PRP-15: Configuration Hot-Reload System

## Overview
Implement configuration hot-reload capability allowing runtime updates without service restart.

## Context
- Configuration changes shouldn't require restart
- Must validate changes before applying
- Some settings cannot be changed at runtime
- Need to notify components of changes

## Requirements
1. Implement file watching for config
2. Create configuration diff system
3. Validate changes before applying
4. Apply changes to running system
5. Notify affected components

## Implementation Tasks
1. Create src/config/reload.rs module
2. Implement file watcher:
   - Use notify crate
   - Watch config file path
   - Debounce rapid changes
   - Trigger reload on change
3. Create config diff system:
   - Compare old vs new config
   - Identify changed sections
   - Classify changes (runtime vs restart)
4. Add validation layer:
   - Check if changes are runtime-applicable
   - Validate new values
   - Reject invalid changes
   - Log rejected changes
5. Implement change application:
   - Update global config
   - Apply to StreamManager
   - Update existing streams
   - Notify components
6. Define reload restrictions:
   - Port changes require restart
   - Storage paths validated
   - Stream defaults apply to new streams only
7. Add reload status API endpoint

## Validation Gates
```bash
# Test config reload
cargo test --package stream-manager config::reload::tests

# Verify file watching
cargo test config_file_watch

# Check change application
cargo test config_hot_reload
```

## Dependencies
- PRP-02: Configuration management
- PRP-09: StreamManager for updates

## References
- Notify crate: https://github.com/notify-rs/notify
- Hot reload patterns: Search for "reload" in config examples
- Diff algorithms: Standard diff patterns

## Success Metrics
- Config changes detected
- Valid changes applied without restart
- Invalid changes rejected with logs
- Components notified of changes

**Confidence Score: 7/10**