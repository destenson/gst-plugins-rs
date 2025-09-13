# GStreamer RTSP API Research Report

## Executive Summary

**UPDATE**: The missing bindings have been added locally to gstreamer-rs! `RTSPConnection` and `RTSPTransport` are now available with safe Rust bindings, including convenient builder patterns for easy construction and configuration.

## Current Status of gstreamer-rtsp Bindings

### Available Types in Safe Rust Bindings

The following types are exposed through the gstreamer-rtsp crate (v0.24.0):

#### Core Structures
- `RTSPUrl` - Represents an RTSP URL
  - Methods: `parse()`, `request_uri()`, `set_port()`, `decode_path_components()`
- `RTSPMessage` - RTSP message handling (minimal documentation)
- `RTSPAuthCredential` - Authentication credential handling
- `RTSPAuthParam` - Authentication parameters

#### Enumerations
- `RTSPAuthMethod` - Authentication methods
- `RTSPFamily` - Protocol family
- `RTSPHeaderField` - RTSP header fields
- `RTSPMsgType` - Message types
- `RTSPRangeUnit` - Range units
- `RTSPResult` - Result codes
- `RTSPState` - Connection states
- `RTSPStatusCode` - Status codes
- `RTSPTimeType` - Time types

#### Flags
- `RTSPEvent` - Event types
- `RTSPLowerTrans` - Lower transport protocols
- `RTSPMethod` - RTSP methods (OPTIONS, DESCRIBE, etc.)
- `RTSPProfile` - Transport profiles
- `RTSPTransMode` - Transport modes

### Newly Added Bindings (Now Available!)

The following critical types have been added to the local gstreamer-rs with safe Rust bindings:

#### RTSPConnection
Full safe bindings now available in `gstreamer_rtsp::rtsp_connection`:
- `RTSPConnection::create()` - Create new connection from URL
- `connect()`, `connect_with_response()` - Connect to server
- `send()`, `receive()` - Send/receive RTSP messages
- `close()` - Close connection
- TLS configuration methods
- Proxy support
- Timeout management
- Builder pattern via `RTSPConnectionBuilder` for convenient construction

#### RTSPTransport  
Complete transport negotiation support in `gstreamer_rtsp::rtsp_transport`:
- `RTSPTransport::new()` - Create new transport
- `set_trans()`, `set_profile()`, `set_lower_transport()` - Configure transport
- `as_text()` - Serialize to string
- `RTSPRange` - Port range configuration
- Builder pattern via `RTSPTransportBuilder` for fluent API

#### Builder Patterns
Convenient builders in `gstreamer_rtsp::builders`:
- `RTSPConnectionBuilder` - Fluent API for connection configuration
- `RTSPTransportBuilder` - Easy transport setup
- Helper functions for common patterns

## GIO Async Patterns in gstreamer-rs

### MainLoop and MainContext Usage

The GStreamer Rust bindings integrate with GLib's MainLoop for async operations:

```rust
// Standard pattern for MainLoop usage
let main_loop = glib::MainLoop::new(None, false);
let main_context = glib::MainContext::default();

// For custom context
let main_loop = glib::MainLoop::new(Some(&main_context), false);
```

### Async I/O Through GIO

The RTSP library uses GIO for async network operations:
- `gio::GSocket` - Socket operations
- `gio::GCancellable` - Cancellation support
- `gio::GAsyncResult` - Async result handling
- `gio::GAsyncReadyCallback` - Async callbacks

### TLS Support

TLS is handled through GIO types (available in FFI):
- `gio::GTlsConnection` - TLS connection management
- `gio::GTlsCertificate` - Certificate handling
- `gio::GTlsCertificateFlags` - Certificate validation flags
- `gio::GTlsDatabase` - Certificate database
- `gio::GTlsInteraction` - User interaction for TLS

## Implementation Status

### ✅ Completed: Local Bindings Created

The missing bindings have been successfully added to the local gstreamer-rs repository:

1. **RTSPConnection module** (`rtsp_connection.rs`)
   - Full safe wrapper around FFI types
   - Proper memory management with Drop trait
   - GIO integration for async operations
   - TLS and proxy configuration support

2. **RTSPTransport module** (`rtsp_transport.rs`)
   - Complete transport configuration API
   - Range and port management
   - Serialization support

3. **Builder patterns** (`builders.rs`)
   - RTSPConnectionBuilder for fluent connection setup
   - RTSPTransportBuilder for easy transport configuration
   - Helper functions for common patterns

### Next Phase: Migration Implementation

