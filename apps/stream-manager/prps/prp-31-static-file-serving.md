# PRP-31: Static File Serving Integration

## Overview
Configure the actix-web backend to serve the frontend static files in production while maintaining API routes.

## Context
- Backend uses actix-web 4.11.0
- API routes are mounted at `/api/v1/*`
- WebSocket endpoint at `/api/v1/events`
- Frontend build outputs to `static/` directory
- Must support SPA routing (return index.html for unknown routes)

## Requirements
1. Add static file serving to actix-web application
2. Serve files from `static/` directory in production
3. Implement SPA fallback routing
4. Maintain existing API routes
5. Add proper cache headers for static assets
6. Support gzip compression for text assets
7. CORS configuration for development mode

## Implementation Tasks
1. Add actix-files dependency to Cargo.toml
2. Create static file service in api/mod.rs
3. Configure routing priority (API first, then static files)
4. Implement SPA fallback handler for client-side routing
5. Add cache control headers (1 year for hashed assets, no-cache for index.html)
6. Configure gzip middleware for responses
7. Add development mode flag to disable caching
8. Update main.rs to mount static file service
9. Create config option for static file directory path
10. Add tests for static file serving

## Resources
- actix-files documentation: https://docs.rs/actix-files/latest/actix_files/
- SPA routing with actix: https://github.com/actix/examples/tree/master/basics/static-files
- Cache control best practices: https://web.dev/http-cache/

## Validation Gates
```bash
# Build frontend first
cd apps/stream-manager/web-ui && deno run build && cd ..

# Run the Rust server
cargo run --package stream-manager

# Test static file serving
curl http://localhost:8080/index.html

# Test SPA routing (should return index.html)
curl http://localhost:8080/dashboard

# Test API still works
curl http://localhost:8080/api/v1/health

# Check cache headers
curl -I http://localhost:8080/assets/main.js
```

## Success Criteria
- Frontend files served at root path
- API endpoints still accessible at /api/v1/*
- SPA routing works (unknown paths return index.html)
- Proper cache headers set
- Gzip compression enabled for text files
- No conflicts with existing routes

## Dependencies
- PRP-30 must be completed (frontend build system)

## Estimated Effort
2 hours

## Confidence Score
9/10 - Well-documented actix-files integration
