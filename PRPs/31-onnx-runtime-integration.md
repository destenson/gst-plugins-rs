# PRP-31: ONNX Runtime Integration for Model Inference

## Overview
Integrate ONNX Runtime into the cpuinfer element to enable model loading and inference execution on CPU. This PRP focuses on the core inference engine setup without the full preprocessing pipeline.

## Context
- ONNX is widely supported format for ML models
- ort crate provides Rust bindings to ONNX Runtime
- Need efficient memory management for video frames
- Must handle different model architectures (detection, classification)

## Requirements
1. Initialize ONNX Runtime environment
2. Load and validate ONNX models
3. Create inference sessions
4. Handle input/output tensor management
5. Implement basic inference execution

## Implementation Tasks
1. Add ONNX runtime initialization:
   - Create OnnxInferenceEngine struct
   - Initialize environment with appropriate providers
   - Configure CPU execution provider options
   - Handle thread pool configuration
   - Implement error handling for initialization failures

2. Implement model loading:
   - Load ONNX model from file path
   - Validate model inputs/outputs
   - Extract input dimensions and types
   - Store model metadata
   - Create inference session with options

3. Create tensor management:
   - Convert GStreamer buffers to tensors
   - Handle different pixel formats (RGB, BGR, NV12)
   - Implement efficient memory copying
   - Manage tensor lifecycles
   - Support batching multiple frames

4. Implement inference execution:
   - Prepare input tensors from video frames
   - Run inference session
   - Extract output tensors
   - Handle different output types (boxes, classes, scores)
   - Implement async inference option

5. Add model type detection:
   - Detect model architecture from outputs
   - Support detection models (YOLO, SSD style)
   - Support classification models
   - Handle custom output formats
   - Store model capabilities

## Validation Gates
```bash
# Test ONNX model loading
cargo test onnx_model_loading

# Test inference execution
cargo test onnx_inference_execution

# Benchmark inference performance
cargo bench cpuinfer_onnx_performance

# Test with sample model
gst-launch-1.0 videotestsrc ! cpuinfer model-path=tests/test_model.onnx ! fakesink
```

## Dependencies
- PRP-29: Plugin structure
- PRP-30: Properties for model-path
- ort crate (ONNX Runtime bindings)
- ndarray for tensor operations

## References
- https://github.com/pykeio/ort - ONNX Runtime Rust bindings
- ONNX model zoo for test models
- video/dav1d for frame handling patterns
- DeepStream nvinfer source for output formats

## Success Metrics
- ONNX models load successfully
- Inference produces valid outputs
- Memory usage is efficient
- No memory leaks during operation
- Performance is reasonable for CPU

**Confidence Score: 7/10**