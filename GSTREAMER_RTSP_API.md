# GStreamer RTSP API Research Report

## Executive Summary

The gstreamer-rtsp crate provides Rust bindings for the GStreamer RTSP library, but **critical functionality is missing**. Most notably, `RTSPConnection` and `RTSPTransport` - the core types for RTSP client implementation - lack safe Rust bindings despite being available in the FFI layer.

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

### Missing Bindings (Available in FFI but Not Safe Rust)

Critical types present in `gstreamer-rtsp-sys` but lacking safe bindings:

#### RTSPConnection
The most critical missing type. The C implementation (gstrtspsrc.c) heavily uses:
- `gst_rtsp_connection_create()`
- `gst_rtsp_connection_connect_with_response_usec()`
- `gst_rtsp_connection_send_usec()`
- `gst_rtsp_connection_receive_usec()`
- `gst_rtsp_connection_close()`
- `gst_rtsp_connection_free()`
- `gst_rtsp_connection_set_tls_*()` functions
- `gst_rtsp_connection_set_proxy()`
- `gst_rtsp_connection_flush()`
- `gst_rtsp_connection_reset_timeout()`

#### RTSPTransport
Another critical missing type for transport negotiation:
- `gst_rtsp_transport_new()`
- `gst_rtsp_transport_init()`
- `gst_rtsp_transport_free()`
- `gst_rtsp_transport_as_text()`

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

## Implementation Approach

### Option 1: Create Manual Bindings (Recommended)

Since RTSPConnection and RTSPTransport are not exposed through auto-generated bindings, manual bindings need to be created:

1. **Create wrapper types** in gstreamer-rtsp/src/
2. **Implement safe wrappers** for FFI functions
3. **Handle memory management** with proper ownership semantics
4. **Integrate with GIO** for async operations

### Option 2: Fix gstreamer-rs Upstream

As suggested, contributing the missing bindings to gstreamer-rs would benefit the entire ecosystem:

1. **Add manual bindings** in gstreamer-rtsp/src/
2. **Submit PR** to gitlab.freedesktop.org/gstreamer/gstreamer-rs
3. **Use git dependency** until merged

### Option 3: Direct FFI Usage (Not Recommended)

Use unsafe FFI directly from gstreamer-rtsp-sys, but this approach:
- Requires extensive unsafe code
- Loses type safety benefits
- Increases maintenance burden

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

### Creating an RTSP Connection (C Code Reference)
```c
GstRTSPConnection *conn;
gst_rtsp_connection_create(url, &conn);
gst_rtsp_connection_connect_with_response_usec(conn, timeout, &response);
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

## Required Work

### Immediate Tasks
1. **Create RTSPConnection bindings**
   - Wrapper struct with proper lifetime management
   - Safe methods for create, connect, send, receive
   - Integration with GIO async patterns

2. **Create RTSPTransport bindings**
   - Transport negotiation support
   - Parameter parsing and serialization

3. **Implement async patterns**
   - MainLoop integration
   - Callback registration
   - Error propagation

### Long-term Tasks
1. **Upstream contribution** to gstreamer-rs
2. **Documentation** and examples
3. **Test coverage** for new bindings

## Conclusion

While gstreamer-rtsp provides basic RTSP types, the absence of RTSPConnection and RTSPTransport bindings makes it impossible to implement a full RTSP client using only safe Rust. Manual bindings must be created, either locally or preferably as an upstream contribution to gstreamer-rs.

The migration from Tokio to GStreamer's RTSP library requires not just replacing network code, but also adopting GLib's async patterns and MainLoop architecture. However, this would provide:
- Proven, battle-tested RTSP implementation
- Built-in features (keep-alive, reconnection, TLS)
- Better integration with GStreamer ecosystem
- Reduced code complexity

## Next Steps

1. **Decision Required**: Local bindings vs upstream contribution
2. **Prototype**: Create minimal RTSPConnection binding
3. **Validate**: Test async I/O patterns with MainLoop
4. **Implement**: Full binding implementation
5. **Migrate**: Port rtspsrc from Tokio to GStreamer RTSP