# PRP-35: TensorFlow Lite Runtime Support

## Overview
Add TensorFlow Lite runtime support alongside ONNX to enable inference with TFLite models, which are optimized for edge devices and often smaller than ONNX models.

## Context
- TFLite models are common for edge deployment
- Often more optimized than ONNX for CPU
- Different API from ONNX Runtime
- Need unified interface for both runtimes
- Some models only available in TFLite format

## Requirements
1. Integrate TensorFlow Lite runtime
2. Create unified inference interface
3. Support TFLite model loading
4. Handle TFLite-specific optimizations
5. Maintain API compatibility with ONNX path

## Implementation Tasks
1. Add TFLite runtime integration:
   - Add tflite crate dependency
   - Create TfLiteInferenceEngine struct
   - Initialize TFLite interpreter
   - Configure CPU optimization options
   - Handle delegate selection

2. Implement unified inference trait:
   - Define InferenceEngine trait
   - Implement for both ONNX and TFLite
   - Abstract model loading
   - Unified tensor interface
   - Common error handling

3. Create TFLite model support:
   - Load .tflite model files
   - Extract model metadata
   - Validate input/output tensors
   - Support quantized models
   - Handle dynamic shapes

4. Implement TFLite-specific features:
   - Support INT8 quantized models
   - Handle quantization parameters
   - Implement dequantization
   - Support TFLite metadata
   - Enable XNNPACK delegate

5. Add runtime selection logic:
   - Auto-detect model format
   - Property for explicit selection
   - Fallback mechanisms
   - Performance comparison mode
   - Runtime switching support

## Validation Gates
```bash
# Test TFLite model loading
cargo test tflite_model_loading

# Test inference with TFLite
cargo test tflite_inference

# Compare ONNX vs TFLite outputs
cargo test runtime_comparison

# Benchmark both runtimes
cargo bench inference_runtime_comparison

# Test with TFLite model
gst-launch-1.0 videotestsrc ! cpuinfer model-path=model.tflite model-type=tflite ! fakesink
```

## Dependencies
- PRP-31: ONNX runtime implementation
- PRP-32: Preprocessing pipeline
- tflite crate or tensorflow-lite-sys
- Common inference interface design

## References
- TensorFlow Lite Rust bindings
- TFLite C API documentation
- TFLite model optimization guide
- Edge TPU compiler compatibility

## Success Metrics
- TFLite models load successfully
- Inference produces correct results
- Quantized models work properly
- Performance comparable or better than ONNX
- Seamless runtime switching

**Confidence Score: 7/10**