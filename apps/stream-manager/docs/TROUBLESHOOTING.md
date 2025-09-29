# Stream Manager Troubleshooting Guide

## Quick Diagnostics

Before diving into specific issues, run these quick checks:

```bash
# Check service status
systemctl status stream-manager

# Check recent logs
journalctl -u stream-manager -n 50 --no-pager

# Verify configuration
stream-manager --config /etc/stream-manager/config.toml --validate

# Test API connectivity
curl -v http://localhost:3000/health

# Check GStreamer installation
gst-inspect-1.0 --version
```

## Common Issues and Solutions

### Service Issues

#### Problem: Service Won't Start

**Symptoms:**
- `systemctl start stream-manager` fails
- Service exits immediately after starting

**Solutions:**

1. **Check configuration syntax:**
```bash
stream-manager --config /etc/stream-manager/config.toml --validate
```

2. **Verify file permissions:**
```bash
# Check ownership
ls -la /etc/stream-manager/
ls -la /var/lib/stream-manager/
ls -la /var/recordings/

# Fix permissions
sudo chown -R stream-manager:stream-manager /etc/stream-manager
sudo chown -R stream-manager:stream-manager /var/lib/stream-manager
sudo chown -R stream-manager:stream-manager /var/recordings
```

3. **Check port availability:**
```bash
# Check if ports are in use
sudo netstat -tulpn | grep -E "3000|8554|9090"
sudo ss -tulpn | grep -E "3000|8554|9090"

# Find process using port
sudo lsof -i :3000
```

4. **Review error logs:**
```bash
# System logs
journalctl -xe

# Application logs
tail -f /var/log/stream-manager/error.log
```

5. **Test with minimal configuration:**
```toml
# /tmp/minimal.toml
[server]
port = 3000

[recording]
enabled = false
```
```bash
stream-manager --config /tmp/minimal.toml
```

#### Problem: Service Crashes Repeatedly

**Symptoms:**
- Service restarts every few seconds
- `systemctl status` shows "activating (auto-restart)"

**Solutions:**

1. **Check memory limits:**
```bash
# View current limits
systemctl show stream-manager | grep -i memory

# Increase memory limit
sudo systemctl edit stream-manager
# Add:
# [Service]
# MemoryMax=8G
```

2. **Check for core dumps:**
```bash
# Enable core dumps
ulimit -c unlimited

# Find core files
find /var/lib/systemd/coredump -name "core.stream-manager*"

# Analyze core dump
coredumpctl info stream-manager
coredumpctl gdb stream-manager
```

3. **Reset service failure counter:**
```bash
sudo systemctl reset-failed stream-manager
sudo systemctl start stream-manager
```

### Stream Connection Issues

#### Problem: Cannot Connect to RTSP Stream

**Symptoms:**
- "Connection refused" or "Connection timeout" errors
- Stream shows as "disconnected" in API

**Solutions:**

1. **Test stream directly:**
```bash
# Test with GStreamer
gst-launch-1.0 rtspsrc location=rtsp://camera:554/stream ! fakesink

# Test with ffmpeg
ffmpeg -rtsp_transport tcp -i rtsp://camera:554/stream -t 5 -f null -

# Test with curl (basic connectivity)
curl -v telnet://camera:554
```

2. **Check network connectivity:**
```bash
# Ping camera
ping camera-ip

# Traceroute
traceroute camera-ip

# Check routing
ip route get camera-ip
```

3. **Verify credentials:**
```toml
[[streams]]
id = "camera-1"
source_url = "rtsp://username:password@camera:554/stream"
```

4. **Try different RTSP transports:**
```toml
[[streams]]
id = "camera-1"
source_url = "rtsp://camera:554/stream"
rtsp_transport = "tcp"  # or "udp", "http"
```

5. **Check firewall rules:**
```bash
# List rules
sudo iptables -L -n -v
sudo ufw status verbose

# Allow RTSP
sudo ufw allow 554/tcp
sudo ufw allow 554/udp
```

#### Problem: Stream Keeps Reconnecting

**Symptoms:**
- Stream connects then immediately disconnects
- Logs show continuous reconnection attempts

