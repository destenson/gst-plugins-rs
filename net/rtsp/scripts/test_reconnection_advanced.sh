#!/bin/bash

# Advanced RTSP reconnection test script with network simulation
# Requires tc (traffic control) for network simulation on Linux

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Configuration
RTSP_URL="${1:-rtsp://localhost:8554/test}"
PLUGIN_PATH="$PROJECT_ROOT/target/debug"
TEST_DURATION=60
DISCONNECT_INTERVAL=15

echo -e "${GREEN}Advanced RTSP Reconnection Testing Suite${NC}"
echo "==========================================="
echo

# Check if running as root for network simulation
if [ "$EUID" -ne 0 ]; then 
    echo -e "${YELLOW}Warning: Not running as root. Network simulation tests will be skipped.${NC}"
    echo "Run with sudo for full network simulation capabilities."
    echo
fi

# Set up environment
export GST_PLUGIN_PATH="${PLUGIN_PATH}:${GST_PLUGIN_PATH}"
export GST_DEBUG="rtspsrc:6"

# Function to start RTSP pipeline in background
start_pipeline() {
    local test_name="$1"
    local pipeline_args="$2"
    local log_file="/tmp/rtsp_test_${test_name// /_}.log"
    
    echo -e "${GREEN}Starting test: ${test_name}${NC}"
    echo "Log file: $log_file"
    
    gst-launch-1.0 \
        rtspsrc2 location="$RTSP_URL" \
        protocols=tcp \
        max-reconnection-attempts=10 \
        reconnection-timeout=2000000000 \
        retry-strategy=exponential \
        initial-retry-delay=1000000000 \
        $pipeline_args \
        ! decodebin \
        ! fakesink \
        > "$log_file" 2>&1 &
    
    echo $!
}

# Function to monitor reconnection events
monitor_reconnections() {
    local pid="$1"
    local log_file="$2"
    local duration="$3"
    
    echo "Monitoring for $duration seconds..."
    
    local start_time=$(date +%s)
    local reconnect_count=0
    local last_line_count=0
    
    while [ $(($(date +%s) - start_time)) -lt $duration ]; do
        if ! kill -0 $pid 2>/dev/null; then
            echo -e "${RED}Pipeline terminated unexpectedly${NC}"
            break
        fi
        
        # Check for reconnection messages
        if [ -f "$log_file" ]; then
            local current_reconnects=$(grep -c "reconnection-attempt\|Reconnected via\|Connection lost" "$log_file" 2>/dev/null || echo 0)
            if [ $current_reconnects -gt $reconnect_count ]; then
                reconnect_count=$current_reconnects
                echo -e "${YELLOW}Reconnection event detected (total: $reconnect_count)${NC}"
                
                # Show last few relevant lines
                tail -n 20 "$log_file" | grep -E "(reconnect|connection|EOF|attempt|success)" | tail -n 3
            fi
        fi
        
        sleep 1
    done
    
    echo -e "${GREEN}Test completed. Total reconnections: $reconnect_count${NC}"
}

# Function to simulate network issues (requires root)
simulate_network_issues() {
    local interface="$1"
    local issue_type="$2"
    
    if [ "$EUID" -ne 0 ]; then
        echo -e "${YELLOW}Skipping network simulation (requires root)${NC}"
        return
    fi
    
    case $issue_type in
        "packet_loss")
            echo "Simulating 50% packet loss..."
            tc qdisc add dev $interface root netem loss 50%
            ;;
        "high_latency")
            echo "Simulating high latency (500ms)..."
            tc qdisc add dev $interface root netem delay 500ms
            ;;
        "bandwidth_limit")
            echo "Simulating bandwidth limitation (100kbps)..."
            tc qdisc add dev $interface root tbf rate 100kbit burst 32kbit latency 400ms
            ;;
        "connection_drop")
            echo "Simulating connection drop..."
            iptables -A OUTPUT -p tcp --dport 554 -j DROP
            ;;
        *)
            echo "Unknown network issue type: $issue_type"
            ;;
    esac
}

# Function to restore network
restore_network() {
    local interface="$1"
    
    if [ "$EUID" -ne 0 ]; then
        return
    fi
    
    echo "Restoring network conditions..."
    tc qdisc del dev $interface root 2>/dev/null || true
    iptables -D OUTPUT -p tcp --dport 554 -j DROP 2>/dev/null || true
}

# Test 1: Basic reconnection test
echo -e "\n${GREEN}=== Test 1: Basic Reconnection ===${NC}"
LOG_FILE="/tmp/rtsp_basic_reconnect.log"
PID=$(start_pipeline "basic_reconnection" "")

# Let it run for a bit
sleep 5

# Simulate server restart by killing connection (if you have access to the server)
echo -e "${YELLOW}Simulating connection interruption...${NC}"
echo "To test manually: restart your RTSP server now"

monitor_reconnections $PID $LOG_FILE 30

# Clean up
kill $PID 2>/dev/null || true
sleep 2

# Test 2: Reconnection with network issues (requires root)
if [ "$EUID" -eq 0 ]; then
    echo -e "\n${GREEN}=== Test 2: Reconnection with Packet Loss ===${NC}"
    
    # Get network interface
    INTERFACE=$(ip route get 8.8.8.8 | grep -oP 'dev \K[^ ]+')
    echo "Using network interface: $INTERFACE"
    
    LOG_FILE="/tmp/rtsp_packet_loss.log"
    PID=$(start_pipeline "packet_loss_reconnection" "")
    
    sleep 5
    simulate_network_issues $INTERFACE "packet_loss"
    sleep 10
    restore_network $INTERFACE
    
    monitor_reconnections $PID $LOG_FILE 20
    
    kill $PID 2>/dev/null || true
    restore_network $INTERFACE
fi

# Test 3: Multiple reconnections
echo -e "\n${GREEN}=== Test 3: Multiple Reconnections ===${NC}"
LOG_FILE="/tmp/rtsp_multiple_reconnect.log"
PID=$(start_pipeline "multiple_reconnections" "max-reconnection-attempts=-1")

# Simulate multiple disconnections
for i in {1..3}; do
    sleep 10
    echo -e "${YELLOW}Simulating disconnection #$i${NC}"
    
    if [ "$EUID" -eq 0 ]; then
        INTERFACE=$(ip route get 8.8.8.8 | grep -oP 'dev \K[^ ]+')
        simulate_network_issues $INTERFACE "connection_drop"
        sleep 3
        restore_network $INTERFACE
    else
        echo "Manual intervention needed: restart RTSP server"
        sleep 5
    fi
done

monitor_reconnections $PID $LOG_FILE 10

kill $PID 2>/dev/null || true

# Summary
echo
echo -e "${GREEN}=== Test Summary ===${NC}"
echo "Test logs available in /tmp/rtsp_*.log"
echo

# Analyze logs
for log in /tmp/rtsp_*.log; do
    if [ -f "$log" ]; then
        echo "$(basename $log):"
        echo "  - Total lines: $(wc -l < $log)"
        echo "  - Reconnection attempts: $(grep -c "reconnection-attempt" $log 2>/dev/null || echo 0)"
        echo "  - Successful reconnections: $(grep -c "Reconnected via" $log 2>/dev/null || echo 0)"
        echo "  - Connection errors: $(grep -c "EOF\|error" $log 2>/dev/null || echo 0)"
        echo
    fi
done

echo -e "${GREEN}Testing complete!${NC}"
