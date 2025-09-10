# PRP-RTSP-43: Proxy and HTTP Tunneling Properties Implementation  

## Overview
Implement proxy and HTTP tunneling properties (`proxy`, `proxy-id`, `proxy-pw`, `extra-http-request-headers`) to match original rtspsrc network traversal capabilities for restrictive network environments.

## Context
The original rtspsrc provides HTTP proxy support and tunneling capabilities through properties for proxy URL, authentication credentials, and custom HTTP headers. These are essential for RTSP streaming through corporate firewalls and restricted networks.

## Research Context
- Original rtspsrc proxy properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- HTTP proxy authentication mechanisms (RFC 7617)
- RTSP over HTTP tunneling (RFC 2326 Appendix C)
- GstStructure for HTTP header storage
- Proxy URL format: `[http://][user:passwd@]host[:port]`

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `proxy` string property for proxy URL (default: null)
2. Add `proxy-id` string property for proxy username (default: null)  
3. Add `proxy-pw` string property for proxy password (default: null)
4. Add `extra-http-request-headers` GstStructure property (default: null)
5. Add URL format validation for proxy property

Does NOT implement:
- Actual HTTP proxy connection logic
- Proxy authentication protocol
- HTTP tunneling establishment  
- HTTP header injection into requests

## Implementation Tasks
1. Add proxy and tunneling fields to RtspSrcSettings struct
2. Implement `proxy` string property with URL format validation
3. Implement `proxy-id` and `proxy-pw` string properties for authentication
4. Implement `extra-http-request-headers` GstStructure property
5. Add proxy URL parsing and validation logic
6. Add property change restrictions (changeable only in NULL or READY state)
7. Document proxy URL format and header structure format

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and validation
- May need URL parsing utilities for proxy format validation

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_proxy_properties --all-targets --all-features -- --nocapture

# Proxy URL Validation Test
cargo test test_proxy_url_validation --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
proxy               : Proxy settings for HTTP tunneling. Format: [http://][user:passwd@]host[:port]
                      flags: readable, writable, changeable only in NULL or READY state
                      String. Default: null

proxy-id            : HTTP proxy URI user id for authentication  
                      flags: readable, writable, changeable only in NULL or READY state
                      String. Default: null

proxy-pw            : HTTP proxy URI user password for authentication
                      flags: readable, writable, changeable only in NULL or READY state  
                      String. Default: null

extra-http-request-headers: Extra headers to append to HTTP requests when in tunneled mode
                            flags: readable, writable, changeable only in NULL or READY state
                            Boxed pointer of type "GstStructure"
```

## Property Behavior Details
- **proxy**: HTTP proxy URL for tunneling RTSP connections through firewalls
- **proxy-id**: Username for HTTP proxy authentication (Basic/Digest)
- **proxy-pw**: Password for HTTP proxy authentication  
- **extra-http-request-headers**: Custom HTTP headers for tunneling requests

## Proxy URL Format Examples
Valid proxy URL formats:
- `"proxy.company.com:8080"` - Basic proxy host and port
- `"http://proxy.company.com:8080"` - Explicit HTTP protocol
- `"http://user:pass@proxy.company.com:8080"` - With embedded credentials
- `"proxy.company.com"` - Default port 8080 if not specified

## Extra Headers Structure Format
GstStructure format for HTTP headers:
```c
headers = gst_structure_new ("headers",
    "User-Agent", G_TYPE_STRING, "MyApp/1.0",
    "X-Custom-Header", G_TYPE_STRING, "custom-value",
    NULL);
```

## Proxy Authentication Flow
1. Client connects to proxy server
2. If authentication required, proxy returns 407 Proxy Authentication Required  
3. Client uses proxy-id/proxy-pw for authentication
4. Proxy establishes tunnel to RTSP server
5. RTSP communication flows through tunnel

## Dependencies
- **GstStructure**: For extra HTTP headers property
- **URL parsing**: For proxy URL validation

## Success Criteria
- [ ] All four properties visible in gst-inspect output
- [ ] proxy property validates URL format correctly
- [ ] proxy-id/proxy-pw string properties work correctly
- [ ] extra-http-request-headers accepts GstStructure values
- [ ] Properties changeable only in NULL/READY states
- [ ] Property defaults match original rtspsrc (all null)
- [ ] No actual proxy connection logic implemented (out of scope)

## Risk Assessment
**MEDIUM RISK** - URL validation and GstStructure property handling.

## Estimated Effort
3-4 hours (URL validation and structure property)

## Confidence Score  
7/10 - URL parsing and GstStructure properties add moderate complexity.