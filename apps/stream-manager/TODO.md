# TODO

Current technical debt and pending features for the Stream Manager.

## 🔥 Critical Priority

### Core Application Integration
- [ ] Update main.rs to remove completed TODOs (lines 78-81)
  - ✅ Pipeline manager (PRP-04) - COMPLETED
  - ✅ Stream manager (PRP-09) - COMPLETED  
  - ✅ REST API server (PRP-11) - COMPLETED
  - ✅ Health monitoring (PRP-10) - COMPLETED
- [ ] Replace simple event loop in main.rs:83 with proper service orchestration
- [ ] Implement actual readiness check in API routes (src/api/routes.rs:45)

### Configuration & Authentication
- [ ] Implement config update logic (src/api/routes.rs:60) - **PRP-15**
- [ ] Implement config reload logic (src/api/routes.rs:67) - **PRP-15**
- [ ] Implement actual authentication logic (src/api/middleware.rs:224)

### Recording System
- [ ] Get actual recording state from recording branch (src/manager/mod.rs:381)
- [ ] Implement actual start recording (src/manager/mod.rs:418)
- [ ] Implement actual stop recording (src/manager/mod.rs:432)

### Metrics & Monitoring
- [ ] Add actual latency measurement (src/manager/mod.rs:256)

## 📋 High Priority - Remaining PRPs

### PRP Implementation Status
- [x] **PRP-01 through PRP-12**: ✅ COMPLETED
- [x] **PRP-13**: ✅ COMPLETED (Metrics and Statistics)
- [ ] **PRP-14**: WebSocket Events
- [ ] **PRP-15**: Config Hot Reload  
- [ ] **PRP-16**: Storage Management
- [ ] **PRP-17**: Disk Rotation
- [ ] **PRP-18**: Systemd Service
- [ ] **PRP-19**: Error Recovery
- [ ] **PRP-20**: State Persistence  
- [ ] **PRP-21**: NVIDIA Inference Branch
- [ ] **PRP-22**: CPU Inference Fallback
- [ ] **PRP-23**: Telemetry Tracing
- [ ] **PRP-24**: Backup & Disaster Recovery
- [ ] **PRP-25**: RTSP Server Proxy
- [ ] **PRP-26**: WebRTC Server  
- [ ] **PRP-27**: WHIP/WHEP Protocols
- [ ] **PRP-28**: Integration Testing

### Module Stubs (Need Implementation)
- [ ] **src/webrtc/mod.rs**: WebRTC functionality (PRP-26)
- [ ] **src/telemetry/mod.rs**: Telemetry and tracing (PRP-23)  
- [ ] **src/rtsp/mod.rs**: RTSP server proxy (PRP-25)
- [ ] **src/recovery/mod.rs**: Error recovery system (PRP-19)
- [ ] **src/service/mod.rs**: Systemd service integration (PRP-18)
- [ ] **src/storage/mod.rs**: Storage management (PRP-16)
- [ ] **src/backup/mod.rs**: Backup system (PRP-24)
- [ ] **src/inference/mod.rs**: AI inference pipelines (PRP-21, PRP-22)
- [ ] **src/database/mod.rs**: State persistence (PRP-20)

## 🔧 Medium Priority - Enhancements

### Performance & Reliability
- [ ] Optimize performance for handling larger number of concurrent streams.
- [ ] Improve error handling and logging throughout the application.
- [ ] Add more comprehensive tests, including integration tests.
- [ ] Handle unused variables (warnings about _error, _state parameters)

### Protocol Support
- [ ] Add support for more streaming protocols (e.g., SRT, RTMP).
- [ ] Add support for more video formats and codecs.
- [ ] Add support for more streaming platforms (e.g., Twitch, YouTube, Facebook Live).

### Storage & Data Management  
- [ ] Add support for additional storage backends (e.g., cloud storage).
- [ ] Implement a backup and restore system for configurations and recorded data.
- [ ] Add support for more notification channels (e.g., SMS, Slack).

## 🌟 Low Priority - Future Features

### User Experience
- [ ] Implement a web-based dashboard for monitoring and managing streams.
- [ ] Improve documentation, including more examples and use cases.
- [ ] Implement advanced scheduling and automation features (e.g., scheduled recordings).
- [ ] Implement user authentication and authorization for the REST API.

### Extensibility
- [ ] Add support for more inference models and frameworks.
- [ ] Add support for more advanced inference features (e.g., multi-model pipelines).
- [ ] Implement a plugin system for extending functionality.
- [ ] Improve the configuration system, including support for dynamic reloading.

## 📊 Technical Debt

### Code Quality
- [ ] Clean up "for now" and "placeholder" comments
- [ ] Remove temporary test configurations scattered throughout tests
- [ ] Fix unused variable warnings (various files)
- [ ] Consolidate max_reconnect_attempts configurations
- [ ] Review and standardize error handling patterns

### Dependencies  
- [ ] Evaluate OpenTelemetry dependency (marked "for later PRPs")
- [ ] Review tempfile usage in tests for consistency

---

**Last Updated**: After PRP-13 completion  
**Completed PRPs**: 1-13 (13 of 28 total)  
**Next Priority**: PRP-14 (WebSocket Events) or PRP-15 (Config Hot Reload)