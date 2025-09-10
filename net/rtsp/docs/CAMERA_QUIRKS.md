# Camera Quirks and Known Issues

This document describes known quirks, issues, and workarounds for various IP camera models when used with rtspsrc2.

## Axis Communications

### General Axis Quirks
- **High default bitrate**: Many Axis cameras have very high default bitrate settings that can cause bandwidth issues
  - **Workaround**: Configure camera to use lower bitrate or adjust network buffer sizes
- **Digest authentication required**: Most Axis cameras require digest authentication
  - **Solution**: Always set `user-id` and `user-pw` properties with digest auth

### Axis M3045-V
- **Firmware**: 9.80.1+
- **RTSP URL**: `rtsp://<ip>/axis-media/media.amp`
- **Features**: H.264, H.265, Audio, ONVIF Profile S/T
- **Known Issues**:
  - Bandwidth limitation errors with default ONVIF profile
  - Requires explicit fps and resolution parameters for optimal performance
  - Example: `rtsp://<ip>/axis-media/media.amp?fps=15&resolution=1920x1080`

### Axis P-Series
- **PTZ Control**: Requires VAPIX API in addition to ONVIF for full PTZ functionality
- **Event Stream**: Proprietary event format, not fully ONVIF compliant

## Hikvision

### General Hikvision Quirks
- **ONVIF Account**: No default ONVIF account; must be created manually
  - **Solution**: Create ONVIF user through camera web interface
- **Channel numbering**: Uses specific channel IDs
  - Channel 101: Main stream, camera 1
  - Channel 102: Sub stream, camera 1  
  - Channel 201: Main stream, camera 2
  - Channel 202: Sub stream, camera 2

### DS-2CD2132F Series
- **Firmware**: V5.4.5+
- **RTSP URL**: `rtsp://<ip>:554/Streaming/Channels/<channel>`
- **Features**: H.264, H.265, Audio, ONVIF, Motion Detection
- **Known Issues**:
  - Requires manual ONVIF account setup
  - May disconnect after extended periods without keep-alive
  - Sub-stream quality settings affect main stream performance

### DS-2CD2385G1 Series
- **Special Requirements**:
  - Requires specific User-Agent header for some firmware versions
  - Smart codec features can cause compatibility issues
  - **Workaround**: Disable smart codec features for stable streaming

## Dahua Technology

### General Dahua Quirks
- **ONVIF Port**: Uses port 8080 for ONVIF instead of standard port 80
- **Stream URLs**: Different format than other manufacturers
  - Main: `rtsp://<ip>:554/cam/realmonitor?channel=1&subtype=0`
  - Sub: `rtsp://<ip>:554/cam/realmonitor?channel=1&subtype=1`

### IPC-HFW4431E Series
- **Firmware**: 2.800.0000000.16.R+
- **Features**: H.264, Audio, ONVIF, IVS (Intelligent Video System)
- **Known Issues**:
  - RTCP feedback may be delayed
  - Requires explicit TCP transport for stable streaming over WAN
  - Privacy mask features can cause encoding artifacts

### IPC-HDW4631C Series
- **Audio Issues**: 
  - G.711 audio codec may have synchronization issues
  - **Workaround**: Use AAC audio if available or disable audio

## Generic Issues and Solutions

### Connection Issues

#### "Too many viewers" Error
- **Cause**: Camera limits concurrent connections
- **Solutions**:
  1. Reduce number of concurrent streams
  2. Use sub-stream for multiple viewers
  3. Configure camera to allow more connections

#### Bandwidth Limitation
- **Cause**: Stream bitrate exceeds network or NVR capacity
- **Solutions**:
  1. Lower camera bitrate settings
  2. Use H.265 if supported
  3. Adjust GOP/I-frame interval

### Transport Protocol Issues

#### UDP Packet Loss
- **Symptoms**: Corrupted frames, artifacts
- **Solution**: Force TCP transport
  ```
  rtspsrc2 location=rtsp://... protocols=tcp
  ```

#### NAT/Firewall Issues
- **Symptoms**: Connection succeeds but no video
- **Solution**: Use TCP or HTTP tunneling
  ```
  rtspsrc2 location=rtsp://... protocols=tcp+http
  ```

### Authentication Issues

#### Digest Authentication Failures
- **Symptoms**: 401 Unauthorized errors
- **Solutions**:
  1. Ensure correct username/password
  2. Check for special characters in password
  3. Try basic auth if supported

#### ONVIF Token Authentication
- **Note**: Not all cameras support WS-UsernameToken
- **Fallback**: Use digest authentication for media streams

### Performance Issues

#### High Latency
- **Causes**: Network congestion, large buffer sizes
- **Solutions**:
  1. Reduce `latency` property value
  2. Use lower resolution sub-stream
  3. Optimize network path

#### Frame Drops
- **Causes**: CPU overload, network issues
- **Solutions**:
  1. Use hardware decoding if available
  2. Reduce stream resolution/framerate
  3. Check network stability

## Testing Recommendations

### Pre-Deployment Testing
1. Test with actual camera model and firmware
2. Verify all required features work
3. Test under expected network conditions
4. Document any workarounds needed

### Compatibility Matrix Template
```toml
[[cameras]]
name = "Camera Model"
vendor = "Manufacturer"
model = "Model Number"
firmware = "Version"
tested_date = "2025-01-01"
rtsp_url = "rtsp://..."
working_features = ["h264", "audio", "ptz"]
known_issues = ["Issue 1", "Issue 2"]
workarounds = ["Workaround 1", "Workaround 2"]
```

## Contributing

If you discover new camera quirks or solutions:
1. Test thoroughly with specific camera model
2. Document firmware version
3. Provide clear reproduction steps
4. Submit PR with updates to this document

## References

- [ONVIF Specifications](https://www.onvif.org/specs/)
- [Axis VAPIX Documentation](https://www.axis.com/vapix-library/)
- [Hikvision ISAPI Documentation](https://www.hikvision.com/en/support/download/sdk/)
- [Dahua HTTP API](http://www.dahuasecurity.com/support/downloadCenter)