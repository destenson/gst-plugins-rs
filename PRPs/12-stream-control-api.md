# PRP-12: Stream Control API Endpoints

## Overview
Implement REST API endpoints for stream management operations including add, remove, list, and control recording.

## Context
- Primary interface for stream operations
- Must validate inputs before operations
- Need async handlers for non-blocking
- Should return meaningful status codes

## Requirements
1. Implement stream CRUD endpoints
2. Add recording control endpoints
3. Create stream query endpoints
4. Implement input validation
5. Add operation status responses

## Implementation Tasks
1. Create src/api/streams.rs module
2. Implement POST /api/v1/streams:
   - Accept AddStreamRequest JSON
   - Validate URI format
   - Call manager.add_stream()
   - Return 201 with stream ID
3. Implement DELETE /api/v1/streams/{id}:
   - Extract path parameter
   - Call manager.remove_stream()
   - Return 204 No Content
4. Implement GET /api/v1/streams:
   - Call manager.list_streams()
   - Return stream list JSON
5. Implement GET /api/v1/streams/{id}:
   - Get specific stream details
   - Include health and statistics
   - Return 404 if not found
6. Add recording control:
   - POST /api/v1/streams/{id}/record/start
   - POST /api/v1/streams/{id}/record/stop
   - GET /api/v1/streams/{id}/record/status
7. Add input validation using validator crate

## Validation Gates
```bash
# Test stream endpoints
cargo test --package stream-manager api::streams::tests

# Verify CRUD operations
cargo test api_stream_crud

# Check recording control
cargo test api_recording_control
```

## Dependencies
- PRP-11: REST API foundation
- PRP-09: StreamManager operations

## References
- Actix extractors: https://actix.rs/docs/extractors/
- Validator crate: https://docs.rs/validator/
- RESTful patterns: Standard REST conventions

## Success Metrics
- All CRUD operations work
- Recording control functions
- Proper status codes returned
- Input validation catches bad data

**Confidence Score: 9/10**