With the bindings now available, the rtspsrc can be migrated from Tokio to use these native GStreamer RTSP APIs.

## Migration Path from Tokio

### Current Architecture (Tokio-based)
- Uses tokio::net for TCP/UDP sockets
- Custom RTSP protocol implementation
- Manual keep-alive and timeout management
- Custom TLS handling

### Target Architecture (GStreamer RTSP)
- GIO-based async I/O
- Built-in RTSP protocol handling
- Automatic keep-alive management
- Integrated TLS through GIO

### Key Differences
1. **Event Loop**: Tokio reactor vs GLib MainLoop
2. **Async Model**: Futures vs GIO callbacks
3. **Error Handling**: Result<T,E> vs GstRTSPResult
4. **Memory Management**: Rust ownership vs GObject reference counting

## Example Patterns

### Creating an RTSP Connection (New Rust API)
```rust
use gstreamer_rtsp::{RTSPConnection, RTSPUrl};
use gstreamer_rtsp::builders::RTSPConnectionBuilder;

// Direct API
let (result, url) = RTSPUrl::parse("rtsp://localhost:554/test");
let url = url.unwrap();
let conn = RTSPConnection::create(&url)?;
conn.connect(timeout)?;

// Builder pattern (recommended)
let conn = RTSPConnectionBuilder::new("rtsp://localhost:554/test")?
    .auth(RTSPAuthMethod::Basic, "user", "pass")
    .proxy("proxy.example.com", 8080)
    .timeout(Duration::from_secs(5))
    .build()?;
```

### Configuring Transport
```rust
use gstreamer_rtsp::{RTSPTransport, RTSPTransMode, RTSPProfile, RTSPLowerTrans};
use gstreamer_rtsp::builders::RTSPTransportBuilder;

// Direct API
let mut transport = RTSPTransport::new()?;
transport.set_trans(RTSPTransMode::RTP);
transport.set_profile(RTSPProfile::AVP);
transport.set_lower_transport(RTSPLowerTrans::UDP);

// Builder pattern (recommended)
let transport = RTSPTransportBuilder::new()
    .trans(RTSPTransMode::RTP)
    .profile(RTSPProfile::AVP)
    .lower_transport(RTSPLowerTrans::UDP)
    .client_port_range(5000, 5001)
    .build()?;
```

### MainLoop Integration Pattern
```rust
let main_loop = glib::MainLoop::new(None, false);
let main_loop_clone = main_loop.clone();

// Set up bus watch for messages
bus.add_watch(move |_, msg| {
    match msg.view() {
        gst::MessageView::Eos(_) => main_loop_clone.quit(),
        _ => {}
    }
    glib::ControlFlow::Continue
});

main_loop.run();
```

## Migration Tasks

### Immediate Tasks
1. **Update Cargo.toml dependencies**
   - Point to local gstreamer-rs with new bindings
   - Remove unnecessary Tokio dependencies

2. **Refactor connection management**
   - Replace Tokio TCP/UDP sockets with RTSPConnection
   - Migrate from futures to GIO callbacks
   - Update error handling to use RTSPResult

3. **Implement GLib MainLoop integration**
   - Replace Tokio runtime with MainLoop
   - Convert async/await patterns to callback-based
   - Handle message dispatch through GIO

### Implementation Considerations
1. **Memory Management**
   - RTSPConnection uses GObject reference counting
   - Ensure proper cleanup with Drop implementations
   - Handle ownership transfers correctly

2. **Error Handling**
   - Map RTSPResult to Rust Result types
   - Preserve error context for debugging
   - Handle GIO-specific errors

3. **Testing Strategy**
   - Unit tests for individual components
   - Integration tests with real RTSP servers
   - Performance comparison with Tokio version

## Conclusion

With the successful addition of RTSPConnection and RTSPTransport bindings to the local gstreamer-rs, we now have all the necessary components to migrate rtspsrc from Tokio to the native GStreamer RTSP library. This provides:

- ✅ Complete RTSP protocol implementation
- ✅ Built-in keep-alive and reconnection
- ✅ Native TLS support through GIO
- ✅ Better GStreamer ecosystem integration
- ✅ Reduced code complexity
- ✅ Battle-tested implementation

## Next Steps

1. ✅ **COMPLETED**: Added RTSPConnection and RTSPTransport bindings
2. **IN PROGRESS**: Update documentation with new API examples
3. **TODO**: Begin migration of rtspsrc from Tokio to GStreamer RTSP
4. **TODO**: Implement MainLoop integration
5. **TODO**: Test with various RTSP servers
6. **FUTURE**: Consider upstream contribution to gstreamer-rs
