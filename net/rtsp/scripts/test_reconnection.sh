#!/bin/bash

echo "Testing RTSP reconnection logic..."
echo
echo "This test will:"
echo "1. Connect to an RTSP stream"
echo "2. Simulate connection loss"
echo "3. Verify automatic reconnection"
echo
echo "Make sure you have an RTSP server running or use a test URL."
echo

# Set GST_PLUGIN_PATH to include our plugin
export GST_PLUGIN_PATH="$(pwd)/target/debug:${GST_PLUGIN_PATH}"

# Enable debug output for our plugin
export GST_DEBUG="rtspsrc:6"

# Test with a sample RTSP URL (replace with your test server)
# You can use a public test stream or your own RTSP server
RTSP_URL="${1:-rtsp://localhost:8554/test}"

echo "Using RTSP URL: $RTSP_URL"
echo

# Function to test reconnection with different scenarios
test_reconnection() {
    local test_name="$1"
    local extra_args="$2"
    
    echo "----------------------------------------"
    echo "Test: $test_name"
    echo "----------------------------------------"
    echo "Starting pipeline with reconnection enabled..."
    echo "Extra args: $extra_args"
    echo
    
    timeout 30 gst-launch-1.0 \
        rtspsrc2 location="$RTSP_URL" \
        protocols=tcp \
        max-reconnection-attempts=5 \
        reconnection-timeout=3000000000 \
        retry-strategy=exponential \
        $extra_args \
        ! decodebin \
        ! fakesink \
        2>&1 | grep -E "(Reconnect|connection|EOF|attempt|success)"
    
    echo
}

# Run multiple test scenarios
echo "=== Test 1: Basic TCP reconnection ==="
test_reconnection "Basic TCP" ""

echo
echo "=== Test 2: UDP with fallback ==="
test_reconnection "UDP fallback" "protocols=udp"

echo
echo "=== Test 3: Auto retry mode ==="
test_reconnection "Auto retry" "retry-strategy=auto"

echo
echo "=== Test 4: Immediate retry ==="
test_reconnection "Immediate retry" "retry-strategy=immediate initial-retry-delay=0"

echo
echo "=== Test 5: Infinite retries ==="
test_reconnection "Infinite retries" "max-reconnection-attempts=-1"

echo
echo "----------------------------------------"
echo "Test completed."
echo
echo "To test with a public stream, try:"
echo "  $0 rtsp://wowzaec2demo.streamlock.net/vod/mp4:BigBuckBunny_115k.mp4"
echo
echo "To simulate connection loss:"
echo "  1. Start this script"
echo "  2. In another terminal, stop/restart your RTSP server"
echo "  3. Watch the reconnection attempts in the output"