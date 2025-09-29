# PRP-33: API Client Service Layer

## Overview
Create a robust TypeScript API client layer for communication with the Stream Manager backend REST API.

## Context
- Backend API uses actix-web on `/api/v1/*` endpoints
- API documentation available in docs/API.md
- Need type-safe client with error handling
- Must handle authentication tokens
- Should support request cancellation

## Requirements
1. TypeScript interfaces for all API types
2. Axios-based HTTP client with interceptors
3. Authentication token management
4. Request/response interceptors for errors
5. Automatic retry logic with exponential backoff
6. Request cancellation support
7. Type-safe API methods for all endpoints
8. Response caching where appropriate

## Implementation Tasks
1. Install axios and axios-retry
2. Create TypeScript types from API documentation
3. Setup base API client with axios instance
4. Implement authentication interceptor
5. Create error handling interceptor
6. Add retry logic configuration
7. Implement API service classes by domain:
   - StreamsAPI (stream management)
   - RecordingAPI (recording control)
   - ConfigAPI (configuration)
   - MetricsAPI (statistics)
   - BackupAPI (backup/restore)
8. Add request cancellation tokens
9. Implement response caching for GET requests
10. Create mock API client for testing
11. Add API client to React context
12. Create custom hooks for API calls

## API Endpoints to Implement
- Health: GET /health, GET /api/v1/status
- Streams: GET/POST/PUT/DELETE /api/v1/streams
- Recording: POST /api/v1/streams/{id}/recording/*
- Metrics: GET /api/v1/metrics, GET /api/v1/streams/{id}/metrics
- Config: GET/PUT /api/v1/config, POST /api/v1/config/reload
- Backup: POST /api/v1/backup, GET /api/v1/backup/{id}, POST /api/v1/restore

## Resources
- Axios documentation: https://axios-http.com/docs/intro
- axios-retry: https://github.com/softonic/axios-retry
- TypeScript with Axios: https://blog.logrocket.com/how-to-use-axios-typescript/
- React Query (optional): https://tanstack.com/query/latest
- API documentation: apps/stream-manager/docs/API.md

## Validation Gates
```bash
cd apps/stream-manager/web-ui

# TypeScript compilation
deno run type-check

# Run tests
deno test

# Test API calls (with backend running)
deno run dev
# Open browser console and test: 
# window.api.streams.list()
# window.api.health.check()
```

## Success Criteria
- All API endpoints have TypeScript types
- API calls work with authentication
- Errors are properly typed and handled
- Retry logic works for failed requests
- Request cancellation prevents memory leaks
- API client can be used in React components
- Mock client works for testing

## Dependencies
- PRP-30 (Frontend setup) must be completed
- Backend API server should be running for integration testing

## Estimated Effort
3 hours

## Confidence Score
9/10 - Standard API client pattern with good documentation available
