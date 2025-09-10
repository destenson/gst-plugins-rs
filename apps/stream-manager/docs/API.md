# Stream Manager API Documentation

## Overview

The Stream Manager provides a RESTful API for managing streams, recordings, and system configuration. All API endpoints are prefixed with `/api`.

## Authentication

Currently, the API uses token-based authentication. Include the token in the `Authorization` header:

```
Authorization: Bearer <your-token>
```

## Base URL

```
http://localhost:3000
```

## API Endpoints

### Health & Status

#### GET /health
Health check endpoint for monitoring.

**Response:**
```json
{
  "status": "healthy",
  "uptime": 3600,
  "version": "0.1.0"
}
```

#### GET /api/status
Get overall system status.

**Response:**
```json
{
  "active_streams": 5,
  "total_streams": 10,
  "recording_streams": 3,
  "cpu_usage": 45.2,
  "memory_usage": 2048,
  "disk_usage": {
    "used": 104857600,
    "total": 1073741824
  }
}
```

### Stream Management

#### GET /api/streams
List all configured streams.

**Query Parameters:**
- `status` (optional): Filter by status (active, inactive, error)
- `recording` (optional): Filter by recording status (true/false)

**Response:**
```json
{
  "streams": [
    {
      "id": "camera-1",
      "source_url": "rtsp://camera.local:554/stream",
      "status": "active",
      "recording": {
        "enabled": true,
        "status": "recording",
        "current_file": "/recordings/camera-1/2024-01-15_10-30-00.mp4",
        "duration": 3600
      },
      "metrics": {
        "bitrate": 4096000,
        "framerate": 30,
        "resolution": "1920x1080",
        "packets_received": 1000000,
        "packets_lost": 10
      }
    }
  ]
}
```

#### GET /api/streams/{id}
Get details for a specific stream.

**Response:**
```json
{
  "id": "camera-1",
  "source_url": "rtsp://camera.local:554/stream",
  "status": "active",
  "created_at": "2024-01-15T10:00:00Z",
  "last_connected": "2024-01-15T10:30:00Z",
  "recording": {
    "enabled": true,
    "status": "recording",
    "current_file": "/recordings/camera-1/2024-01-15_10-30-00.mp4",
    "duration": 3600,
    "total_size": 524288000,
    "segments": [
      {
        "filename": "2024-01-15_10-00-00.mp4",
        "size": 104857600,
        "duration": 600
      }
    ]
  },
  "pipeline": {
    "state": "playing",
    "latency": 150,
    "buffer_level": 80
  },
  "errors": []
}
```

#### POST /api/streams
Create a new stream.

**Request Body:**
```json
{
  "id": "camera-2",
  "source_url": "rtsp://camera2.local:554/stream",
  "recording": {
    "enabled": true,
    "segment_duration": 600,
    "retention_days": 7
  },
  "inference": {
    "enabled": true,
    "model": "yolov5",
    "threshold": 0.5
  },
  "reconnect": {
    "enabled": true,
    "max_attempts": 10,
    "backoff_ms": 1000
  }
}
```

**Response:** 201 Created
```json
{
  "id": "camera-2",
  "status": "initializing"
}
```

#### PUT /api/streams/{id}
Update stream configuration.

**Request Body:**
```json
{
  "recording": {
    "enabled": false
  },
  "inference": {
    "threshold": 0.7
  }
}
```

**Response:** 200 OK

#### DELETE /api/streams/{id}
Remove a stream.

**Response:** 204 No Content

#### POST /api/streams/{id}/start
Start a stream.

**Response:** 200 OK
```json
{
  "status": "starting"
}
```

#### POST /api/streams/{id}/stop
Stop a stream.

**Response:** 200 OK
```json
{
  "status": "stopping"
}
```

#### POST /api/streams/{id}/restart
Restart a stream.

**Response:** 200 OK
```json
{
  "status": "restarting"
}
```

### Recording Control

#### POST /api/streams/{id}/recording/start
Start recording for a stream.

**Request Body (optional):**
```json
{
  "filename": "custom-recording.mp4",
  "segment_duration": 300
}
```

**Response:** 200 OK
```json
{
  "status": "recording",
  "filename": "/recordings/camera-1/2024-01-15_11-00-00.mp4"
}
```

#### POST /api/streams/{id}/recording/stop
Stop recording for a stream.

