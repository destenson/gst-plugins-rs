# PRP-32: Video Frame Preprocessing Pipeline

## Overview
Implement the video frame preprocessing pipeline to prepare GStreamer video buffers for inference, including colorspace conversion, resizing, normalization, and padding to maintain aspect ratios.

## Context
- Models expect specific input formats and dimensions
- GStreamer buffers come in various formats (I420, NV12, RGB, etc.)
- Need efficient preprocessing without excessive copying
- Must maintain aspect ratio to prevent distortion
- Preprocessing is often the bottleneck in inference pipelines

## Requirements
1. Support multiple input video formats
2. Resize frames to model input dimensions
3. Handle aspect ratio preservation
4. Normalize pixel values for model input
5. Implement efficient buffer transformations

## Implementation Tasks
1. Create frame preprocessor module:
   - Create preprocessor.rs in cpuinfer/
   - Define PreprocessorConfig struct
   - Support different preprocessing modes
   - Handle preprocessing parameters from properties
   - Implement builder pattern for configuration

2. Implement colorspace conversion:
   - Support I420, NV12, RGB, BGR formats
   - Use efficient conversion algorithms
   - Handle planar and packed formats
   - Implement zero-copy where possible
   - Add format negotiation in sink caps

3. Add resizing functionality:
   - Implement bilinear interpolation
   - Support nearest neighbor for speed
   - Handle aspect ratio preservation
   - Calculate padding requirements
   - Implement letterboxing/pillarboxing

4. Implement normalization:
   - Support different normalization schemes
   - Mean subtraction (ImageNet style)
   - Scale to [0,1] or [-1,1]
   - Channel-wise normalization
   - Configurable per model requirements

5. Create batching support:
   - Collect frames for batch processing
   - Handle incomplete batches
   - Implement timeout for low-framerate streams
   - Manage batch buffer allocation
   - Track frame metadata through batching

## Validation Gates
```bash
# Test preprocessing with different formats
cargo test preprocessing_formats

# Test aspect ratio preservation
cargo test preprocessing_aspect_ratio

# Benchmark preprocessing performance
cargo bench preprocessing_performance

# Visual test with videotestsrc
gst-launch-1.0 videotestsrc ! video/x-raw,format=I420 ! cpuinfer ! videoconvert ! autovideosink
```

## Dependencies
- PRP-29: Plugin structure
- PRP-31: Tensor format requirements
- image crate for image operations
- rayon for parallel processing

## References
- video/videoconvert for format conversion patterns
- image crate documentation
- OpenCV preprocessing examples
- DeepStream preprocessing documentation

## Success Metrics
- All common formats supported
- Aspect ratio preserved correctly
- Preprocessing performance acceptable
- No visual artifacts in output
- Memory usage optimized

**Confidence Score: 8/10**