-- Initial schema for stream-manager database

CREATE TABLE IF NOT EXISTS streams (
    id TEXT PRIMARY KEY,
    uri TEXT NOT NULL,
    config TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_streams_status ON streams(status);
CREATE INDEX IF NOT EXISTS idx_streams_created_at ON streams(created_at);

CREATE TABLE IF NOT EXISTS recordings (
    id TEXT PRIMARY KEY,
    stream_id TEXT NOT NULL,
    path TEXT NOT NULL,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    size_bytes INTEGER,
    duration_ms INTEGER,
    status TEXT NOT NULL,
    metadata TEXT,
    FOREIGN KEY (stream_id) REFERENCES streams(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_recordings_path ON recordings(path);
CREATE INDEX IF NOT EXISTS idx_recordings_stream_id ON recordings(stream_id);
CREATE INDEX IF NOT EXISTS idx_recordings_start_time ON recordings(start_time);
CREATE INDEX IF NOT EXISTS idx_recordings_end_time ON recordings(end_time);
CREATE INDEX IF NOT EXISTS idx_recordings_status ON recordings(status);
CREATE INDEX IF NOT EXISTS idx_recordings_stream_start ON recordings(stream_id, start_time);
CREATE INDEX IF NOT EXISTS idx_recordings_stream_end ON recordings(stream_id, end_time);

CREATE TABLE IF NOT EXISTS state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
