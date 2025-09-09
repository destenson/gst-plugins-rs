# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This is the GStreamer Rust plugins repository (gst-plugins-rs), containing various GStreamer plugins and elements written in Rust. The plugins build upon the GStreamer Rust bindings from https://gitlab.freedesktop.org/gstreamer/gstreamer-rs.

## Project Structure

The repository is organized as a Cargo workspace with plugins grouped by category:
- `audio/` - Audio processing plugins (audiofx, claxon, csound, lewton, spotify)
- `generic/` - General purpose plugins (file, sodium, threadshare, inter, streamgrouper)
- `net/` - Network and streaming plugins (aws, webrtc, quinn, ndi, onvif, rtp, rtsp)
- `video/` - Video processing plugins (closedcaption, dav1d, gtk4, rav1e, png, gif)
- `mux/` - Muxing/demuxing plugins (fmp4, mp4, flavors)
- `text/` - Text processing plugins (ahead, json, regex, wrap)
- `utils/` - Utility plugins (fallbackswitch, livesync, togglerecord, tracers)
- `tutorial/` - Example plugins for learning

## Build Commands

### Windows-specific considerations
- Use `-j 1` flag when encountering memory/paging file errors
- Windows paths must be quoted in commands
- Some plugins are excluded on Windows (csound, webp, gtk4 with certain features)

### Basic Build Commands
```bash
# Build all plugins (default members only)
cargo build --all

# Build with release optimizations
cargo build --all --release

# Build a specific plugin
cargo cbuild -p gst-plugin-threadshare

# Build with all features
cargo build --all --all-features --exclude gst-plugin-gtk4

# Build with no default features
cargo build --all --no-default-features
```

### Running Tests
```bash
# Run all tests (uses cargo-nextest if available)
cargo test --all

# Run tests for a specific plugin
cargo test -p gst-plugin-threadshare

# Run tests with different feature configurations
cargo test --all --no-default-features
cargo test --all --all-features --exclude gst-plugin-gtk4
```

### Installing Plugins
```bash
# Install using cargo-c (required: cargo install cargo-c)
cargo cbuild -p gst-plugin-<name> --prefix=/usr
cargo cinstall -p gst-plugin-<name> --prefix=/usr
```

## Plugin Architecture

Each plugin follows a standard structure:
1. `Cargo.toml` - Defines dependencies and metadata
2. `build.rs` - Build script using `gst_plugin_version_helper::info()`
3. `src/lib.rs` - Plugin entry point with `gst::plugin_define!` macro
4. `src/<element>/` - Individual element implementations with `imp.rs` and `mod.rs`
5. `tests/` - Integration tests for the plugin

### Creating a New Element

Elements typically have:
- `mod.rs` - Public interface and element registration
- `imp.rs` - Implementation using GStreamer subclassing

Example element registration pattern:
```rust
fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "elementname",
        gst::Rank::NONE,
        ElementType::static_type(),
    )
}
```

## Key Dependencies

The workspace uses git dependencies for GStreamer bindings:
- `gstreamer` crates from https://gitlab.freedesktop.org/gstreamer/gstreamer-rs (branch: main)
- `gtk-rs` crates from https://github.com/gtk-rs/gtk-rs-core (branch: main)

## Testing Environment

- Tests use `gst-check` for GStreamer testing utilities
- Set `RUST_BACKTRACE=1` and `G_DEBUG=fatal_warnings` for debugging
- CI uses `cargo-nextest` with `--profile=ci` for parallel test execution
- Some plugins require specific environment variables (e.g., `CSOUND_LIB_DIR`)

## Common Development Tasks

### Adding a dependency to a plugin
```bash
cd <plugin-directory>
cargo add <dependency-name>
```

### Checking for clippy warnings
```bash
cargo clippy --all
```

### Running cargo-deny checks
```bash
cargo deny check
```

### Building documentation
```bash
cargo doc --all --no-deps
```

## Important Notes

- MSRV (Minimum Supported Rust Version): 1.83
- Each plugin is built as both cdylib (for GStreamer) and rlib (for Rust usage)
- Use `#[allow(clippy::non_send_fields_in_send_ty)]` in lib.rs files
- Plugins use `gst_plugin_version_helper` in build.rs for version information
- The threadshare plugin includes C code and requires special build handling

## Critical Development Principles

### The Compiler Is Always Right
**NEVER blame the compiler for "misleading" errors.** The Rust compiler is meticulously designed and battle-tested. If you think the compiler is wrong or misleading, you are misunderstanding something fundamental about your code.

When you encounter a confusing error:
1. **Read the error message literally** - it's telling you exactly what's wrong
2. **Question your assumptions** - what you think your code does vs. what it actually does
3. **Examine the context** - imports, macro expansions, type inference, trait bounds
4. **The compiler sees your code after all transformations** - you might be looking at the wrong abstraction level

The compiler error is not a suggestion or approximation - it's a precise description of why your code cannot compile. Your job is to understand what the compiler is seeing, not to argue with it.