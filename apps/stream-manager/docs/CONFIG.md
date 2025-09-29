# Stream Manager Configuration Guide

## Overview

Stream Manager can be configured through multiple methods with the following precedence (highest to lowest):
1. Runtime configuration (via API calls or hot-reloaded config file changes)
2. Command-line arguments
3. Environment variables
4. Configuration file (TOML)
5. Default values

## Configuration File

The default configuration file is `config.toml` in the working directory. You can specify a different file using:

```bash
stream-manager --config /path/to/custom-config.toml
```

## Complete Configuration Reference

### Example Configuration File

```toml
# Server Configuration
[server]
host = "0.0.0.0"              # Bind address
port = 3000                   # API server port
workers = 4                   # Number of worker threads
max_connections = 1000        # Maximum concurrent connections

# Authentication
[auth]
enabled = true                # Enable authentication
token = "secret-token"        # Bearer token (use env var in production)
jwt_secret = ""              # JWT secret for token generation

# Logging
[logging]
level = "info"               # Log level: trace, debug, info, warn, error
format = "json"              # Log format: json, pretty, compact
file = "/var/log/stream-manager.log"  # Log file path (optional)
rotate_size = "100MB"        # Rotate when file reaches this size
rotate_keep = 10            # Number of rotated files to keep

# Stream Configuration
[streams]
max_streams = 100           # Maximum number of concurrent streams
default_timeout = 10000     # Default stream timeout in ms
buffer_size = 4096          # Buffer size in KB
enable_statistics = true   # Collect stream statistics

# Pipeline Configuration
[pipeline]
use_hardware_acceleration = true  # Use GPU if available
decode_threads = 4              # Number of decode threads
output_format = "H264"          # Default output codec
enable_deinterlacing = false   # Enable deinterlacing filter

# Recording Configuration
[recording]
enabled = true                         # Enable recording globally
base_path = "/var/recordings"         # Base directory for recordings
segment_duration = 600                 # Segment duration in seconds (10 min)
segment_format = "%Y-%m-%d_%H-%M-%S"  # Segment filename format
container = "mp4"                      # Container format: mp4, mkv, ts
retention_days = 7                     # Days to keep recordings
cleanup_interval = 3600                # Cleanup check interval in seconds
min_free_space = "10GB"               # Minimum free disk space
compression = "none"                   # Compression: none, gzip, lz4

# Storage Management
[storage]
# Primary storage
[[storage.volumes]]
path = "/var/recordings"
max_size = "1TB"
priority = 1
type = "local"

# Secondary storage (overflow)
[[storage.volumes]]
path = "/mnt/backup/recordings"
max_size = "10TB"
priority = 2
type = "nfs"

# Archive storage
[[storage.volumes]]
path = "s3://my-bucket/recordings"
max_size = "unlimited"
priority = 3
type = "s3"
access_key = "${S3_ACCESS_KEY}"
secret_key = "${S3_SECRET_KEY}"
region = "us-east-1"

# Inference Configuration
[inference]
enabled = false                    # Enable inference globally
device = "auto"                    # Device: auto, gpu, cpu
gpu_id = 0                        # GPU device ID
models_path = "/var/models"       # Path to model files
default_model = "yolov5"          # Default model to use
batch_size = 1                    # Inference batch size
num_threads = 4                   # CPU inference threads
enable_tensorrt = true            # Use TensorRT optimization

# Inference Models
[[inference.models]]
name = "yolov5"
path = "/var/models/yolov5.onnx"
type = "object_detection"
input_size = [640, 640]
confidence_threshold = 0.5
nms_threshold = 0.4
classes = ["person", "car", "truck", "bicycle"]

[[inference.models]]
name = "face_detection"
path = "/var/models/face_detection.trt"
type = "face_detection"
input_size = [300, 300]
confidence_threshold = 0.7

# Reconnection Strategy
[reconnect]
enabled = true                  # Enable auto-reconnection
max_attempts = 10              # Maximum reconnection attempts
initial_backoff = 1000         # Initial backoff in ms
max_backoff = 30000           # Maximum backoff in ms
backoff_multiplier = 2.0      # Backoff multiplier
jitter = 0.1                  # Jitter factor (0-1)

# Health Monitoring
[health]
enabled = true                    # Enable health monitoring
check_interval = 30              # Health check interval in seconds
unhealthy_threshold = 3          # Failures before marking unhealthy
recovery_threshold = 2           # Successes before marking healthy
timeout = 5000                   # Health check timeout in ms

# Metrics Collection
[metrics]
enabled = true                      # Enable metrics collection
prometheus_port = 9090             # Prometheus metrics port
collection_interval = 10           # Metrics collection interval in seconds
detailed_metrics = true            # Collect detailed per-stream metrics
export_format = "prometheus"       # Format: prometheus, json
retention = 3600                   # Metrics retention in seconds

# WebRTC Configuration
[webrtc]
enabled = true                    # Enable WebRTC support
stun_server = "stun:stun.l.google.com:19302"
turn_server = ""                 # TURN server URL
turn_username = ""               # TURN username
turn_password = ""               # TURN password
ice_servers = []                 # Additional ICE servers
enable_trickle = true           # Enable trickle ICE
port_range = [10000, 10100]     # UDP port range

# WHIP/WHEP Configuration
[whip]
enabled = true                  # Enable WHIP/WHEP protocols
endpoint = "/whip"             # WHIP endpoint path
auth_required = true           # Require authentication
max_sessions = 50              # Maximum concurrent sessions

# RTSP Server Configuration
[rtsp]
enabled = true                    # Enable RTSP server
port = 8554                      # RTSP server port
enable_auth = false              # Enable RTSP authentication
username = ""                    # RTSP username
password = ""                    # RTSP password
path_prefix = "/live"           # Path prefix for streams
enable_rtcp = true              # Enable RTCP
enable_rtp_info = true          # Include RTP info in response

# Backup Configuration
[backup]
enabled = true                        # Enable automatic backups
schedule = "0 2 * * *"               # Cron schedule (2 AM daily)
retention_count = 7                  # Number of backups to keep
backup_path = "/var/backups"        # Backup storage path
include_recordings = false           # Include recordings in backup
compress = true                      # Compress backup files
encryption_key = ""                  # Encryption key (optional)

# Database Configuration (for state persistence)
[database]
type = "sqlite"                     # Database type: sqlite, postgres
path = "/var/lib/stream-manager/state.db"  # SQLite path
connection_string = ""              # PostgreSQL connection string
max_connections = 10                # Maximum DB connections
enable_wal = true                   # Enable Write-Ahead Logging

# Telemetry Configuration
[telemetry]
enabled = false                    # Enable OpenTelemetry
endpoint = "http://localhost:4317"  # OTLP endpoint
service_name = "stream-manager"    # Service name
trace_ratio = 0.1                 # Sampling ratio (0-1)
export_interval = 10               # Export interval in seconds
resource_attributes = { environment = "production", region = "us-east-1" }

# System Service Configuration
[service]
user = "stream-manager"          # Run as user
group = "stream-manager"         # Run as group
working_directory = "/var/lib/stream-manager"
pid_file = "/var/run/stream-manager.pid"
enable_watchdog = true           # Enable systemd watchdog
watchdog_interval = 30           # Watchdog ping interval
restart_on_failure = true        # Auto-restart on failure
restart_delay = 5                # Restart delay in seconds

# Performance Tuning
[performance]
enable_cpu_affinity = false      # Pin threads to CPU cores
cpu_cores = []                   # CPU cores to use (empty = all)
io_threads = 4                   # I/O thread pool size
enable_huge_pages = false        # Use huge pages
memory_limit = "4GB"             # Memory usage limit
enable_jemalloc = true           # Use jemalloc allocator

# Network Configuration
[network]
enable_ipv6 = true              # Enable IPv6 support
tcp_nodelay = true              # Disable Nagle's algorithm
tcp_keepalive = 60              # TCP keepalive in seconds
receive_buffer = "4MB"          # Socket receive buffer
send_buffer = "4MB"             # Socket send buffer
max_udp_payload = 1400          # Maximum UDP payload size
```

