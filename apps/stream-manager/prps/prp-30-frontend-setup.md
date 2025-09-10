# PRP-30: Frontend Project Setup and Build System

## Overview
Initialize a modern web frontend project for the Stream Manager with build tooling, development server, and integration with the existing Rust backend API.

## Context
- Backend uses actix-web on port 3000 with REST API at `/api/*`
- WebSocket events available at `/api/events`
- No existing frontend infrastructure
- Must be served alongside the Rust backend
- Cross-platform compatibility required (Windows, Linux, macOS)

## Requirements
1. Create frontend project structure at `apps/stream-manager/web-ui/`
2. Setup modern build tooling (Vite recommended for fast HMR)
3. Configure proxy for API calls during development
4. Setup production build that outputs to `apps/stream-manager/static/`
5. TypeScript for type safety
6. CSS framework for rapid development (Tailwind CSS recommended)
7. Testing infrastructure (Vitest for unit tests)

## Implementation Tasks
1. Initialize project with Vite and React/Vue/Svelte (React recommended for ecosystem)
2. Configure TypeScript with strict settings
3. Setup Tailwind CSS with custom configuration
4. Configure Vite proxy to forward `/api` to `http://localhost:3000`
5. Create build scripts that output to `../static/` directory
6. Setup ESLint and Prettier for code quality
7. Create basic index.html with proper meta tags
8. Add development scripts to package.json
9. Create .gitignore for node_modules and build artifacts
10. Setup basic CI/CD scripts for frontend builds

## Resources
- Vite documentation: https://vitejs.dev/guide/
- Vite proxy config: https://vitejs.dev/config/server-options.html#server-proxy
- Tailwind CSS setup: https://tailwindcss.com/docs/guides/vite
- TypeScript with React: https://react-typescript-cheatsheet.netlify.app/

## Validation Gates
```bash
# Navigate to frontend directory
cd apps/stream-manager/web-ui

# Check installation
deno npm install

# Run development server
deno run dev

# Build for production
deno run build

# Run tests
deno test

# Lint check
deno lint
```

## Success Criteria
- Development server runs on http://localhost:5173
- API calls proxy correctly to backend
- Production build outputs minified assets to static/
- Hot module replacement works during development
- TypeScript compilation has no errors
- Basic "Hello World" page renders

## Dependencies
- No dependencies on other PRPs
- Backend API server must be running for full functionality

## Estimated Effort
3 hours

## Confidence Score
9/10 - Straightforward frontend setup with well-documented tools
