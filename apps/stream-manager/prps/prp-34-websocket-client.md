# PRP-34: WebSocket Client Implementation

## Overview
Implement a robust WebSocket client for real-time event streaming from the Stream Manager backend.

## Context
- Backend WebSocket endpoint at `/api/events` (actix-ws)
- Events defined in src/api/websocket.rs
- Backend default port should be 8080 (configurable)
- Need automatic reconnection and event handling
- Must integrate with React components

## Requirements
1. WebSocket client with automatic reconnection
2. TypeScript types for all event types
3. Event subscription system for components
4. Connection state management
5. Reconnection with exponential backoff
6. Message queuing during disconnection
7. React hooks for WebSocket events
8. Debug logging for connection issues

## Implementation Tasks
1. Create TypeScript types from WebSocket event definitions
2. Implement WebSocket client class with reconnection logic
3. Add event emitter for typed events
4. Create connection state management
5. Implement exponential backoff for reconnection
6. Add message queue for offline mode
7. Create React context for WebSocket
8. Implement useWebSocket hook
9. Create useWebSocketEvent hook for specific events
10. Add connection status indicator component
11. Implement heartbeat/ping mechanism
12. Create debug panel for WebSocket traffic

## Event Types to Support
- stream.added
- stream.removed
- stream.health_changed
- recording.started
- recording.stopped
- statistics.update
- system.alert
- config.changed
- error.occurred

## Resources
- WebSocket API: https://developer.mozilla.org/en-US/docs/Web/API/WebSocket
- Reconnecting WebSocket: https://github.com/pladaria/reconnecting-websocket
- React WebSocket patterns: https://www.pluralsight.com/guides/using-websockets-in-react
- Socket.IO alternative: https://socket.io/docs/v4/client-initialization/

## Validation Gates
```bash
cd apps/stream-manager/web-ui

# TypeScript check
deno run type-check

# Run tests
deno test

# Test with backend (on port 8080)
VITE_API_PORT=8080 deno run dev

# Check WebSocket connection in browser console:
# - Should auto-connect on page load
# - Should show "WebSocket connected" message
# - Should reconnect after backend restart
```

## Success Criteria
- WebSocket connects automatically on app start
- Events are received and typed correctly
- Reconnection works after connection loss
- React components can subscribe to specific events
- Connection status is visible in UI
- No memory leaks from event listeners
- Works with configurable backend port

## Dependencies
- PRP-30 (Frontend setup) must be completed
- PRP-33 (API client) recommended for shared configuration

## Notes
- Backend port should default to 8080 instead of 3000
- Port should be configurable via environment variable
- WebSocket URL should be built from window.location for production

## Estimated Effort
3 hours

## Confidence Score
8/10 - WebSocket reconnection logic requires careful implementation