**Response:** 200 OK
```json
{
  "status": "stopped",
  "files": [
    "/recordings/camera-1/2024-01-15_10-30-00.mp4",
    "/recordings/camera-1/2024-01-15_10-40-00.mp4"
  ],
  "total_duration": 1200,
  "total_size": 209715200
}
```

#### POST /api/streams/{id}/recording/pause
Pause recording (keeps stream active).

**Response:** 200 OK

#### POST /api/streams/{id}/recording/resume
Resume paused recording.

**Response:** 200 OK

#### GET /api/streams/{id}/recordings
List all recordings for a stream.

**Query Parameters:**
- `start_date` (optional): ISO 8601 date
- `end_date` (optional): ISO 8601 date
- `limit` (optional): Maximum number of results

**Response:**
```json
{
  "recordings": [
    {
      "filename": "2024-01-15_10-00-00.mp4",
      "path": "/recordings/camera-1/2024-01-15_10-00-00.mp4",
      "size": 104857600,
      "duration": 600,
      "created_at": "2024-01-15T10:00:00Z"
    }
  ],
  "total_size": 524288000,
  "total_duration": 3000
}
```

### Metrics & Statistics

#### GET /api/metrics
Get Prometheus-compatible metrics.

**Response:** (text/plain)
```
# HELP stream_manager_active_streams Number of active streams
# TYPE stream_manager_active_streams gauge
stream_manager_active_streams 5

# HELP stream_manager_bytes_received Total bytes received
# TYPE stream_manager_bytes_received counter
stream_manager_bytes_received{stream="camera-1"} 1073741824

# HELP stream_manager_packets_lost Total packets lost
# TYPE stream_manager_packets_lost counter
stream_manager_packets_lost{stream="camera-1"} 10
```

#### GET /api/streams/{id}/metrics
Get detailed metrics for a specific stream.

**Response:**
```json
{
  "bitrate": {
    "current": 4096000,
    "average": 4000000,
    "peak": 5000000
  },
  "framerate": {
    "current": 30,
    "average": 29.97,
    "dropped": 5
  },
  "latency": {
    "pipeline": 150,
    "network": 50,
    "processing": 100
  },
  "packets": {
    "received": 1000000,
    "lost": 10,
    "recovered": 8
  },
  "errors": {
    "decode": 0,
    "network": 2,
    "total": 2
  }
}
```

### Configuration

#### GET /api/config
Get current configuration.

**Response:**
```json
{
  "server": {
    "port": 3000,
    "host": "0.0.0.0"
  },
  "recording": {
    "base_path": "/recordings",
    "segment_duration": 600,
    "retention_days": 7
  },
  "inference": {
    "enabled": true,
    "device": "gpu",
    "models_path": "/models"
  }
}
```

#### PUT /api/config
Update configuration (requires reload).

**Request Body:**
```json
{
  "recording": {
    "retention_days": 14
  }
}
```

**Response:** 200 OK

#### POST /api/config/reload
Reload configuration from file.

**Response:** 200 OK

### Backup & Recovery

#### POST /api/backup
Create a backup of configuration and state.

**Request Body:**
```json
{
  "include_recordings": false,
  "destination": "/backups/stream-manager-backup.tar.gz"
}
```

**Response:** 202 Accepted
```json
{
  "job_id": "backup-123",
  "status": "running"
}
```

#### GET /api/backup/{job_id}
Get backup job status.

**Response:**
```json
{
  "job_id": "backup-123",
  "status": "completed",
  "progress": 100,
  "file": "/backups/stream-manager-backup-20240115.tar.gz",
  "size": 10485760
}
```

#### POST /api/restore
Restore from backup.

**Request Body:**
```json
{
  "backup_file": "/backups/stream-manager-backup-20240115.tar.gz",
  "restore_config": true,
  "restore_state": true
}
```

**Response:** 202 Accepted

### WebSocket Events

#### WS /api/events
WebSocket endpoint for real-time events.

**Connection:**
```javascript
const ws = new WebSocket('ws://localhost:3000/api/events');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Event:', data);
};
```

**Event Types:**

