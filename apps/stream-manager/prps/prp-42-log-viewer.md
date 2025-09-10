# PRP-42: Real-time Log Viewer

## Overview
Create a powerful log viewing interface with real-time streaming, filtering, and search capabilities for system and stream logs.

## Context
- Logs streamed via WebSocket
- Multiple log levels (debug, info, warn, error)
- Need filtering by component and stream
- Support for large log volumes
- Uses Deno for frontend tooling

## Requirements
1. Real-time log streaming
2. Log level filtering
3. Component/stream filtering
4. Full-text search
5. Time range selection
6. Log export functionality
7. Syntax highlighting
8. Virtual scrolling
9. Log tail mode

## Implementation Tasks
1. Create LogViewer page component
2. Implement WebSocket log streaming
3. Create virtual scrolling list
4. Add log level filter buttons
5. Implement component/stream selector
6. Create search bar with highlighting
7. Add time range picker
8. Implement log export functionality
9. Create syntax highlighting for JSON logs
10. Add tail mode toggle
11. Implement log line wrapping toggle
12. Create context menu for log lines
13. Add clipboard copy functionality
14. Implement log statistics panel

## Log Features
- Filters: Level, Component, Stream, Time Range
- Search: Regex and plain text
- Display: Line numbers, timestamps, colored levels
- Actions: Copy, Export, Share
- Modes: Tail, Pause, Historical

## Resources
- React Window for virtualization: https://github.com/bvaughn/react-window
- Ansi-to-html: https://github.com/drudru/ansi_up
- Regular expressions: https://regexr.com/
- Log4j pattern reference: https://logging.apache.org/log4j/2.x/manual/layouts.html

## Validation Gates
```bash
cd apps/stream-manager/web-ui

# Using Deno
deno task dev

# Test log viewer features:
# 1. Logs stream in real-time
# 2. Filters work correctly
# 3. Search highlights matches
# 4. Virtual scrolling handles 10000+ lines
# 5. Export creates valid log file
# 6. Tail mode follows new logs
# 7. Time range limits displayed logs
# 8. Copy to clipboard works

# Run tests with Deno
deno task test

# Performance testing with large logs
deno task test:performance
```

## Success Criteria
- Log viewer displays logs in real-time
- Filters reduce displayed logs correctly
- Search finds and highlights matches
- Virtual scrolling maintains performance
- Export provides usable log files
- Tail mode auto-scrolls to new logs
- Mobile view is functional
- Can handle 10,000+ log lines

## Dependencies
- PRP-32 (Base layout) must be completed
- PRP-34 (WebSocket client) must be completed
- PRP-35 (Authentication) must be completed

## Performance Considerations
- Virtual scrolling for large log volumes
- Debounce search input
- Limit WebSocket message rate
- Use Web Workers for search in large logs
- Implement log rotation/cleanup

## Estimated Effort
3 hours

## Confidence Score
8/10 - Virtual scrolling and WebSocket streaming are well-established patterns