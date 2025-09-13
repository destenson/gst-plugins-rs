# PRP-RTSP-52: Server Interaction Signals Implementation

## Overview
Implement server interaction signal (`handle-request`) to match original rtspsrc server-initiated request handling capabilities. This signal allows applications to handle RTSP requests sent from the server to the client.

## Context
The original rtspsrc provides the `handle-request` signal which is emitted when the server sends an RTSP request to the client (rather than the typical client-to-server request flow). This is essential for advanced RTSP scenarios where servers need to send commands to clients, such as recording control or stream redirection.

## Research Context
- Original rtspsrc handle-request signal in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- RTSP RFC 2326 bidirectional request handling
- Server-initiated RTSP requests (ANNOUNCE, REDIRECT, etc.)
- GstRTSPMessage structure for request and response handling
- ONVIF and professional streaming server command capabilities

## Scope
This PRP implements ONLY the signal infrastructure:
1. Add `handle-request` signal: `void (GstElement, GstRTSPMessage *request, GstRTSPMessage *response)`
2. Add signal registration with request and response message parameters
3. Add signal emission placeholder function
4. Add RTSP message parameter validation

Does NOT implement:
- Actual server request detection or parsing
- RTSP message processing logic  
- Server request routing or handling
- Response message generation

## Implementation Tasks
1. Add server interaction signal definition to RtspSrc element class
2. Register `handle-request` signal with request and response GstRTSPMessage parameters
3. Add signal emission placeholder function for server request handling
4. Add RTSP message parameter type validation
5. Document signal usage scenarios and parameter meanings
6. Add signal parameter null checking and validation
7. Document typical server-initiated request types

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Server interaction signal registration
- RTSP message type handling utilities

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_server_interaction_signal --all-targets --all-features -- --nocapture

# Handle Request Signal Test
cargo test test_handle_request_signal_registration --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
Element Signals:

  "handle-request" :  void user_function (GstElement * object,
                                          GstRTSPMessage * arg0,
                                          GstRTSPMessage * arg1,
                                          gpointer user_data);
```

## Signal Behavior Details
- **handle-request**: Emitted when server sends an RTSP request to the client

## Signal Parameter Details

### handle-request Signal
- **object**: The rtspsrc2 element
- **arg0**: GstRTSPMessage containing the server's request
- **arg1**: GstRTSPMessage for the client's response (to be filled by application)
- **return**: void
- **purpose**: Allow application to handle server-initiated RTSP requests

## Application Usage Examples

### Basic Request Handling
```c
g_signal_connect (rtspsrc, "handle-request", G_CALLBACK (handle_request_callback), NULL);

static void handle_request_callback (GstElement *element, GstRTSPMessage *request, 
                                     GstRTSPMessage *response, gpointer data) {
    GstRTSPMethod method;
    const gchar *uri;
    
    gst_rtsp_message_get_method (request, &method);
    gst_rtsp_message_get_uri (request, &uri);
    
    g_print ("Server sent %s request for %s\n", 
             gst_rtsp_method_as_text (method), uri);
    
    // Build appropriate response
    switch (method) {
        case GST_RTSP_ANNOUNCE:
            handle_announce_request (request, response);
            break;
        case GST_RTSP_REDIRECT:
            handle_redirect_request (request, response);
            break;
        default:
            gst_rtsp_message_init_response (response, GST_RTSP_STS_NOT_IMPLEMENTED, 
                                           NULL, request);
            break;
    }
}
```

### ANNOUNCE Request Handling
```c
static void handle_announce_request (GstRTSPMessage *request, GstRTSPMessage *response) {
    const gchar *content_type;
    guint8 *data;
    guint size;
    
    // Get SDP from ANNOUNCE
    gst_rtsp_message_get_header (request, GST_RTSP_HDR_CONTENT_TYPE, &content_type, 0);
    gst_rtsp_message_get_body (request, &data, &size);
    
    if (g_strcmp0 (content_type, "application/sdp") == 0) {
        // Process SDP announcement from server
        process_server_sdp (data, size);
        gst_rtsp_message_init_response (response, GST_RTSP_STS_OK, NULL, request);
    } else {
        gst_rtsp_message_init_response (response, GST_RTSP_STS_UNSUPPORTED_MEDIA_TYPE, 
                                       NULL, request);
    }
}
```

## Server-Initiated Request Types

### ANNOUNCE Requests
- Server provides new SDP session description
- Used for session parameter updates
- Common in recording and streaming control scenarios

### REDIRECT Requests  
- Server requests client to connect to different URL
- Used for load balancing and server migration
- Contains new location in response header

### Custom Methods
- Proprietary server commands
- ONVIF-specific requests
- Vendor extensions to RTSP protocol

## Response Requirements
- Applications must build valid RTSP responses
- Response status codes should be appropriate
- Required headers must be included
- Empty response body is acceptable for most methods

## Common Response Status Codes
- **200 OK**: Request handled successfully
- **501 Not Implemented**: Method not supported by client
- **415 Unsupported Media Type**: Content type not supported
- **400 Bad Request**: Malformed request from server

## Professional Use Cases
- **Recording control**: Server commands recording start/stop
- **Stream switching**: Server redirects to different quality streams  
- **Load balancing**: Server moves client to less loaded server
- **Session updates**: Server provides updated SDP parameters

## Dependencies
- **RTSP types**: GstRTSPMessage for request and response handling
- **Message utilities**: RTSP message parsing and building functions

## Success Criteria
- [ ] handle-request signal visible in gst-inspect output
- [ ] Signal accepts two GstRTSPMessage parameters correctly
- [ ] Signal registration succeeds without errors
- [ ] Signal can be connected from application code
- [ ] RTSP message parameter validation works correctly
- [ ] Signal emission function is prepared (placeholder)
- [ ] No actual server request handling logic implemented (out of scope)

## Risk Assessment
**MEDIUM RISK** - RTSP message parameter handling requires careful type management.

## Estimated Effort
3-4 hours (RTSP message type integration)

## Confidence Score
7/10 - RTSP message parameters add moderate complexity but follow established patterns.