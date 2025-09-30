# Automated PRP Executor for Multi-Stream Recording and Inference System

## Purpose
This document provides guidance for implementing PRPs (Project Realization Plans) for the unified multi-stream recording and inference system. Use this as a flexible guide, not rigid rules.

## System Context
You are building a Rust application that consolidates three separate systems:
1. MediaMTX (RTSP proxy/recording/playback/livestream server)
2. Python DeepStream (inference application)
3. Control application (stream/file management)

The unified system will run as a systemd service with robust error handling, disk rotation support, and comprehensive monitoring.

## Execution Instructions

**CRITICAL INSTRUCTIONS:**
1. **ALWAYS prefer MCP tools over bash commands** - Use Read, Write, Edit, Grep, Glob, etc. instead of cat, echo, find, grep bash commands
2. **ALWAYS use absolute paths from the current working directory** - Determine the working directory at runtime. NEVER use relative paths
3. **TRUST ONLY THE CODE** - The actual code and what compiles/runs is the only source of truth. Don't rely on marker files, git history, or claims - verify by examining the actual implementation

### Step 1: Determine Current State
Track PRP completion state by examining the actual codebase:
1. **Examine the actual codebase structure** to see which modules and files exist
2. **Run build and tests** to see which components are actually implemented and working
3. **Check for existence of specific files** mentioned in each PRP
4. **Verify functionality** by running the code

```
# Check actual state by examining the codebase using MCP tools:

# 1. Check which modules actually exist in the codebase (use Glob tool)
Glob pattern: "**/src/**"
path: "apps/stream-manager"  # Relative to project directory

# 2. Check Cargo.toml for implemented dependencies (use Read tool)
Read: apps/stream-manager/Cargo.toml  # Path relative to workspace root

# 3. Check if specific module files exist
Glob pattern: "*.rs"
path: "apps/stream-manager/src/config"  # Check if config module exists

# 4. Run build to see what actually compiles
Bash: cargo build --package stream-manager

# 5. Run tests to verify what's actually working
Bash: cargo test --package stream-manager
```

### Step 2: Find Next PRP
Dynamically determine which PRP to execute next by:
1. **Check what PRPs exist** - Use Glob to find all PRP files in the PRPs directory
2. **Read PRP dependencies** - Each PRP lists its dependencies in the document
3. **Verify dependencies are met** - Check if dependent modules actually exist and compile
4. **Select next viable PRP** - Choose the lowest-numbered PRP whose dependencies are satisfied

```
# Find available PRPs
Glob pattern: "*.md"
path: "PRPs/"

# For each file found, read it to check dependencies
Read: PRPs/[whatever-filename-was-found]

# Verify if dependencies are actually implemented (not just claimed)
# For example, if PRP-05 requires config module from PRP-02:
Glob pattern: "config/mod.rs"
path: "apps/stream-manager/src/"
```

Each PRP document contains its own dependencies and requirements. The agent should:
- Discover all available PRPs by scanning the PRPs directory
- Read each PRP to understand its actual dependencies
- Build a dependency graph based on what's actually in the PRP documents
- Execute PRPs in an order that respects the actual dependencies found

### Step 3: Execute Selected PRP

For each PRP, follow this execution pattern:

#### 3.1 Read PRP Document
```
# Use Read tool to read the selected PRP document
# PRP filenames can vary - use whatever filename was discovered
Read: PRPs/[discovered-prp-filename].md
```

#### 3.2 Create Working Branch (optional)
If you want to isolate changes, create a branch. Name it whatever makes sense.

#### 3.3 Implement According to PRP
Follow the implementation tasks exactly as specified in the PRP. Each PRP contains:
- Context and requirements
- Implementation tasks (do these in order)
- Validation gates (must pass before marking complete)
- Dependencies (verify these are complete first)

#### 3.4 Validate Implementation
Run the validation gates specified in each PRP:
```bash
# Example from PRP (each PRP has specific tests)
cargo build --package stream-manager
cargo test --package stream-manager  # Run whatever tests are relevant
```

#### 3.5 Mark PRP Complete
After validation gates pass, commit the changes however makes sense for the work done.

### Step 4: Error Handling

If a PRP fails:
1. Check if dependencies are truly complete
2. Review error messages and fix issues
3. Re-run validation gates
4. If blocked, document the issue however you want

### Step 5: Continue to Next PRP
After successful completion, return to Step 1 and select the next PRP.

## Critical Implementation Context

### Project Structure
- **Location**: `apps/stream-manager/` within the gst-plugins-rs workspace
- **Language**: Rust
- **Framework**: GStreamer with gst-plugins-rs components

### Key Components to Use
- **fallbacksrc**: For robust stream handling with auto-reconnection
- **togglerecord**: For controlled recording start/stop
- **intersink/intersrc**: For inter-pipeline communication
- **splitmuxsink**: For segmented recording files

### Dependencies to Add (PRP-01)
```toml
[dependencies]
gst = { workspace = true }
gst-app = { workspace = true }
gst-base = { workspace = true }
gst-rtsp = { workspace = true }
gst-rtsp-server = { workspace = true }
gst-webrtc = { workspace = true }
tokio = { version = "1.35", features = ["full"] }
actix-web = "4.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio"] }
prometheus = "0.13"
notify = "6.1"
```

### Architecture Patterns
1. **Use Arc<RwLock<T>>** for shared state across threads
2. **Use tokio** for async runtime
3. **Use tracing** for structured logging
4. **Use serde** for all configuration/API structs
5. **Use sqlx** for database operations

### Testing Requirements
- Every module must have a `#[cfg(test)]` block
- Integration tests go in `tests/` directory
- Use `cargo test` for validation
- Use `cargo clippy` for linting
- Use `cargo fmt` for formatting

