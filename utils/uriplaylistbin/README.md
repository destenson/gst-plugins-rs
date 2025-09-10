# GStreamer URI Playlist Bin Plugin

A GStreamer plugin that provides seamless playlist playback functionality.

## Features

- **uriplaylistbin**: High-level element for continuous playback of multiple media files
- Seamless transitions between playlist items
- Support for various media formats through GStreamer's decoder infrastructure
- Automatic handling of different media types in the same playlist

## Installation

### From Debian Package

```bash
sudo apt install gst-plugin-uriplaylistbin
```

### From Source

```bash
cargo build --release -p gst-plugin-uriplaylistbin
```

## Usage

### Basic Pipeline

```bash
gst-launch-1.0 uriplaylistbin uris="file:///path/to/file1.mp4,file:///path/to/file2.mp4" ! autovideosink
```

### Playlist Example

See `examples/playlist.rs` for a complete example of using the uriplaylistbin element in a Rust application.

```bash
cargo run --example playlist -- --uris file1.mp4 file2.mp4
```

## Element Properties

- `uris`: Comma-separated list of URIs to play
- `iterations`: Number of times to repeat the playlist (0 = infinite)
- `current-iteration`: Current iteration count (read-only)
- `current-uri-index`: Index of currently playing URI (read-only)

## Requirements

- GStreamer 1.24 or later
- Rust 1.70 or later (for building from source)

## License

This plugin is licensed under the Mozilla Public License Version 2.0.