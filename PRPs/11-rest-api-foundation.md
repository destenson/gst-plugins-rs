# PRP-11: REST API Foundation

## Overview
Implement the REST API server using Actix-Web for stream control, monitoring, and configuration management.

## Context
- API is primary control interface
- Must be thread-safe with StreamManager
- Need proper error responses
- Should support JSON requests/responses

## Requirements
1. Setup Actix-Web server
2. Define API routes structure
3. Implement request/response DTOs
4. Add error handling middleware
5. Create shared app state

## Implementation Tasks
1. Create src/api/mod.rs module
2. Define app state struct:
   - Arc<StreamManager> reference
   - Configuration reference
   - Metrics collector
3. Setup Actix-Web server:
   - Configure bind address from config
   - Setup worker threads
   - Add graceful shutdown
4. Define route structure:
   - /api/v1/streams - stream operations
   - /api/v1/health - health endpoints
   - /api/v1/config - configuration
   - /api/v1/metrics - metrics
5. Create request/response DTOs:
   - AddStreamRequest
   - StreamResponse
   - HealthResponse
   - ErrorResponse
6. Implement error handling:
   - Custom error type
   - Error middleware
   - Proper HTTP status codes
7. Add request logging middleware

## Validation Gates
```bash
# Test API server startup
cargo test --package stream-manager api::tests

# Verify route registration
cargo test api_routes

# Check error handling
cargo test api_error_responses
```

## Dependencies
- PRP-09: StreamManager for app state

## References
- Actix-Web docs: https://actix.rs/docs/
- Error handling: https://actix.rs/docs/errors/
- Middleware: https://actix.rs/docs/middleware/

## Success Metrics
- API server starts on configured port
- Routes properly registered
- JSON serialization works
- Errors return proper status codes

**Confidence Score: 9/10**