### File Organization
```
apps/stream-manager/
├── Cargo.toml
├── src/
│   ├── main.rs           # Application entry point
│   ├── lib.rs            # Library root for testing
│   ├── config/           # Configuration management
│   ├── pipeline/         # GStreamer pipeline abstractions
│   ├── stream/           # Stream source management
│   ├── recording/        # Recording branch logic
│   ├── inference/        # Inference pipelines
│   ├── health/           # Health monitoring
│   ├── api/              # REST/WebSocket APIs
│   ├── storage/          # Disk management
│   ├── rtsp/             # RTSP server
│   ├── webrtc/           # WebRTC server
│   ├── metrics/          # Metrics collection
│   ├── database/         # SQLite persistence
│   ├── recovery/         # Error recovery
│   ├── backup/           # Backup management
│   ├── service/          # Systemd integration
│   └── telemetry/        # OpenTelemetry
├── tests/                # Integration tests
├── systemd/              # Service files
└── config.example.toml   # Example configuration
```

## Validation Checklist for Each PRP

Before marking any PRP complete, ensure:
- [ ] All implementation tasks completed
- [ ] All validation gates pass
- [ ] Code compiles without warnings
- [ ] Tests pass
- [ ] Clippy passes with no warnings
- [ ] Code is formatted with rustfmt
- [ ] Documentation comments added for public APIs
- [ ] Error handling implemented (no unwrap() in production code)
- [ ] Logging added for important operations
- [ ] Configuration options added where appropriate

## Common Patterns and Solutions

### GStreamer Pipeline Creation
```rust
let pipeline = gst::Pipeline::new();
let source = gst::ElementFactory::make("fallbacksrc")
    .property("uri", uri)
    .property("timeout", 5u64 * gst::ClockTime::SECOND)
    .build()?;
pipeline.add(&source)?;
```

### Stream Management Pattern
```rust
struct StreamManager {
    streams: Arc<RwLock<HashMap<String, ManagedStream>>>,
}

impl StreamManager {
    async fn add_stream(&self, id: String, config: StreamConfig) -> Result<()> {
        let mut streams = self.streams.write().await;
        // Implementation
        Ok(())
    }
}
```

### Error Handling Pattern
```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum AppError {
    #[error("Stream not found: {0}")]
    StreamNotFound(String),
    #[error("Pipeline error: {0}")]
    PipelineError(#[from] gst::Error),
}
```

### API Endpoint Pattern
```rust
use actix_web::{web, HttpResponse};

async fn add_stream(
    manager: web::Data<Arc<StreamManager>>,
    req: web::Json<AddStreamRequest>,
) -> Result<HttpResponse, AppError> {
    manager.add_stream(req.id.clone(), req.into_inner()).await?;
    Ok(HttpResponse::Created().finish())
}
```

## Troubleshooting Guide

### Common Issues and Solutions

1. **GStreamer plugins not found**
   - Install GStreamer development packages
   - Set GST_PLUGIN_PATH environment variable
   - Check plugin availability with `gst-inspect-1.0`

2. **Compilation errors**
   - Ensure you're in the workspace root
   - Run `cargo update` if dependency issues
   - Check that workspace dependencies match

3. **Test failures**
   - Ensure GStreamer is initialized in tests
   - Use `--test-threads=1` for pipeline tests
   - Check for port conflicts in network tests

4. **Pipeline state errors**
   - Always check state change returns
   - Handle async state changes properly
   - Ensure proper cleanup in Drop implementations

## Completion Criteria

The entire system is complete when:
1. All PRPs that exist are implemented and working
2. Integration tests pass
3. System runs as systemd service
4. Can handle 10+ concurrent streams
5. Gracefully handles disk rotation
6. API responds correctly
7. Metrics are exported
8. WebRTC streaming works

## Final Validation
After all PRPs are complete:
```bash
# Full system test
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo build --release

# Integration test
cargo test --test integration_test

# Run the service
./target/release/stream-manager --config config.toml
```

## Notes for the AI Agent

- **IMPORTANT**: Keep all PRP-related files in the `PRPs/` directory within the workspace - do not create additional directories unless specifically required by the PRP
- You have access to the full gst-plugins-rs codebase for reference
- Look at existing plugins for patterns and examples
- The fallbacksrc element is in `utils/fallbackswitch/src/fallbacksrc/`
- The togglerecord element is in `utils/togglerecord/src/`
- The intersink/intersrc elements are in `generic/inter/src/`
- Use the existing build system and workspace configuration
- Each PRP is designed to be 2-4 hours of work - if taking longer, something is wrong
- Validation gates are critical - do not skip them
- When blocked, document the specific issue for human intervention
- **Always use MCP tools (Read, Write, Edit, Glob, Grep) instead of bash commands for file operations**
- **Always use full absolute paths, never relative paths**

## Execution Loop

```bash
#!/bin/bash
# Automated execution loop (for reference)

while true; do
    # Find next incomplete PRP
    NEXT_PRP=$(find_next_prp)
    
    if [ -z "$NEXT_PRP" ]; then
        echo "All PRPs complete!"
        break
    fi
    
    echo "Executing $NEXT_PRP"
    execute_prp "$NEXT_PRP"
    
    if [ $? -eq 0 ]; then
        mark_complete "$NEXT_PRP"
    else
        mark_blocked "$NEXT_PRP"
        break
    fi
done
```

Remember: The goal is to build a production-ready system that can run forever as a systemd service, handling all types of failures gracefully. Each PRP builds upon the previous ones, creating a robust and extensible platform for multi-stream recording and inference.