Stream Events:
```json
{
  "type": "stream.started",
  "stream_id": "camera-1",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

Recording Events:
```json
{
  "type": "recording.started",
  "stream_id": "camera-1",
  "filename": "/recordings/camera-1/2024-01-15_10-30-00.mp4",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

Error Events:
```json
{
  "type": "stream.error",
  "stream_id": "camera-1",
  "error": "Connection timeout",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

Metric Events:
```json
{
  "type": "metrics.update",
  "stream_id": "camera-1",
  "metrics": {
    "bitrate": 4096000,
    "framerate": 30,
    "packets_lost": 5
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

## Error Responses

All error responses follow this format:

```json
{
  "error": {
    "code": "STREAM_NOT_FOUND",
    "message": "Stream with ID 'camera-99' not found",
    "details": {
      "stream_id": "camera-99"
    }
  }
}
```

### Common Error Codes

| Code | HTTP Status | Description |
|------|------------|-------------|
| `STREAM_NOT_FOUND` | 404 | Stream ID does not exist |
| `STREAM_ALREADY_EXISTS` | 409 | Stream ID already in use |
| `INVALID_CONFIGURATION` | 400 | Invalid configuration parameters |
| `STREAM_BUSY` | 409 | Stream is busy with another operation |
| `RECORDING_IN_PROGRESS` | 409 | Recording already active |
| `NO_RECORDING` | 404 | No active recording to stop |
| `UNAUTHORIZED` | 401 | Missing or invalid authentication |
| `INTERNAL_ERROR` | 500 | Internal server error |

## Rate Limiting

API requests are rate-limited to prevent abuse:
- 100 requests per minute for read operations
- 20 requests per minute for write operations
- WebSocket connections limited to 10 per IP

## Examples

### Python Client

```python
import requests
import json

BASE_URL = "http://localhost:3000/api"
TOKEN = "your-auth-token"

headers = {
    "Authorization": f"Bearer {TOKEN}",
    "Content-Type": "application/json"
}

# Create a stream
stream_config = {
    "id": "front-door",
    "source_url": "rtsp://192.168.1.100:554/stream",
    "recording": {"enabled": True}
}

response = requests.post(
    f"{BASE_URL}/streams",
    json=stream_config,
    headers=headers
)

# Start recording
response = requests.post(
    f"{BASE_URL}/streams/front-door/recording/start",
    headers=headers
)

# Get stream metrics
response = requests.get(
    f"{BASE_URL}/streams/front-door/metrics",
    headers=headers
)
metrics = response.json()
print(f"Current bitrate: {metrics['bitrate']['current']}")
```

### cURL Examples

```bash
# List all streams
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:3000/api/streams

# Create a new stream
curl -X POST \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"id":"camera-3","source_url":"rtsp://camera3.local:554/stream"}' \
     http://localhost:3000/api/streams

# Start recording with custom settings
curl -X POST \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"segment_duration":300}' \
     http://localhost:3000/api/streams/camera-3/recording/start

# Get system metrics in Prometheus format
curl http://localhost:3000/api/metrics
```

### JavaScript/Node.js

```javascript
const axios = require('axios');

const api = axios.create({
  baseURL: 'http://localhost:3000/api',
  headers: {
    'Authorization': 'Bearer your-token',
    'Content-Type': 'application/json'
  }
});

// Async function to manage streams
async function manageStream() {
  try {
    // Create stream
    const createResponse = await api.post('/streams', {
      id: 'office-cam',
      source_url: 'rtsp://office.local:554/stream',
      recording: { enabled: true }
    });
    
    console.log('Stream created:', createResponse.data);
    
    // Get stream status
    const statusResponse = await api.get('/streams/office-cam');
    console.log('Stream status:', statusResponse.data.status);
    
    // Start recording
    await api.post('/streams/office-cam/recording/start');
    console.log('Recording started');
    
  } catch (error) {
    console.error('Error:', error.response?.data || error.message);
  }
}

// WebSocket event listener
const WebSocket = require('ws');
const ws = new WebSocket('ws://localhost:3000/api/events');

ws.on('message', (data) => {
  const event = JSON.parse(data);
  console.log('Event received:', event);
  
  if (event.type === 'stream.error') {
    console.error(`Stream ${event.stream_id} error: ${event.error}`);
  }
});
```

## SDK Support

Official SDKs are planned for:
- Python
- JavaScript/TypeScript
- Go
- Rust

Check the [GitHub repository](https://github.com/your-org/stream-manager) for SDK availability.