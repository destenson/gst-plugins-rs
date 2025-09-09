# PRP-30: CPU Inference Element Properties and Configuration

## Overview
Implement the property system for the cpuinfer element to maintain compatibility with DeepStream's nvinfer configuration while adapting for CPU-based inference. This includes model paths, confidence thresholds, and processing parameters.

## Context
- DeepStream nvinfer uses config files and properties for configuration
- Need to support similar properties for drop-in compatibility
- Must handle both ONNX and TensorFlow Lite models
- Properties should be runtime-configurable where possible

## Requirements
1. Define property structure compatible with DeepStream
2. Implement property getters/setters
3. Support configuration file parsing
4. Validate property values
5. Handle property change notifications

## Implementation Tasks
1. Define properties enum in cpuinfer/imp.rs:
   - model-path: Path to ONNX/TFLite model
   - config-file-path: Optional config file (DeepStream compatible)
   - batch-size: Processing batch size
   - confidence-threshold: Detection confidence threshold
   - inference-interval: Process every Nth frame
   - model-type: "onnx" or "tflite"
   - num-detected-classes: Number of output classes
   - maintain-aspect-ratio: Preserve aspect ratio during preprocessing
   - symmetric-padding: Use symmetric padding
   - processing-width/height: Input dimensions for model

2. Implement property handling:
   - Use glib::ParamSpec for property definitions
   - Implement ObjectImpl trait methods
   - Handle property validation in set_property
   - Return current values in property getter

3. Create config file parser:
   - Support subset of DeepStream config format
   - Parse INI-style configuration
   - Map config values to element properties
   - Handle model-specific parameters

4. Add property change handling:
   - Validate model path exists
   - Check batch size constraints
   - Ensure dimensions are valid
   - Handle runtime property updates where safe

5. Implement settings struct:
   - Store validated property values
   - Provide default values
   - Handle serialization for debugging

## Validation Gates
```bash
# Test property setting via gst-launch
gst-launch-1.0 videotestsrc ! cpuinfer model-path=model.onnx ! fakesink

# Test config file loading
cargo test cpuinfer_config_parsing

# Verify property introspection
gst-inspect-1.0 cpuinfer | grep -A1 "Element Properties"
```

## Dependencies
- PRP-29: Plugin structure must be in place
- GStreamer property system
- TOML or INI parsing library

## References
- nvinfer properties documentation
- gstreamer-rs property examples in other elements
- DeepStream configuration guide
- video/rav1e/imp.rs for complex property handling

## Success Metrics
- All properties can be set and retrieved
- Config file parsing works correctly
- Property validation prevents invalid states
- gst-inspect shows all properties

**Confidence Score: 8/10**