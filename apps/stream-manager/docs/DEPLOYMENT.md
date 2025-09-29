# Stream Manager Deployment Guide

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Installation Methods](#installation-methods)
3. [Production Setup](#production-setup)
4. [Security Hardening](#security-hardening)
5. [Performance Tuning](#performance-tuning)
6. [High Availability](#high-availability)
7. [Monitoring & Alerting](#monitoring--alerting)
8. [Backup & Recovery](#backup--recovery)
9. [Troubleshooting](#troubleshooting)

## System Requirements

### Minimum Requirements

- **CPU**: 4 cores (x86_64 or ARM64)
- **RAM**: 8 GB
- **Storage**: 100 GB SSD for OS and application
- **Network**: 1 Gbps network interface
- **OS**: Ubuntu 20.04+, Debian 11+, RHEL 8+, or compatible

### Recommended Requirements

- **CPU**: 16+ cores
- **RAM**: 32 GB
- **Storage**: 
  - 500 GB NVMe SSD for active recordings
  - 10+ TB HDD for archive storage
- **Network**: 10 Gbps network interface
- **GPU** (for inference): NVIDIA GPU with 8+ GB VRAM

### Software Dependencies

```bash
# Ubuntu/Debian
apt-get update
apt-get install -y \
    gstreamer1.0-tools \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    gstreamer1.0-rtsp \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev

# RHEL/CentOS/Fedora
dnf install -y \
    gstreamer1 \
    gstreamer1-plugins-base \
    gstreamer1-plugins-good \
    gstreamer1-plugins-bad-free \
    gstreamer1-plugins-ugly \
    gstreamer1-rtsp-server \
    gstreamer1-devel \
    gstreamer1-plugins-base-devel
```

### GPU Support (Optional)

For NVIDIA GPU inference:

```bash
# Install NVIDIA drivers
apt-get install -y nvidia-driver-525

# Install CUDA Toolkit
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.0-1_all.deb
dpkg -i cuda-keyring_1.0-1_all.deb
apt-get update
apt-get install -y cuda-toolkit-12-3

# Install cuDNN
apt-get install -y libcudnn8

# Install TensorRT
apt-get install -y libnvinfer8 libnvinfer-plugin8
```

## Installation Methods

### Method 1: Binary Installation

```bash
# Download latest release
wget https://github.com/your-org/stream-manager/releases/latest/download/stream-manager-linux-amd64.tar.gz

# Extract
tar -xzvf stream-manager-linux-amd64.tar.gz -C /opt/

# Create symlink
ln -s /opt/stream-manager/bin/stream-manager /usr/local/bin/stream-manager

# Verify installation
stream-manager --version
```

### Method 2: From Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone repository
git clone https://github.com/your-org/gst-plugins-rs.git
cd gst-plugins-rs/apps/stream-manager

# Build release binary
cargo build --release

# Install
sudo cp target/release/stream-manager /usr/local/bin/
sudo chmod +x /usr/local/bin/stream-manager
```

### Method 3: Docker

```dockerfile
# Dockerfile
FROM rust:1.83 as builder

# Install GStreamer development packages
RUN apt-get update && apt-get install -y \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    pkg-config

# Build application
WORKDIR /app
COPY . .
RUN cargo build --release --package stream-manager

# Runtime image
FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    gstreamer1.0-tools \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    gstreamer1.0-rtsp \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/stream-manager /usr/local/bin/

# Create user
RUN useradd -m -s /bin/bash stream-manager

USER stream-manager
EXPOSE 3000 8554 9090

ENTRYPOINT ["stream-manager"]
```

Build and run:

```bash
docker build -t stream-manager .
docker run -d \
    --name stream-manager \
    -p 3000:3000 \
    -p 8554:8554 \
    -p 9090:9090 \
    -v /etc/stream-manager:/etc/stream-manager \
    -v /var/recordings:/var/recordings \
    stream-manager --config /etc/stream-manager/config.toml
```

### Method 4: Kubernetes/Helm

```yaml
# helm/values.yaml
replicaCount: 3

image:
  repository: your-registry/stream-manager
  tag: latest
  pullPolicy: IfNotPresent

service:
  type: LoadBalancer
  ports:
    api: 3000
    rtsp: 8554
    metrics: 9090

resources:
  limits:
    cpu: 4
    memory: 8Gi
  requests:
    cpu: 2
    memory: 4Gi

persistence:
  enabled: true
  storageClass: fast-ssd
  size: 500Gi

config:
  recording:
    base_path: /recordings
    retention_days: 7
  inference:
    device: gpu
```

Deploy:

```bash
helm install stream-manager ./helm
```

## Production Setup

### 1. Create System User

```bash
# Create dedicated user
sudo useradd -r -m -d /var/lib/stream-manager -s /bin/bash stream-manager

# Create directories
sudo mkdir -p /etc/stream-manager
sudo mkdir -p /var/log/stream-manager
sudo mkdir -p /var/recordings
sudo mkdir -p /var/lib/stream-manager

# Set permissions
sudo chown -R stream-manager:stream-manager /etc/stream-manager
sudo chown -R stream-manager:stream-manager /var/log/stream-manager
sudo chown -R stream-manager:stream-manager /var/recordings
sudo chown -R stream-manager:stream-manager /var/lib/stream-manager
```

### 2. Configure systemd Service

```ini
# /etc/systemd/system/stream-manager.service
[Unit]
Description=Stream Manager Service
Documentation=https://github.com/your-org/stream-manager
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
User=stream-manager
Group=stream-manager
WorkingDirectory=/var/lib/stream-manager

# Service configuration
Environment="RUST_LOG=info"
Environment="STREAM_MANAGER_CONFIG=/etc/stream-manager/config.toml"

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/recordings /var/lib/stream-manager /var/log/stream-manager
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictRealtime=true
RestrictNamespaces=true
RestrictSUIDSGID=true
LockPersonality=true

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096
MemoryLimit=8G
CPUQuota=400%

# Watchdog
WatchdogSec=30
Restart=always
RestartSec=5
StartLimitBurst=5
StartLimitInterval=60

# Start command
ExecStart=/usr/local/bin/stream-manager \
    --config /etc/stream-manager/config.toml \
    --log-level info

ExecReload=/bin/kill -SIGHUP $MAINPID
ExecStop=/bin/kill -SIGTERM $MAINPID

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable stream-manager
sudo systemctl start stream-manager
sudo systemctl status stream-manager
```

### 3. Configure Firewall

```bash
# UFW (Ubuntu/Debian)
sudo ufw allow 3000/tcp   # API
sudo ufw allow 8554/tcp   # RTSP
sudo ufw allow 9090/tcp   # Metrics
sudo ufw allow 10000:10100/udp  # WebRTC

# firewalld (RHEL/CentOS)
sudo firewall-cmd --permanent --add-port=3000/tcp
sudo firewall-cmd --permanent --add-port=8554/tcp
sudo firewall-cmd --permanent --add-port=9090/tcp
sudo firewall-cmd --permanent --add-port=10000-10100/udp
sudo firewall-cmd --reload
```

### 4. Setup Reverse Proxy (nginx)

```nginx
# /etc/nginx/sites-available/stream-manager
upstream stream_manager_api {
    server 127.0.0.1:3000;
    keepalive 32;
}

server {
    listen 80;
    server_name stream.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name stream.example.com;

    # SSL configuration
    ssl_certificate /etc/letsencrypt/live/stream.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/stream.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # API proxy
    location /api {
        proxy_pass http://stream_manager_api;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # WebSocket support
    location /api/v1/events {
        proxy_pass http://stream_manager_api;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WHIP/WHEP endpoints
    location /whip {
        proxy_pass http://stream_manager_api;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Security Hardening

### 1. Authentication & Authorization

```toml
# /etc/stream-manager/config.toml
[auth]
enabled = true
# Use environment variable for sensitive data
token = "${STREAM_MANAGER_AUTH_TOKEN}"
jwt_secret = "${STREAM_MANAGER_JWT_SECRET}"
token_expiry = 3600
```

### 2. TLS/SSL Configuration

```toml
[server.tls]
enabled = true
cert_file = "/etc/stream-manager/certs/server.crt"
key_file = "/etc/stream-manager/certs/server.key"
client_auth = false
ca_file = "/etc/stream-manager/certs/ca.crt"
```

### 3. Network Security

```bash
# Restrict access to management ports
iptables -A INPUT -p tcp --dport 3000 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 3000 -j DROP

# Rate limiting
iptables -A INPUT -p tcp --dport 3000 -m limit --limit 100/minute -j ACCEPT
```

### 4. File System Security

```bash
# Set secure permissions
chmod 600 /etc/stream-manager/config.toml
chmod 700 /var/lib/stream-manager
chmod 755 /var/recordings

# Enable SELinux (RHEL/CentOS)
semanage fcontext -a -t bin_t /usr/local/bin/stream-manager
restorecon -v /usr/local/bin/stream-manager
```

## Performance Tuning

### 1. Kernel Parameters

```bash
# /etc/sysctl.d/99-stream-manager.conf

# Network tuning
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.ipv4.tcp_rmem = 4096 87380 134217728
net.ipv4.tcp_wmem = 4096 65536 134217728
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_congestion_control = bbr

# File system
fs.file-max = 2097152
fs.inotify.max_user_watches = 524288

# Memory
vm.swappiness = 10
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5

# Apply settings
sudo sysctl -p /etc/sysctl.d/99-stream-manager.conf
```

### 2. Storage Optimization

```bash
# Mount recordings with optimized options
mount -o noatime,nodiratime /dev/nvme0n1p1 /var/recordings

# Setup RAID for redundancy and performance
mdadm --create /dev/md0 --level=10 --raid-devices=4 /dev/sd[abcd]
mkfs.ext4 -E stride=128,stripe-width=256 /dev/md0
```

### 3. Application Tuning

```toml
# Performance configuration
[performance]
enable_cpu_affinity = true
cpu_cores = [0, 1, 2, 3, 4, 5, 6, 7]
io_threads = 8
memory_limit = "16GB"
enable_huge_pages = true

[pipeline]
decode_threads = 8
use_hardware_acceleration = true

[server]
workers = 16
max_connections = 5000
```

## High Availability

### 1. Active-Passive Setup

```bash
# Install Pacemaker/Corosync
apt-get install -y pacemaker corosync

# Configure cluster
crm configure primitive stream-manager systemd:stream-manager \
    op monitor interval=30s \
    op start timeout=60s \
    op stop timeout=60s

crm configure primitive vip ocf:heartbeat:IPaddr2 \
    params ip=192.168.1.100 cidr_netmask=24 \
    op monitor interval=10s

crm configure group stream-manager-group vip stream-manager
```

### 2. Load Balancing

```nginx
# HAProxy configuration
global
    maxconn 4096
    log 127.0.0.1 local0

defaults
    mode http
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms

frontend stream_manager_frontend
    bind *:80
    default_backend stream_manager_backend

backend stream_manager_backend
    balance roundrobin
    option httpchk GET /health
    server node1 192.168.1.10:3000 check
    server node2 192.168.1.11:3000 check
    server node3 192.168.1.12:3000 check
```

### 3. Database Replication

```toml
# Primary node
[database]
type = "postgres"
connection_string = "postgresql://user:pass@primary:5432/stream_manager"
enable_replication = true
role = "primary"

# Replica nodes
[database]
type = "postgres"
connection_string = "postgresql://user:pass@replica:5432/stream_manager"
enable_replication = true
role = "replica"
primary_host = "primary.example.com"
```

## Monitoring & Alerting

### 1. Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'stream-manager'
    static_configs:
      - targets: ['localhost:9090']
    metric_relabel_configs:
      - source_labels: [__name__]
        regex: 'stream_manager_.*'
        action: keep
```

### 2. Grafana Dashboard

Import the provided dashboard JSON from `monitoring/grafana-dashboard.json`.

Key metrics to monitor:
- Active streams count
- Stream error rate
- Recording disk usage
- API response time
- CPU and memory usage
- Network throughput

### 3. Alerting Rules

```yaml
# alerts.yml
groups:
  - name: stream_manager
    rules:
      - alert: HighStreamErrorRate
        expr: rate(stream_manager_stream_errors_total[5m]) > 0.1
        for: 5m
        annotations:
          summary: "High stream error rate"
      
      - alert: DiskSpaceLow
        expr: stream_manager_disk_free_bytes < 10737418240
        for: 10m
        annotations:
          summary: "Less than 10GB disk space remaining"
      
      - alert: ServiceDown
        expr: up{job="stream-manager"} == 0
        for: 1m
        annotations:
          summary: "Stream Manager service is down"
```

## Backup & Recovery

### 1. Automated Backups

```bash
#!/bin/bash
# /usr/local/bin/stream-manager-backup.sh

BACKUP_DIR="/var/backups/stream-manager"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup configuration
tar -czf "$BACKUP_DIR/config_$TIMESTAMP.tar.gz" /etc/stream-manager/

# Backup database
pg_dump stream_manager | gzip > "$BACKUP_DIR/database_$TIMESTAMP.sql.gz"

# Backup state
tar -czf "$BACKUP_DIR/state_$TIMESTAMP.tar.gz" /var/lib/stream-manager/

# Optional: Backup recordings metadata
find /var/recordings -name "*.json" | tar -czf "$BACKUP_DIR/metadata_$TIMESTAMP.tar.gz" -T -

# Cleanup old backups (keep 7 days)
find "$BACKUP_DIR" -name "*.tar.gz" -mtime +7 -delete
find "$BACKUP_DIR" -name "*.sql.gz" -mtime +7 -delete

# Sync to remote storage
aws s3 sync "$BACKUP_DIR" s3://backup-bucket/stream-manager/
```

Add to crontab:

```bash
0 2 * * * /usr/local/bin/stream-manager-backup.sh
```

### 2. Disaster Recovery

```bash
#!/bin/bash
# /usr/local/bin/stream-manager-restore.sh

BACKUP_DIR="/var/backups/stream-manager"
RESTORE_DATE=$1

if [ -z "$RESTORE_DATE" ]; then
    echo "Usage: $0 YYYYMMDD"
    exit 1
fi

# Stop service
systemctl stop stream-manager

# Restore configuration
tar -xzf "$BACKUP_DIR/config_$RESTORE_DATE*.tar.gz" -C /

# Restore database
gunzip -c "$BACKUP_DIR/database_$RESTORE_DATE*.sql.gz" | psql stream_manager

# Restore state
tar -xzf "$BACKUP_DIR/state_$RESTORE_DATE*.tar.gz" -C /

# Fix permissions
chown -R stream-manager:stream-manager /etc/stream-manager
chown -R stream-manager:stream-manager /var/lib/stream-manager

# Start service
systemctl start stream-manager
```

## Troubleshooting

### Common Issues

#### Service Won't Start

```bash
# Check logs
journalctl -u stream-manager -n 100 --no-pager

# Verify configuration
stream-manager --config /etc/stream-manager/config.toml --validate

# Check permissions
ls -la /etc/stream-manager/
ls -la /var/lib/stream-manager/

# Test with minimal config
stream-manager --config /etc/stream-manager/config.minimal.toml
```

#### High Memory Usage

```bash
# Check memory usage
ps aux | grep stream-manager
pmap -x $(pgrep stream-manager)

# Enable memory profiling
RUST_LOG=debug stream-manager --enable-profiling

# Limit memory
systemctl edit stream-manager
# Add: MemoryMax=4G
```

#### Stream Connection Issues

```bash
# Test RTSP connectivity
gst-launch-1.0 rtspsrc location=rtsp://camera:554/stream ! fakesink

# Check network
netstat -tunlp | grep stream-manager
ss -tunlp | grep stream-manager

# Firewall rules
iptables -L -n -v
```

### Debug Mode

```bash
# Run in debug mode
RUST_LOG=debug,stream_manager=trace stream-manager

# Enable GStreamer debugging
GST_DEBUG=3 stream-manager

# Core dump analysis
ulimit -c unlimited
gdb stream-manager core
```

### Performance Analysis

```bash
# CPU profiling
perf record -g stream-manager
perf report

# Memory profiling
valgrind --leak-check=full stream-manager

# I/O analysis
iotop -p $(pgrep stream-manager)
```

## Maintenance

### Log Rotation

```logrotate
# /etc/logrotate.d/stream-manager
/var/log/stream-manager/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 644 stream-manager stream-manager
    postrotate
        systemctl reload stream-manager
    endscript
}
```

### Updates

```bash
# Backup before update
/usr/local/bin/stream-manager-backup.sh

# Download new version
wget https://github.com/your-org/stream-manager/releases/latest/download/stream-manager-linux-amd64

# Replace binary
systemctl stop stream-manager
mv stream-manager-linux-amd64 /usr/local/bin/stream-manager
chmod +x /usr/local/bin/stream-manager

# Verify and restart
stream-manager --version
systemctl start stream-manager
```

### Health Checks

```bash
# API health check
curl http://localhost:3000/health

# Detailed status
curl http://localhost:3000/api/v1/status

# Metrics check
curl http://localhost:9090/metrics | grep stream_manager
```

## Support

For production support:
- Documentation: https://docs.example.com/stream-manager
- Issues: https://github.com/your-org/stream-manager/issues
- Enterprise Support: support@example.com
