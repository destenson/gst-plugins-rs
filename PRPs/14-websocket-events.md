# PRP-14: WebSocket Event Streaming

## Overview
Implement WebSocket support for real-time event streaming including health updates, statistics, and system notifications.

## Context
- Clients need real-time updates without polling
- Must handle multiple concurrent connections
- Should support event filtering
- Need graceful connection handling

## Requirements
1. Add WebSocket server to Actix
2. Implement event broadcasting system
3. Create event subscription model
4. Handle connection lifecycle
5. Add event filtering

## Implementation Tasks
1. Create src/api/websocket.rs module
2. Define WebSocket actor:
   - Connection state
   - Event subscriptions
   - Send queue
3. Define event types enum:
   - StreamAdded
   - StreamRemoved
   - HealthChanged
   - RecordingStarted/Stopped
   - SystemAlert
4. Implement WebSocket handler:
   - Upgrade HTTP to WebSocket
   - Create actor per connection
   - Handle ping/pong
5. Create event broadcaster:
   - Collect events from system
   - Fan out to connections
   - Handle backpressure
6. Add subscription management:
   - Subscribe to event types
   - Filter by stream ID
   - Unsubscribe handling
7. Implement connection cleanup on disconnect

## Validation Gates
```bash
# Test WebSocket upgrade
cargo test --package stream-manager api::websocket::tests

# Verify event broadcasting
cargo test websocket_broadcast

# Check connection handling
cargo test websocket_lifecycle
```

## Dependencies
- PRP-11: API server for WebSocket endpoint
- PRP-10: Health monitoring for events

## References
- Actix WebSocket: https://actix.rs/docs/websockets/
- Event patterns: Search for "websocket" in examples
- Actor model: Actix actor documentation

## Success Metrics
- WebSocket connections established
- Events broadcast to all clients
- Subscription filtering works
- Clean disconnection handling

**Confidence Score: 7/10**