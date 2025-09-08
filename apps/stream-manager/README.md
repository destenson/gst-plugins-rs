# Stream Manager

A unified multi-stream recording and inference system built on GStreamer.

## Overview

Stream Manager consolidates the functionality of:
- MediaMTX (RTSP proxy/recording/playback/livestream server)
- Python DeepStream (inference application)
- Control application (stream/file management)

Into a single, robust Rust application that runs as a systemd service.

## Features

- **Multi-stream Management**: Handle 100+ concurrent RTSP streams
- **Recording**: Segmented recording with configurable retention
- **Inference**: NVIDIA GPU and CPU fallback inference pipelines
- **Streaming**: RTSP server, WebRTC with WHIP/WHEP support
- **Monitoring**: Health checks, metrics, OpenTelemetry tracing
- **Resilience**: Automatic recovery, disk rotation, state persistence
- **API**: REST API and WebSocket event streaming

## Architecture

The application uses:
- `fallbacksrc` for robust stream handling with auto-reconnection
- `togglerecord` for controlled recording start/stop
- `intersink/intersrc` for inter-pipeline communication
- `splitmuxsink` for segmented recording files

## Building

```bash
cargo build --package stream-manager
```

## Running

```bash
cargo run --package stream-manager -- --config config.toml
```

## Configuration

See `config.example.toml` for configuration options.

## Development Status

This application is being developed through a series of Progressive Refinement Proposals (PRPs).
See the PRPs directory for implementation details.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
