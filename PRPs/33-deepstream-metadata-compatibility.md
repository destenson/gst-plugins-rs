# PRP-33: DeepStream Metadata Compatibility Layer

## Overview
Implement DeepStream-compatible metadata structures and attach them to GStreamer buffers, enabling downstream elements to process inference results using the same metadata format as NVIDIA DeepStream.

## Context
- DeepStream uses NvDsObjectMeta, NvDsFrameMeta structures
- Metadata attached to GstBuffer for downstream processing
- Other elements expect this metadata format (nvdsosd, nvtracker)
- Must maintain binary compatibility where possible
- Rust needs to interface with C structures

## Requirements
1. Define DeepStream metadata structures in Rust
2. Implement metadata registration with GStreamer
3. Attach inference results to buffers
4. Support metadata pools for efficiency
5. Ensure compatibility with DeepStream elements

## Implementation Tasks
1. Create metadata structures module:
   - Create metadata.rs in cpuinfer/
   - Define NvDsObjectMeta equivalent
   - Define NvDsFrameMeta equivalent
   - Define NvDsClassifierMeta equivalent
   - Include all required fields for compatibility

2. Implement GStreamer metadata registration:
   - Create custom GstMeta types
   - Register with GStreamer type system
   - Implement init/free functions
   - Handle metadata copying
   - Support metadata serialization

3. Create metadata pools:
   - Pool for object metadata
   - Pool for frame metadata
   - Efficient allocation/deallocation
   - Thread-safe pool management
   - Configurable pool sizes

4. Implement result conversion:
   - Convert inference outputs to metadata
   - Map bounding boxes to object meta
   - Set class IDs and confidence scores
   - Handle tracking IDs (placeholder)
   - Attach classifier results

5. Add metadata attachment:
   - Get or create frame metadata on buffer
   - Add object metadata to frame
   - Link metadata structures properly
   - Handle metadata ownership
   - Support multiple inference results per frame

## Validation Gates
```bash
# Test metadata creation
cargo test metadata_creation

# Test metadata attachment
cargo test metadata_attachment

# Test with DeepStream-compatible element
gst-launch-1.0 videotestsrc ! cpuinfer ! fakesink dump=true

# Verify metadata with custom pad probe
cargo test metadata_probe_verification
```

## Dependencies
- PRP-29: Plugin structure
- PRP-31: Inference results to convert
- GStreamer metadata API
- Understanding of DeepStream metadata layout

## References
- DeepStream SDK documentation on metadata
- nvdsmeta.h from DeepStream SDK
- gstreamer-rs metadata examples
- video/closedcaption for metadata patterns

## Success Metrics
- Metadata structures match DeepStream format
- Downstream elements can read metadata
- No memory leaks in metadata handling
- Metadata pools improve performance
- Can interoperate with nvdsosd if available

**Confidence Score: 6/10**