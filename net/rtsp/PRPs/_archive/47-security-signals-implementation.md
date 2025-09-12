# PRP-RTSP-47: Security Signals Implementation

## Overview
Implement security-related signals (`accept-certificate`, `before-send`, `request-rtcp-key`, `request-rtp-key`) to match original rtspsrc security and encryption capabilities.

## Context  
The original rtspsrc provides security signals for TLS certificate validation (`accept-certificate`), request modification (`before-send`), and RTP/RTCP encryption key management (`request-rtcp-key`, `request-rtp-key`). These signals are critical for secure streaming and application-controlled encryption.

## Research Context
- Original rtspsrc security signals in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- GIO TLS certificate validation workflow
- RTSP request/response message modification
- SRTP/SRTCP key management in GStreamer
- GstCaps for encryption key parameters

## Scope
This PRP implements ONLY the signal infrastructure:
1. Add `accept-certificate` signal: `gboolean (GstElement, GTlsConnection, GTlsCertificate, GTlsCertificateFlags)`
2. Add `before-send` signal: `gboolean (GstElement, GstRTSPMessage)`  
3. Add `request-rtcp-key` signal: `GstCaps (GstElement, guint)`
4. Add `request-rtp-key` signal: `GstCaps (GstElement, guint)`

Does NOT implement:
- Actual certificate validation logic
- RTSP message modification processing
- SRTP/SRTCP key generation or management
- Signal emission at security checkpoints

## Implementation Tasks
1. Add security signal definitions to RtspSrc element class
2. Register `accept-certificate` signal with TLS types and boolean return
3. Register `before-send` signal with GstRTSPMessage parameter and boolean return
4. Register `request-rtcp-key` signal with stream ID parameter and GstCaps return
5. Register `request-rtp-key` signal with stream ID parameter and GstCaps return
6. Add signal emission placeholder functions  
7. Document security signal usage and parameter meanings

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Security signal registration
- May need GIO and RTSP message type imports

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_security_signals --all-targets --all-features -- --nocapture

# Security Signal Test
cargo test test_security_signal_registration --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
Element Signals:

  "accept-certificate" :  gboolean user_function (GstElement * object,
                                                  GTlsConnection * arg0,
                                                  GTlsCertificate * arg1,
                                                  GTlsCertificateFlags arg2,
                                                  gpointer user_data);

  "before-send" :  gboolean user_function (GstElement * object,
                                           GstRTSPMessage * arg0,  
                                           gpointer user_data);

  "request-rtcp-key" :  GstCaps * user_function (GstElement * object,
                                                 guint arg0,
                                                 gpointer user_data);

  "request-rtp-key" :  GstCaps * user_function (GstElement * object,
                                                guint arg0,  
                                                gpointer user_data);
```

## Signal Behavior Details
- **accept-certificate**: TLS certificate validation callback  
- **before-send**: RTSP message modification callback before transmission
- **request-rtcp-key**: RTCP encryption key retrieval for specific stream
- **request-rtp-key**: RTP encryption key retrieval for specific stream

## Signal Parameter Details

### accept-certificate Signal
- **object**: The rtspsrc2 element
- **arg0**: GTlsConnection for the TLS connection
- **arg1**: GTlsCertificate presented by server
- **arg2**: GTlsCertificateFlags indicating validation issues
- **return**: boolean (true = accept certificate, false = reject)

### before-send Signal  
- **object**: The rtspsrc2 element
- **arg0**: GstRTSPMessage about to be sent
- **return**: boolean (true = send message, false = cancel send)

### request-rtcp-key Signal
- **object**: The rtspsrc2 element  
- **arg0**: Stream index (unsigned int)
- **return**: GstCaps with SRTCP key parameters

### request-rtp-key Signal
- **object**: The rtspsrc2 element
- **arg0**: Stream index (unsigned int)
- **return**: GstCaps with SRTP key parameters

## Application Usage Examples

### Certificate Validation
```c
g_signal_connect (rtspsrc, "accept-certificate", G_CALLBACK (accept_cert_callback), NULL);

static gboolean accept_cert_callback (GstElement *element, GTlsConnection *conn, 
                                      GTlsCertificate *cert, GTlsCertificateFlags flags, 
                                      gpointer data) {
    // Custom certificate validation logic
    return (flags & G_TLS_CERTIFICATE_UNKNOWN_CA) == 0; // Accept known CAs only
}
```

### Message Modification
```c  
g_signal_connect (rtspsrc, "before-send", G_CALLBACK (before_send_callback), NULL);

static gboolean before_send_callback (GstElement *element, GstRTSPMessage *message, gpointer data) {
    // Modify message headers if needed
    gst_rtsp_message_add_header (message, GST_RTSP_HDR_USER_AGENT, "MyApp/1.0");
    return TRUE; // Send the message
}
```

## Security Use Cases
- **Custom certificate validation**: Accept self-signed or enterprise certificates
- **Request authentication**: Add custom authentication headers
- **Encryption key management**: Provide SRTP/SRTCP keys from external sources
- **Message filtering**: Block or modify specific RTSP requests

## Dependencies
- **GIO types**: GTlsConnection, GTlsCertificate, GTlsCertificateFlags
- **RTSP types**: GstRTSPMessage  
- **Caps types**: GstCaps for key parameters

## Success Criteria
- [ ] All four security signals visible in gst-inspect output
- [ ] Signal parameter types match original rtspsrc exactly
- [ ] Boolean return signals work correctly
- [ ] GstCaps return signals handle null returns properly
- [ ] Signals can be connected from application code
- [ ] Signal emission functions are prepared (placeholder)
- [ ] No actual security processing implemented (out of scope)

## Risk Assessment
**MEDIUM-HIGH RISK** - Complex parameter types and return value handling.

## Estimated Effort
4-5 hours (complex parameter types and return handling)

## Confidence Score  
6/10 - Security signal types and return value handling add significant complexity.