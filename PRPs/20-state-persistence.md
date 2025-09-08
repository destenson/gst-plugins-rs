# PRP-20: State Persistence and Database Integration

## Overview
Implement state persistence using SQLite to maintain stream configuration, recording metadata, and system state across restarts.

## Context
- Need to restore state after restart
- Must track recording segments
- Should store stream configurations
- Need query capabilities for history

## Requirements
1. Setup SQLite database
2. Define schema for persistence
3. Implement state save/restore
4. Add recording metadata tracking
5. Create query interfaces

## Implementation Tasks
1. Create src/database/mod.rs module
2. Setup SQLite with sqlx:
   - Connection pool
   - Migration system
   - Async queries
3. Define database schema:
   - streams table (id, uri, config)
   - recordings table (stream_id, path, start, end)
   - events table (timestamp, type, data)
   - metrics table (time-series data)
4. Implement state operations:
   - Save stream configuration
   - Update stream status
   - Record health events
   - Store metrics samples
5. Add recording tracking:
   - Insert on segment start
   - Update on segment complete
   - Track file sizes
   - Store metadata
6. Create restoration logic:
   - Load streams on startup
   - Restore recording state
   - Resume from last position
7. Add cleanup procedures:
   - Purge old events
   - Archive completed recordings
   - Vacuum database

## Validation Gates
```bash
# Test database operations
cargo test --package stream-manager database::tests

# Verify migrations
sqlx migrate run --database-url sqlite://test.db

# Check state restoration
cargo test state_restoration
```

## Dependencies
- PRP-09: StreamManager state to persist
- PRP-07: Recording metadata to store

## References
- SQLx: https://github.com/launchbadge/sqlx
- Migration patterns: SQLx migration documentation
- Schema design: Standard relational patterns

## Success Metrics
- State persisted to database
- Restoration works after restart
- Query performance acceptable
- Database size manageable

**Confidence Score: 8/10**