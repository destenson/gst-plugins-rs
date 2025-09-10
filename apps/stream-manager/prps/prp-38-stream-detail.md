# PRP-38: Stream Detail View

## Overview
Create a detailed stream view page with live preview, comprehensive metrics, controls, and configuration.

## Context
- Accessed from stream list by clicking on a stream
- Shows detailed information about a single stream
- Includes live preview if stream is active
- Real-time metrics and event updates
- Full control over stream operations

## Requirements
1. Stream information display
2. Live video preview
3. Real-time metrics charts
4. Stream control buttons
5. Recording management section
6. Event log for this stream
7. Configuration editor
8. Pipeline visualization
9. Error history

## Implementation Tasks
1. Create StreamDetail page component
2. Implement video player for live preview
3. Add stream info cards (URL, status, uptime)
4. Create metrics charts (bitrate, FPS, latency)
5. Implement control button bar
6. Add recording section with history
7. Create event log filtered by stream
8. Implement configuration editor panel
9. Add pipeline graph visualization
10. Create error/warning display
11. Implement health check results
12. Add breadcrumb navigation
13. Create share/embed functionality
14. Add fullscreen video option

## Page Sections
- Header (name, status, quick actions)
- Video Preview (with player controls)
- Metrics Dashboard (charts and gauges)
- Controls (start/stop/restart/record)
- Recording History (list of recordings)
- Configuration (editable settings)
- Event Log (filtered events)
- Pipeline View (optional advanced view)

## Resources
- Video.js player: https://videojs.com/
- HLS.js for streaming: https://github.com/video-dev/hls.js/
- Chart.js for metrics: https://www.chartjs.org/
- Monaco Editor for config: https://microsoft.github.io/monaco-editor/
- D3.js for pipeline graph: https://d3js.org/

## Validation Gates
```bash
cd apps/stream-manager/web-ui

# Run development server
deno run dev

# Test stream detail features:
# 1. Navigate to stream detail from list
# 2. Video preview loads and plays
# 3. Metrics update in real-time
# 4. Control buttons work correctly
# 5. Recording section shows history
# 6. Configuration can be edited and saved
# 7. Event log updates with new events
# 8. Fullscreen video works

# Run tests
deno test

# Type checking
deno run type-check
```

## Success Criteria
- Stream detail page loads completely
- Video preview plays without buffering issues
- Metrics charts update every second
- All control buttons function correctly
- Configuration changes save successfully
- Event log shows relevant events only
- Page handles stream offline state gracefully
- Mobile layout is usable

## Dependencies
- PRP-32 (Base layout) must be completed
- PRP-33 (API client) must be completed  
- PRP-34 (WebSocket client) must be completed
- PRP-37 (Stream list) recommended

## Technical Considerations
- Use HLS.js for adaptive streaming
- Implement player error recovery
- Cache metrics data for performance
- Lazy load video player and charts
- Handle stream URL authentication

## Estimated Effort
4 hours

## Confidence Score
7/10 - Video player integration and real-time charts require careful implementation
