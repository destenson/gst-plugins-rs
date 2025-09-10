# PRP-RTSP-21: HTTP/SOCKS Proxy Support

## Overview
Implement proxy support for RTSP connections, enabling streaming through corporate firewalls and proxy servers.

## Current State
- Listed as missing: "Proxy support"
- Direct connections only
- Cannot work in proxy-only networks
- Common requirement in enterprises

## Success Criteria
- [ ] HTTP proxy support
- [ ] SOCKS5 proxy support
- [ ] Proxy authentication
- [ ] Auto-detect from environment
- [ ] Tests verify proxy connections

## Technical Details

### Proxy Types
1. HTTP CONNECT proxy (for TCP)
2. SOCKS5 proxy
3. Transparent proxy detection
4. Proxy authentication (Basic/Digest)
5. Environment variable detection

### Configuration
- proxy property: proxy URL
- proxy-id: username
- proxy-pw: password  
- Auto-detect from http_proxy/https_proxy env

### Connection Flow
1. Connect to proxy server
2. Send CONNECT method (HTTP) or SOCKS handshake
3. Authenticate if required
4. Establish tunnel to RTSP server
5. Continue normal RTSP flow

## Implementation Blueprint
1. Add proxy configuration properties
2. Create proxy module
3. Implement HTTP CONNECT
4. Implement SOCKS5 protocol
5. Add proxy authentication
6. Detect environment variables
7. Integrate with connection code
8. Test with mock proxy

## Resources
- HTTP CONNECT: https://datatracker.ietf.org/doc/html/rfc7231#section-4.3.6
- SOCKS5: https://datatracker.ietf.org/doc/html/rfc1928
- tokio-socks: https://docs.rs/tokio-socks/
- Local ref: net/reqwest proxy handling

## Validation Gates
```bash
# Test HTTP proxy
cargo test -p gst-plugin-rtsp http_proxy -- --nocapture

# Test SOCKS5 proxy
cargo test -p gst-plugin-rtsp socks_proxy -- --nocapture

# Test proxy authentication
cargo test -p gst-plugin-rtsp proxy_auth -- --nocapture
```

## Dependencies
- PRP-RTSP-03/04 (Authentication) - for proxy auth

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - multiple proxy protocols
- Challenge: Testing various proxy types

## Success Confidence Score
7/10 - Well-defined protocols with existing libraries