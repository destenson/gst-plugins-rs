# Stream Manager

A high-performance, production-ready multi-stream recording and inference system built on GStreamer.

## Overview

Stream Manager is a unified Rust application that consolidates and extends the functionality of multiple streaming tools:

- **MediaMTX replacement**: RTSP proxy, recording, playback, and livestream server
- **DeepStream alternative**: GPU-accelerated and CPU-based inference pipelines  
- **Control plane**: Centralized stream and file management with REST API

Designed to run as a systemd service handling 100+ concurrent streams in production environments.

## Key Features

### Core Capabilities
- **Multi-stream Management**: Handle 100+ concurrent RTSP/HTTP/WebRTC streams
- **Recording System**: 
  - Segmented recording with configurable chunk duration
  - Automatic file rotation and retention policies
  - Pause/resume recording without stream interruption
- **Inference Pipelines**:
  - NVIDIA GPU acceleration via TensorRT
  - CPU fallback for environments without GPU
  - Real-time object detection and classification
- **Streaming Protocols**:
  - RTSP server with authentication support
  - WebRTC with WHIP/WHEP protocols
  - HLS/DASH adaptive streaming
  
### Reliability & Operations
- **Automatic Recovery**: Stream reconnection with exponential backoff
- **Health Monitoring**: Prometheus metrics, health endpoints, OpenTelemetry tracing
- **State Persistence**: Survives restarts with full state recovery
- **Storage Management**: Automatic disk space management and rotation
- **Hot Configuration Reload**: Update settings without service restart

### Integration
- **REST API**: Complete control plane for all operations
- **WebSocket Events**: Real-time event streaming for monitoring
- **Backup System**: Automated configuration and data backup
- **systemd Integration**: Native service management and logging

## Architecture

### Pipeline Components

The application leverages advanced GStreamer elements for robust stream handling:

```
Input → fallbacksrc → Processing → Distribution
         ↓              ↓            ↓
    (auto-reconnect) (inference)  (recording/streaming)
```

- **`fallbacksrc`**: Automatic stream reconnection with fallback sources
- **`togglerecord`**: Seamless recording start/stop without frame loss
- **`intersink/intersrc`**: Zero-copy inter-pipeline communication
- **`splitmuxsink`**: Segmented recording with configurable chunk size
- **`webrtcbin`**: WebRTC streaming with ICE/STUN/TURN support

### Module Structure

```
stream-manager/
├── src/
│   ├── api/          # REST API and WebSocket handlers
│   ├── config/       # Configuration management
│   ├── manager/      # Core stream management logic
│   ├── pipeline/     # GStreamer pipeline builders
│   ├── recording/    # Recording branch management
│   ├── inference/    # AI inference pipelines
│   ├── storage/      # Disk management and rotation
│   ├── metrics/      # Prometheus metrics collection
│   ├── health/       # Health monitoring system
│   ├── backup/       # Backup and recovery system
│   ├── webrtc/       # WebRTC server implementation
│   └── rtsp/         # RTSP proxy server
├── tests/            # Integration and unit tests
└── docs/             # Additional documentation
```

## Quick Start

### Prerequisites

- Rust 1.83+ 
- GStreamer 1.24+ with development packages
- (Optional) NVIDIA drivers and CUDA for GPU inference

### Building

```bash
# Build with all features
cargo build --package stream-manager --release

# Build without GPU support
cargo build --package stream-manager --release --no-default-features
```

### Running

```bash
# Run with default configuration
cargo run --package stream-manager

# Run with custom config file
cargo run --package stream-manager -- --config /path/to/config.toml

# Run with environment variable overrides
STREAM_MANAGER_PORT=8080 cargo run --package stream-manager
```

### Basic Usage

1. Start a stream:
```bash
curl -X POST http://localhost:3000/api/streams \
  -H "Content-Type: application/json" \
  -d '{
    "id": "camera-1",
    "source_url": "rtsp://camera.local:554/stream",
    "recording": {"enabled": true}
  }'
```

2. Check stream status:
```bash
curl http://localhost:3000/api/streams/camera-1
```

3. Start recording:
```bash
curl -X POST http://localhost:3000/api/streams/camera-1/recording/start
```

## Configuration

Configuration can be provided via:
1. TOML configuration file
2. Environment variables (prefix: `STREAM_MANAGER_`)
3. Command-line arguments

See `config.example.toml` for all available options or check the [Configuration Guide](docs/CONFIG.md).

## Development Status

This application is being developed through a series of Progressive Refinement Proposals (PRPs).
See the PRPs directory for implementation details.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
