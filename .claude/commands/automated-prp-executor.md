# Automated PRP Executor for Multi-Stream Recording and Inference System

## Purpose
This document serves as an automated execution guide for an AI agent to sequentially implement all PRPs for the unified multi-stream recording and inference system. The agent should read this document, determine the next unimplemented PRP, and execute it completely before moving to the next.

## System Context
You are building a Rust application that consolidates three separate systems:
1. MediaMTX (RTSP proxy/recording/playback/livestream server)
2. Python DeepStream (inference application)
3. Control application (stream/file management)

The unified system will run as a systemd service with robust error handling, disk rotation support, and comprehensive monitoring.

## Execution Instructions

NOTE: Instructions use bash for clarity, but you need to use MCP tools and other tools you have available instead of bash whenever possible.

### Step 1: Determine Current State
Check for the existence of these marker files to determine which PRPs are complete:
```bash
# Check completion markers
ls -la apps/stream-manager/.prp-completed/
```

If the directory doesn't exist, create it and start with PRP-01:
```bash
mkdir -p apps/stream-manager/.prp-completed/
```

### Step 2: Find Next PRP
Execute PRPs in this exact order (dependencies must be respected):

**Phase 1 - Foundation (MUST complete before Phase 2)**
- [ ] PRP-01: Project Structure and Cargo Workspace Setup
- [ ] PRP-02: Configuration Management Layer  
- [ ] PRP-03: GStreamer Initialization and Plugin Discovery
- [ ] PRP-04: Pipeline Abstraction Layer
- [ ] PRP-05: Stream Source Management

**Phase 2 - Stream Processing (MUST complete before Phase 3)**
- [ ] PRP-06: Stream Branching with Tee Element
- [ ] PRP-07: Recording Branch Implementation
- [ ] PRP-08: Inter-Pipeline Communication Setup
- [ ] PRP-09: Stream Manager Core Orchestration
- [ ] PRP-10: Stream Health Monitoring System

**Phase 3 - Control Interface (MUST complete before Phase 4)**
- [ ] PRP-11: REST API Foundation
- [ ] PRP-12: Stream Control API Endpoints
- [ ] PRP-13: Metrics and Statistics Collection
- [ ] PRP-14: WebSocket Event Streaming
- [ ] PRP-15: Configuration Hot-Reload System

**Phase 4 - Storage and Resilience**
- [ ] PRP-16: Storage Management and Disk Monitoring
- [ ] PRP-17: Disk Rotation and Hot-Swap Support
- [ ] PRP-18: Systemd Service Integration
- [ ] PRP-19: Error Recovery and Resilience
- [ ] PRP-20: State Persistence and Database Integration

**Phase 5 - Advanced Features (Can parallel with Phase 6)**
- [ ] PRP-21: NVIDIA Inference Branch Implementation
- [ ] PRP-22: CPU Inference Fallback Implementation
- [ ] PRP-23: Telemetry and Distributed Tracing
- [ ] PRP-24: Backup and Disaster Recovery

**Phase 6 - Streaming Servers (Can parallel with Phase 5)**
- [ ] PRP-25: RTSP Server and Proxy Implementation
- [ ] PRP-26: WebRTC Server Implementation
- [ ] PRP-27: WHIP/WHEP Protocol Support
- [ ] PRP-28: Integration Testing and Validation Framework

### Step 3: Execute Selected PRP

For each PRP, follow this execution pattern:

#### 3.1 Read PRP Document
```bash
# Read the PRP document
cat PRPs/{prp-number}-{prp-name}.md
```

#### 3.2 Create Working Branch
```bash
git checkout -b prp-{number}-implementation
```

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
cargo test --package stream-manager {module}::tests
```

#### 3.5 Mark PRP Complete
Only after ALL validation gates pass:
```bash
touch apps/stream-manager/.prp-completed/prp-{number}.done
git add -A
git commit -m "Complete PRP-{number}: {prp-title}"
```

### Step 4: Error Handling

If a PRP fails:
1. Check if dependencies are truly complete
2. Review error messages and fix issues
3. Re-run validation gates
4. If blocked, create a `.blocked` file with details:
```bash
echo "Blocked: {reason}" > apps/stream-manager/.prp-completed/prp-{number}.blocked
```

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
1. All 28 PRPs have `.done` files
2. Integration tests pass (PRP-28)
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

- You have access to the full gst-plugins-rs codebase for reference
- Look at existing plugins for patterns and examples
- The fallbacksrc element is in `utils/fallbackswitch/src/fallbacksrc/`
- The togglerecord element is in `utils/togglerecord/src/`
- The intersink/intersrc elements are in `generic/inter/src/`
- Use the existing build system and workspace configuration
- Each PRP is designed to be 2-4 hours of work - if taking longer, something is wrong
- Validation gates are critical - do not skip them
- When blocked, document the specific issue for human intervention

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
