# PRP-32: Base Layout and Navigation Components

## Overview
Create the foundational UI layout with navigation, responsive sidebar, and routing infrastructure for the Stream Manager web interface.

## Context
- Frontend setup completed with React/Vite/TypeScript (PRP-30)
- Need responsive layout that works on desktop and mobile
- Must support both light and dark themes
- Navigation should reflect main functionality areas
- Uses Tailwind CSS for styling

## Requirements
1. App shell with header, sidebar, and content area
2. Responsive sidebar that collapses on mobile
3. Navigation menu with icons and labels
4. Theme toggle (light/dark mode)
5. User menu placeholder
6. Breadcrumb navigation
7. React Router setup for client-side routing
8. Loading states and error boundaries

## Implementation Tasks
1. Install React Router v6 and required icon libraries
2. Create Layout component with header/sidebar/content structure
3. Implement collapsible sidebar with hamburger menu
4. Create navigation items configuration
5. Setup React Router with route definitions
6. Implement theme context and toggle
7. Create breadcrumb component
8. Add error boundary components
9. Create loading spinner component
10. Setup layout persistence (sidebar state, theme preference)
11. Add keyboard navigation support
12. Create responsive breakpoint utilities

## Navigation Structure
- Dashboard (home)
- Streams (list and management)
- Recordings (browse and manage)
- Configuration (system settings)
- Metrics (performance monitoring)
- Logs (system logs)
- Help (documentation and support)

## Resources
- React Router v6: https://reactrouter.com/en/main/start/tutorial
- Tailwind UI patterns: https://tailwindui.com/components/application-ui/navigation
- Hero Icons: https://heroicons.com/
- Dark mode with Tailwind: https://tailwindcss.com/docs/dark-mode
- React Error Boundaries: https://react.dev/reference/react/Component#catching-rendering-errors-with-an-error-boundary

## Validation Gates
```bash
cd apps/stream-manager/web-ui

# Install dependencies
deno npm install

# Run development server
deno run dev

# TypeScript check
deno run type-check

# Component tests
deno test

# Build check
deno run build
```

## Success Criteria
- Sidebar toggles on mobile and desktop
- All navigation routes render without errors
- Theme toggle persists across page refresh
- Layout is responsive from 320px to 4K
- Keyboard navigation works (Tab, Enter, Escape)
- Error boundaries catch and display errors gracefully
- Loading states appear during route transitions

## Dependencies
- PRP-30 (Frontend setup) must be completed
- PRP-31 (Static file serving) recommended but not required

## Estimated Effort
4 hours

## Confidence Score
8/10 - Standard patterns but requires careful responsive design
