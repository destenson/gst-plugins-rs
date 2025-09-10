# PRP-RTSP-12: TCP Connection Pooling and Reuse

## Overview
Implement connection pooling for TCP connections to reduce latency when connecting to multiple streams from the same server.

## Current State
- Creates new TCP connection for each stream
- No connection reuse between elements
- Unnecessary handshake overhead
- No connection sharing mechanism

## Success Criteria
- [ ] Pool TCP connections per server
- [ ] Reuse connections for multiple streams
- [ ] Handle connection lifecycle properly
- [ ] Clean up idle connections
- [ ] Tests verify connection reuse

## Technical Details

### Connection Pool Design
1. Global connection pool (lazy_static)
2. Key by server host:port
3. Connection checkout/checkin pattern
4. Idle timeout for cleanup
5. Max connections per server limit

### Pool Management
- Thread-safe connection storage
- Health checking before reuse
- Automatic reconnection if stale
- Reference counting for sharing
- Graceful shutdown handling

### Configuration
- enable-connection-pooling (default: true)
- max-connections-per-server (default: 4)
- idle-connection-timeout (default: 60s)

## Implementation Blueprint
1. Create connection_pool module
2. Design ConnectionPool struct with Arc<Mutex>
3. Implement checkout/checkin methods
4. Add health check for connections
5. Integrate with existing TCP code
6. Add idle connection reaper
7. Handle element cleanup
8. Test concurrent access

## Resources
- HTTP connection pooling patterns: https://docs.rs/hyper/latest/hyper/client/index.html
- deadpool crate for pooling: https://docs.rs/deadpool/
- Local patterns: net/reqwest connection handling

## Validation Gates
```bash
# Test connection pooling
cargo test -p gst-plugin-rtsp connection_pool -- --nocapture

# Test concurrent access
cargo test -p gst-plugin-rtsp pool_concurrent -- --nocapture

# Verify connection reuse
cargo test -p gst-plugin-rtsp pool_reuse -- --nocapture
```

## Dependencies
- None (enhances existing TCP handling)

## Estimated Effort
4 hours

## Risk Assessment
- Medium complexity - thread safety concerns
- Challenge: Managing shared connection state

## Success Confidence Score
7/10 - Common pattern but needs careful implementation