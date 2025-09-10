# PRP-37: Stream List and Management Page

## Overview
Create a comprehensive stream management interface with list view, filtering, sorting, and bulk actions.

## Context
- Streams are the core entities in the system
- Need to handle 100+ streams efficiently
- Real-time status updates via WebSocket
- Must support CRUD operations
- Mobile-responsive table/card view

## Requirements
1. Stream list with pagination
2. Search and filter capabilities
3. Sort by multiple columns
4. Bulk actions (start/stop/delete)
5. Status indicators with colors
6. Quick actions per stream
7. Add new stream modal
8. View switcher (table/card)
9. Real-time status updates

## Implementation Tasks
1. Create StreamList page component
2. Implement data table with pagination
3. Add search bar with debouncing
4. Create filter dropdowns (status, recording, health)
5. Implement column sorting
6. Add row selection for bulk actions
7. Create bulk action toolbar
8. Implement AddStreamModal with form
9. Add stream status badges with colors
10. Create quick action menus (dropdown)
11. Implement card view for mobile
12. Add real-time updates via WebSocket
13. Create stream preview on hover
14. Add export to CSV functionality

## Table Columns
- Selection checkbox
- Stream ID/Name
- Source URL
- Status (Active/Inactive/Error)
- Health indicator
- Recording status
- Bitrate/FPS
- Uptime
- Actions menu

## Resources
- TanStack Table: https://tanstack.com/table/v8
- React Select: https://react-select.com/
- Debouncing: https://www.freecodecamp.org/news/javascript-debounce-example/
- Bulk actions pattern: https://www.nngroup.com/articles/ui-copy-tables/

## Validation Gates
```bash
cd apps/stream-manager/web-ui

# Run development server
deno run dev

# Test stream list features:
# 1. List loads and displays streams
# 2. Pagination works correctly
# 3. Search filters results in real-time
# 4. Sorting changes order correctly
# 5. Bulk actions affect selected streams
# 6. Add stream modal creates new stream
# 7. Real-time updates show status changes
# 8. Mobile view shows cards instead of table

# Run tests
deno test

# Type checking
deno run type-check
```

## Success Criteria
- Stream list displays all streams with pagination
- Search and filters work correctly
- Bulk actions execute on selected streams
- Add stream modal validates and submits
- Real-time status updates appear immediately
- Table is responsive and switches to cards on mobile
- Performance remains good with 100+ streams
- Export generates valid CSV file

## Dependencies
- PRP-32 (Base layout) must be completed
- PRP-33 (API client) must be completed
- PRP-34 (WebSocket client) must be completed

## Performance Considerations
- Virtualize table rows for large lists
- Debounce search input (300ms)
- Use React.memo for row components
- Implement pagination (20-50 items per page)

## Estimated Effort
4 hours

## Confidence Score
7/10 - Complex table with many features requires careful implementation
