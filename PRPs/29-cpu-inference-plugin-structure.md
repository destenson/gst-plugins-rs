# PRP-29: CPU Inference Plugin Structure and Foundation

## Overview
Create the foundational structure for a DeepStream-compatible CPU inference plugin in gst-plugins-rs that can serve as a fallback when NVIDIA GPUs are unavailable. This PRP focuses on setting up the plugin directory structure, Cargo configuration, and basic element registration.

## Context
- DeepStream uses nvinfer/nvinferserver for GPU inference
- Need CPU fallback for environments without NVIDIA GPUs
- Should maintain compatibility with DeepStream metadata structures
- Plugin should live in video/inference directory to group with other video processing plugins
- Must integrate with existing gst-plugins-rs workspace structure

## Requirements
1. Create plugin directory structure under video/inference
2. Set up Cargo.toml with appropriate dependencies
3. Implement basic plugin registration with gst::plugin_define!
4. Create placeholder element structure for cpuinfer
5. Ensure plugin builds within the workspace

## Implementation Tasks
1. Create directory structure:
   - video/inference/
   - video/inference/src/
   - video/inference/src/cpuinfer/
   - video/inference/tests/
   - video/inference/examples/

2. Create Cargo.toml with dependencies:
   - Standard gst workspace dependencies
   - ONNX runtime for model inference (ort crate)
   - Image processing (image, imageproc crates)
   - Serialization for metadata (serde, serde_json)
   - Follow pattern from other video plugins

3. Create src/lib.rs with plugin registration:
   - Use gst::plugin_define! macro
   - Register cpuinfer element
   - Follow pattern from video/closedcaption or video/videofx

4. Create basic element structure in src/cpuinfer/:
   - mod.rs for public interface
   - imp.rs for implementation
   - Follow GStreamer subclassing pattern

5. Create build.rs:
   - Use gst_plugin_version_helper
   - Follow pattern from other plugins

6. Add to workspace Cargo.toml:
   - Add to members list
   - Ensure it's in default members

## Validation Gates
```bash
# Build the new plugin
cargo build -p gst-plugin-inference

# Check plugin can be found
gst-inspect-1.0 target/debug/gstinference.dll

# Run basic tests
cargo test -p gst-plugin-inference
```

## Dependencies
- Existing gst-plugins-rs workspace structure
- GStreamer Rust bindings

## References
- video/videofx structure for plugin pattern
- video/closedcaption for complex video processing
- utils/fallbackswitch for element patterns
- https://gstreamer.freedesktop.org/documentation/plugin-development/

## Success Metrics
- Plugin builds successfully in workspace
- gst-inspect-1.0 shows the plugin
- Basic element registration works
- Tests pass

**Confidence Score: 9/10**