## Environment Variables

All configuration options can be set via environment variables using the prefix `STREAM_MANAGER_` and replacing dots with underscores:

```bash
# Server configuration
export STREAM_MANAGER_SERVER_HOST="0.0.0.0"
export STREAM_MANAGER_SERVER_PORT="3000"

# Authentication
export STREAM_MANAGER_AUTH_TOKEN="your-secret-token"

# Recording
export STREAM_MANAGER_RECORDING_BASE_PATH="/recordings"
export STREAM_MANAGER_RECORDING_RETENTION_DAYS="14"

# Inference
export STREAM_MANAGER_INFERENCE_DEVICE="gpu"
export STREAM_MANAGER_INFERENCE_GPU_ID="0"

# Database
export STREAM_MANAGER_DATABASE_CONNECTION_STRING="postgresql://user:pass@localhost/stream_manager"

# Telemetry
export STREAM_MANAGER_TELEMETRY_ENDPOINT="http://otel-collector:4317"
```

## Command-Line Arguments

```bash
stream-manager [OPTIONS]

OPTIONS:
    -c, --config <FILE>           Configuration file path
    -h, --host <HOST>            Server bind address
    -p, --port <PORT>            Server port
    -l, --log-level <LEVEL>      Log level (trace|debug|info|warn|error)
    -d, --daemon                 Run as daemon
    -v, --version                Print version
    --help                       Print help information
```