**Solutions:**

1. **Increase timeout values:**
```toml
[streams]
default_timeout = 30000  # 30 seconds

[reconnect]
initial_backoff = 5000   # 5 seconds
max_backoff = 60000      # 60 seconds
```

2. **Check stream stability:**
```bash
# Monitor stream for 60 seconds
timeout 60 gst-launch-1.0 rtspsrc location=rtsp://camera:554/stream ! \
    fakesink silent=false 2>&1 | grep -c "ERROR"
```

3. **Reduce load:**
```toml
[streams]
max_streams = 50  # Reduce from 100

[pipeline]
decode_threads = 2  # Reduce from 4
```

### Recording Issues

#### Problem: Recordings Not Being Created

**Symptoms:**
- No files in recording directory
- API shows recording as "inactive"

**Solutions:**

1. **Check disk space:**
```bash
df -h /var/recordings
du -sh /var/recordings/*
```

2. **Verify recording is enabled:**
```bash
# Check via API
curl http://localhost:3000/api/v1/streams/camera-1 | jq '.recording'

# Enable recording
curl -X POST http://localhost:3000/api/v1/streams/camera-1/recording/start
```

3. **Check write permissions:**
```bash
# Test write access
sudo -u stream-manager touch /var/recordings/test.txt
```

4. **Monitor GStreamer pipeline:**
```bash
# Enable debug logging
GST_DEBUG=3 stream-manager 2>&1 | grep -i "record"
```

#### Problem: Recording Files Are Corrupted

**Symptoms:**
- Cannot play recorded files
- Files have 0 bytes or very small size

**Solutions:**

1. **Check file system:**
```bash
# Check for errors
sudo fsck -n /dev/sda1

# Check inode usage
df -i /var/recordings
```

2. **Verify segment settings:**
```toml
[recording]
segment_duration = 600  # 10 minutes
container = "mp4"       # Use compatible container
```

3. **Test with different codec:**
```toml
[pipeline]
output_format = "H264"  # Try H265 or VP8
```

### Performance Issues

#### Problem: High CPU Usage

**Symptoms:**
- CPU usage above 80%
- System becomes unresponsive

**Solutions:**

1. **Profile CPU usage:**
```bash
# Top processes
top -p $(pgrep stream-manager)

# Per-thread usage
top -H -p $(pgrep stream-manager)

# Perf analysis
sudo perf top -p $(pgrep stream-manager)
```

2. **Disable unnecessary features:**
```toml
[inference]
enabled = false  # Disable if not needed

[metrics]
detailed_metrics = false  # Reduce metrics collection
```

3. **Optimize pipeline:**
```toml
[pipeline]
use_hardware_acceleration = true
decode_threads = 4  # Match CPU cores
```

4. **Reduce stream quality:**
```toml
[[streams]]
max_bitrate = 2000000  # 2 Mbps limit
target_framerate = 15  # Reduce from 30
```

#### Problem: High Memory Usage

**Symptoms:**
- Memory usage continuously increases
- Out of memory errors

**Solutions:**

1. **Check for memory leaks:**
```bash
# Monitor memory over time
watch -n 5 'ps aux | grep stream-manager'

# Use valgrind (development only)
valgrind --leak-check=full stream-manager
```

2. **Limit buffer sizes:**
```toml
[streams]
buffer_size = 2048  # Reduce from 4096 KB

[network]
receive_buffer = "2MB"  # Reduce from 4MB
send_buffer = "2MB"
```

3. **Enable memory limits:**
```bash
# systemd limit
sudo systemctl edit stream-manager
# Add:
# [Service]
# MemoryMax=4G
# MemoryHigh=3G
```

#### Problem: High Disk I/O

**Symptoms:**
- Slow response times
- High iowait in top

**Solutions:**

1. **Monitor disk usage:**
```bash
# I/O statistics
iostat -x 1

# Process I/O
iotop -p $(pgrep stream-manager)
```

2. **Optimize recording settings:**
```toml
[recording]
segment_duration = 1800  # Larger segments = less I/O
compression = "none"     # Avoid CPU/IO trade-off
```

