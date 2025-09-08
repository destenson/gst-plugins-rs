# PRP-02: Configuration Management Layer

## Overview
Implement a robust configuration system with TOML parsing, validation, defaults, and runtime reload capabilities.

## Context
- Configuration drives all aspects of the application
- Must support partial updates for hot-reload
- Need both global and per-stream configurations
- Should validate configuration against system capabilities

## Requirements
1. Define configuration structures using serde
2. Implement TOML file parsing with defaults
3. Add configuration validation logic
4. Create configuration merge capabilities for updates
5. Setup file watching for hot-reload

## Implementation Tasks
1. Create src/config/mod.rs with main Config struct
2. Define nested configuration structures:
   - AppConfig (global settings)
   - RecordingConfig (recording defaults)
   - InferenceConfig (inference settings)
   - StorageConfig (disk management)
   - StreamDefaultConfig (default stream settings)
3. Implement Default trait for all config structs
4. Add validation methods that check:
   - Path existence and permissions
   - Numeric bounds (timeouts, sizes)
   - Hardware capabilities (GPU availability)
5. Create ConfigManager with:
   - Load from file method
   - Merge partial updates
   - Validate against system
6. Setup notify crate for file watching
7. Add example config.toml in project root

## Validation Gates
```bash
# Test configuration loading
cargo test --package stream-manager config::tests

# Verify example config
test -f apps/stream-manager/config.example.toml

# Check serde derives compile
cargo check --package stream-manager
```

## Dependencies
- PRP-01: Project structure must exist

## References
- Serde patterns: Check any Cargo.toml parsing in workspace
- Validation approaches: https://docs.rs/validator/latest/validator/
- Hot reload pattern: https://github.com/notify-rs/notify

## Success Metrics
- Config loads from TOML file
- Validation catches invalid configs
- File changes trigger reload events

**Confidence Score: 9/10**