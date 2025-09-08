# PRP-01: Project Structure and Cargo Workspace Setup

## Overview
Create the foundational project structure for the unified multi-stream recording and inference system as a standalone Rust application within the gst-plugins-rs workspace.

## Context
- Building on existing gst-plugins-rs patterns but as a separate application
- Will run as a long-lived systemd service
- Must support hot-reload configuration and plugin-based architecture
- Reference patterns from existing bins in the workspace

## Requirements
1. Create new workspace member under `apps/stream-manager/`
2. Setup Cargo.toml with all necessary dependencies
3. Create binary crate structure with lib.rs for shared components
4. Setup module structure for core components
5. Create placeholder modules for future PRPs

## Implementation Tasks
1. Add new workspace member to root Cargo.toml
2. Create apps/stream-manager directory structure
3. Setup Cargo.toml with dependencies:
   - gstreamer and related crates
   - tokio for async runtime
   - serde/toml for configuration
   - tracing for logging
   - actix-web for REST API
4. Create src/main.rs with basic tokio runtime
5. Create src/lib.rs with module declarations
6. Create module directories:
   - src/config/
   - src/pipeline/
   - src/recording/
   - src/health/
   - src/api/
   - src/storage/
7. Add README.md with project overview

## Validation Gates
```bash
# Build verification
cargo build --package stream-manager

# Check workspace integration
cargo check --workspace

# Verify structure
test -f apps/stream-manager/Cargo.toml
test -d apps/stream-manager/src/config
test -d apps/stream-manager/src/pipeline
```

## Dependencies
- None (first PRP)

## References
- Workspace structure: root Cargo.toml
- Binary patterns: Look at examples in generic/threadshare/examples/
- Module organization: utils/fallbackswitch/src/

## Success Metrics
- Clean compilation
- All module directories created
- Workspace member properly integrated

**Confidence Score: 9/10**