3. **Use separate disks:**
```toml
[recording]
base_path = "/mnt/ssd/recordings"  # Fast SSD

[database]
path = "/mnt/nvme/stream-manager.db"  # Separate disk
```

### API Issues

#### Problem: API Not Responding

**Symptoms:**
- `curl` commands timeout
- Cannot access web interface

**Solutions:**

1. **Check API server:**
```bash
# Test locally
curl -v http://127.0.0.1:3000/health

# Check listening ports
netstat -tlnp | grep 3000
```

2. **Review nginx proxy (if used):**
```bash
# Test nginx
nginx -t
systemctl status nginx

# Check upstream
curl -H "Host: stream.example.com" http://127.0.0.1/api/health
```

3. **Check rate limiting:**
```bash
# View current limits
curl http://localhost:3000/api/v1/status | jq '.rate_limit'
```

#### Problem: Authentication Failures

**Symptoms:**
- 401 Unauthorized responses
- "Invalid token" errors

**Solutions:**

1. **Verify token configuration:**
```bash
# Check environment variable
echo $STREAM_MANAGER_AUTH_TOKEN

# Test with token
curl -H "Authorization: Bearer your-token" http://localhost:3000/api/v1/streams
```

2. **Check token expiration:**
```bash
# Decode JWT token (if using JWT)
echo "your.jwt.token" | cut -d. -f2 | base64 -d | jq '.exp'
```

### WebRTC Issues

#### Problem: WebRTC Connection Fails

**Symptoms:**
- ICE connection state: failed
- No audio/video in browser

**Solutions:**

1. **Check STUN/TURN servers:**
```toml
[webrtc]
stun_server = "stun:stun.l.google.com:19302"
turn_server = "turn:turnserver.com:3478"
turn_username = "user"
turn_password = "pass"
```

2. **Verify UDP ports:**
```bash
# Check if ports are open
sudo ufw allow 10000:10100/udp

# Test UDP connectivity
nc -u -l 10000  # On server
nc -u server-ip 10000  # On client
```

3. **Enable debug logging:**
```bash
GST_DEBUG=webrtcbin:5 stream-manager
```

### GStreamer Issues

#### Problem: Missing GStreamer Plugins

**Symptoms:**
- "No element X" errors
- Pipeline fails to create

**Solutions:**

1. **Install missing plugins:**
```bash
# Find which package provides element
apt-file search gstreamer | grep element-name

# Install common plugin packages
sudo apt-get install \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly
```

2. **Verify plugin installation:**
```bash
# List all elements
gst-inspect-1.0

# Check specific element
gst-inspect-1.0 x264enc
```

3. **Update plugin cache:**
```bash
rm -rf ~/.cache/gstreamer-1.0
gst-inspect-1.0 --print-all > /dev/null
```

## Debug Techniques

### Enable Verbose Logging

```bash
# Application debug logging
RUST_LOG=debug,stream_manager=trace stream-manager

# GStreamer debug logging
GST_DEBUG=3 stream-manager  # General debug
GST_DEBUG=rtspsrc:5 stream-manager  # Specific element
GST_DEBUG_FILE=/tmp/gst.log GST_DEBUG=4 stream-manager  # Log to file

# Combined
RUST_LOG=trace GST_DEBUG=3 stream-manager 2>&1 | tee debug.log
```

### Generate Debug Graphs

```bash
# Enable dot file generation
GST_DEBUG_DUMP_DOT_DIR=/tmp stream-manager

# Convert to image
dot -Tpng /tmp/pipeline.dot -o pipeline.png
```

### Network Debugging

```bash
# Capture network traffic
sudo tcpdump -i any -w stream.pcap port 554 or port 3000

# Analyze with Wireshark
wireshark stream.pcap

# Monitor bandwidth
iftop -i eth0
nethogs -p $(pgrep stream-manager)
```

### Strace Analysis

```bash
# Trace system calls
strace -f -e trace=network -p $(pgrep stream-manager)

# Trace file operations
strace -f -e trace=file -p $(pgrep stream-manager)

# Full trace
strace -f -o trace.log stream-manager
```

## Performance Profiling

### CPU Profiling

