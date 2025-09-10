# PRP-36: Dashboard Overview Page

## Overview
Create the main dashboard page showing system status, stream statistics, and key metrics at a glance.

## Context
- First page users see after login
- Must load quickly and show real-time updates
- Data from /api/status and WebSocket events
- Should be responsive and work on mobile
- Uses existing API client and WebSocket connections

## Requirements
1. System status overview cards
2. Active streams summary
3. Recording statistics
4. Storage usage visualization
5. Recent events feed
6. Quick action buttons
7. Real-time updates via WebSocket
8. Responsive grid layout

## Implementation Tasks
1. Create Dashboard page component
2. Implement status cards (online/offline/warning)
3. Create stream statistics widget
4. Add storage usage chart (pie/donut chart)
5. Implement recent events list with WebSocket
6. Create quick action buttons (add stream, start recording)
7. Add auto-refresh for statistics
8. Implement loading skeletons
9. Create error state displays
10. Add metric trend indicators (up/down arrows)
11. Implement data refresh controls
12. Add export data functionality

## Dashboard Widgets
- System Health (CPU, Memory, Network)
- Active Streams (count, health status)
- Recording Status (active recordings, storage used)
- Storage Overview (used/available by volume)
- Recent Events (last 10 events)
- Quick Actions (common tasks)

## Resources
- Recharts for charts: https://recharts.org/
- React Loading Skeleton: https://github.com/dvtng/react-loading-skeleton
- Dashboard patterns: https://tailwindui.com/components/application-ui/page-examples/home-screens
- Real-time updates: https://www.patterns.dev/posts/polling

## Validation Gates
```bash
cd apps/stream-manager/web-ui

# Run development server
deno run dev

# Test dashboard features:
# 1. Dashboard loads within 2 seconds
# 2. All widgets display data
# 3. WebSocket events update in real-time
# 4. Charts render correctly
# 5. Quick actions trigger correct navigation
# 6. Responsive layout works on mobile

# Run tests
deno test

# Performance check
deno run build && deno run preview
```

## Success Criteria
- Dashboard loads all widgets without errors
- Real-time updates work via WebSocket
- Charts display correctly with data
- Responsive layout adapts to screen size
- Loading states show during data fetch
- Error states handle API failures gracefully
- Quick actions navigate to correct pages
- Data refreshes automatically every 30 seconds

## Dependencies
- PRP-32 (Base layout) must be completed
- PRP-33 (API client) must be completed
- PRP-34 (WebSocket client) must be completed
- PRP-35 (Authentication) must be completed

## Performance Considerations
- Use React.memo for widget components
- Implement virtual scrolling for events list
- Lazy load chart libraries
- Cache API responses appropriately

## Estimated Effort
4 hours

## Confidence Score
8/10 - Multiple components but straightforward implementation