## Stream-Specific Configuration

Streams can have individual configuration that overrides defaults:

```toml
[[streams]]
id = "camera-1"
source_url = "rtsp://192.168.1.100:554/stream"
username = "admin"
password = "password"
reconnect_attempts = 20
timeout = 15000

[streams.recording]
enabled = true
segment_duration = 300
retention_days = 14
path = "/recordings/important/camera-1"

[streams.inference]
enabled = true
model = "yolov5"
confidence_threshold = 0.6
regions_of_interest = [
  { x = 0, y = 0, width = 640, height = 480 }
]

[[streams]]
id = "camera-2"
source_url = "rtsp://192.168.1.101:554/stream"
recording = { enabled = false }
inference = { enabled = false }
```

## Configuration Validation

Stream Manager validates configuration on startup and will fail with clear error messages:

```bash
Error: Invalid configuration
  - recording.segment_duration: must be between 60 and 3600 seconds
  - inference.device: must be one of: auto, gpu, cpu
  - server.port: must be between 1 and 65535
```

## Hot Reload

Configuration can be reloaded without restarting the service:

```bash
# Via API
curl -X POST http://localhost:3000/api/v1/config/reload

# Via signal
kill -SIGHUP $(cat /var/run/stream-manager.pid)
```

Note: Not all settings can be changed at runtime. The following require a restart:
- Server host/port
- Database configuration
- Performance tuning options
- System service settings

## Best Practices

### Security

1. **Never hardcode sensitive values** in configuration files:
```toml
# Bad
auth.token = "my-secret-token"

# Good
auth.token = "${AUTH_TOKEN}"
```

2. **Use proper file permissions**:
```bash
chmod 600 /etc/stream-manager/config.toml
chown stream-manager:stream-manager /etc/stream-manager/config.toml
```

3. **Rotate tokens regularly** and use strong, random values.

### Performance

1. **Tune based on workload**:
   - High stream count: Increase `workers` and `max_connections`
   - High-quality streams: Increase `buffer_size`
   - Many recordings: Optimize `segment_duration`

2. **Storage hierarchy**:
   - Use fast SSD for active recordings
   - Use HDD for short-term storage
   - Use object storage for long-term archive

3. **Resource limits**:
   - Set appropriate `memory_limit`
   - Configure `max_streams` based on available resources
   - Use `cpu_cores` to isolate workload

### Reliability

1. **Configure proper reconnection**:
```toml
[reconnect]
max_attempts = 10
max_backoff = 30000  # 30 seconds
```

2. **Enable health monitoring**:
```toml
[health]
enabled = true
unhealthy_threshold = 3
```

3. **Set up backups**:
```toml
[backup]
enabled = true
schedule = "0 2 * * *"  # Daily at 2 AM
retention_count = 7
```

## Troubleshooting Configuration Issues

### Debug Mode

Enable debug logging to see configuration loading:

```bash
STREAM_MANAGER_LOGGING_LEVEL=debug stream-manager
```

### Configuration Dump

View the effective configuration:

```bash
stream-manager --config config.toml --dump-config
```

### Validation Only

Validate configuration without starting:

```bash
stream-manager --config config.toml --validate
```

## Migration from Other Systems

### From MediaMTX

```yaml
# MediaMTX config
paths:
  cam1:
    source: rtsp://camera:554/stream
    sourceOnDemand: yes
```

Converts to:

```toml
[[streams]]
id = "cam1"
source_url = "rtsp://camera:554/stream"
reconnect.enabled = true
```

### From DeepStream

```ini
# DeepStream config
[source0]
type=4
uri=rtsp://camera:554/stream
```

Converts to:

```toml
[[streams]]
id = "source0"
source_url = "rtsp://camera:554/stream"
inference.enabled = true
```

## Configuration Examples

### Minimal Configuration

```toml
[server]
port = 3000

[recording]
base_path = "/recordings"
```

### High-Performance Configuration

```toml
[server]
workers = 16
max_connections = 5000

[pipeline]
decode_threads = 8
use_hardware_acceleration = true

[performance]
enable_cpu_affinity = true
cpu_cores = [0, 1, 2, 3, 4, 5, 6, 7]
io_threads = 8
memory_limit = "16GB"
```

### High-Availability Configuration

```toml
[reconnect]
max_attempts = -1  # Infinite retries
max_backoff = 60000

[health]
check_interval = 10
recovery_threshold = 1

[backup]
enabled = true
schedule = "0 */6 * * *"  # Every 6 hours

[database]
type = "postgres"
max_connections = 20
```

## Further Reading

- [Deployment Guide](DEPLOYMENT.md) - Production deployment instructions
- [API Documentation](API.md) - REST API reference
- [Troubleshooting Guide](TROUBLESHOOTING.md) - Common issues and solutions