```bash
# Using perf
sudo perf record -g -p $(pgrep stream-manager) -- sleep 30
sudo perf report

# Flame graph
git clone https://github.com/brendangregg/FlameGraph
sudo perf record -F 99 -g -p $(pgrep stream-manager) -- sleep 30
sudo perf script | FlameGraph/stackcollapse-perf.pl | FlameGraph/flamegraph.pl > flame.svg
```

### Memory Profiling

```bash
# Heap profile
heaptrack stream-manager
heaptrack --analyze heaptrack.stream-manager.*.gz

# Memory maps
pmap -x $(pgrep stream-manager)

# Detailed memory usage
cat /proc/$(pgrep stream-manager)/status | grep -E "Vm|Rss"
```

## Emergency Recovery

### Reset Everything

```bash
#!/bin/bash
# Emergency reset script

# Stop service
sudo systemctl stop stream-manager

# Backup current state
sudo tar -czf /tmp/stream-manager-backup-$(date +%Y%m%d).tar.gz \
    /etc/stream-manager \
    /var/lib/stream-manager

# Clear state
sudo rm -rf /var/lib/stream-manager/*
sudo rm -rf /var/recordings/*

# Reset configuration to defaults
sudo cp /usr/share/stream-manager/config.default.toml /etc/stream-manager/config.toml

# Fix permissions
sudo chown -R stream-manager:stream-manager /etc/stream-manager
sudo chown -R stream-manager:stream-manager /var/lib/stream-manager
sudo chown -R stream-manager:stream-manager /var/recordings

# Start fresh
sudo systemctl start stream-manager
```

### Rollback to Previous Version

```bash
# Stop service
sudo systemctl stop stream-manager

# Restore previous binary
sudo cp /usr/local/bin/stream-manager.backup /usr/local/bin/stream-manager

# Restore previous config
sudo cp /etc/stream-manager/config.toml.backup /etc/stream-manager/config.toml

# Start service
sudo systemctl start stream-manager
```

## Getting Help

### Collect Diagnostic Information

```bash
#!/bin/bash
# diagnostic.sh - Collect system information for support

OUTPUT_DIR="/tmp/stream-manager-diagnostics-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUTPUT_DIR"

# System information
uname -a > "$OUTPUT_DIR/system.txt"
lsb_release -a >> "$OUTPUT_DIR/system.txt" 2>/dev/null
df -h >> "$OUTPUT_DIR/disk.txt"
free -h >> "$OUTPUT_DIR/memory.txt"

# Service status
systemctl status stream-manager > "$OUTPUT_DIR/service-status.txt" 2>&1
journalctl -u stream-manager -n 1000 > "$OUTPUT_DIR/service-logs.txt" 2>&1

# Configuration (sanitized)
sed 's/password=.*/password=REDACTED/g' /etc/stream-manager/config.toml > "$OUTPUT_DIR/config.toml"

# Network
netstat -tulpn > "$OUTPUT_DIR/network.txt" 2>&1
ip addr > "$OUTPUT_DIR/ip-addr.txt" 2>&1

# GStreamer
gst-inspect-1.0 --version > "$OUTPUT_DIR/gstreamer.txt" 2>&1
gst-inspect-1.0 >> "$OUTPUT_DIR/gstreamer.txt" 2>&1

# Application version
stream-manager --version > "$OUTPUT_DIR/version.txt" 2>&1

# Create archive
tar -czf "$OUTPUT_DIR.tar.gz" "$OUTPUT_DIR"
echo "Diagnostics collected: $OUTPUT_DIR.tar.gz"
```

### Support Channels

- **Documentation**: Check the [official documentation](https://docs.example.com/stream-manager)
- **GitHub Issues**: Report bugs at [GitHub Issues](https://github.com/your-org/stream-manager/issues)
- **Community Forum**: Ask questions at [community.example.com](https://community.example.com)
- **Enterprise Support**: Contact support@example.com for commercial support

When reporting issues, include:
1. Stream Manager version (`stream-manager --version`)
2. Operating system and version
3. Configuration file (sanitized)
4. Error messages and relevant logs
5. Steps to reproduce the issue
6. Diagnostic information from the script above
