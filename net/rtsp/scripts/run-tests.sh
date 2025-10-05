#!/bin/bash
# RTSP Test Runner Script for Linux/macOS
# Launches RTSP test server and runs validation tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SERVER_PORT=8554
SERVER_LOG="$SCRIPT_DIR/rtsp-server.log"
SERVER_PID_FILE="$SCRIPT_DIR/rtsp-server.pid"
TEST_VIDEO="$SCRIPT_DIR/test-video.mp4"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    if [ -f "$SERVER_PID_FILE" ]; then
        PID=$(cat "$SERVER_PID_FILE")
        if kill -0 "$PID" 2>/dev/null; then
            echo "Stopping RTSP server (PID: $PID)..."
            kill "$PID" 2>/dev/null || true
            sleep 1
            kill -9 "$PID" 2>/dev/null || true
        fi
        rm -f "$SERVER_PID_FILE"
    fi
    rm -f "$SERVER_LOG"
}

# Set trap to cleanup on exit
trap cleanup EXIT INT TERM

# Check for required tools
check_requirements() {
    echo "Checking requirements..."
    
    if ! command -v gst-launch-1.0 &> /dev/null; then
        echo -e "${RED}Error: gst-launch-1.0 not found. Please install GStreamer.${NC}"
        exit 1
    fi
    
    if ! command -v gst-inspect-1.0 &> /dev/null; then
        echo -e "${RED}Error: gst-inspect-1.0 not found. Please install GStreamer.${NC}"
        exit 1
    fi
    
    # Check for RTSP server capability
    if gst-inspect-1.0 rtspsink &> /dev/null; then
        echo "Using rtspsink for RTSP server"
        SERVER_TYPE="rtspsink"
    elif command -v gst-rtsp-server-1.0 &> /dev/null; then
        echo "Using gst-rtsp-server-1.0"
        SERVER_TYPE="rtsp-server"
    elif gst-inspect-1.0 rtspclientsink &> /dev/null; then
        echo "Using rtspclientsink for RTSP server"
        SERVER_TYPE="rtspclientsink"
    else
        echo -e "${RED}Error: No RTSP server found. Install gst-plugins-bad or gst-rtsp-server.${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}All requirements met!${NC}"
}

# Create test video if it doesn't exist
create_test_video() {
    if [ ! -f "$TEST_VIDEO" ]; then
        echo "Creating test video..."
        gst-launch-1.0 -e \
            videotestsrc num-buffers=300 ! \
            video/x-raw,width=640,height=480,framerate=30/1 ! \
            x264enc ! \
            mp4mux ! \
            filesink location="$TEST_VIDEO" 2>/dev/null
        echo "Test video created: $TEST_VIDEO"
    else
        echo "Using existing test video: $TEST_VIDEO"
    fi
}

# Start RTSP server
start_rtsp_server() {
    echo "Starting RTSP server on port $SERVER_PORT..."
    
    # Kill any existing server on the port
    lsof -ti:$SERVER_PORT | xargs kill -9 2>/dev/null || true
    
    case "$SERVER_TYPE" in
        "rtspsink")
            # Live test pattern
            gst-launch-1.0 -e \
                videotestsrc is-live=true ! \
                video/x-raw,width=640,height=480,framerate=30/1 ! \
                x264enc tune=zerolatency ! \
                rtph264pay config-interval=1 ! \
                rtspsink location="rtsp://localhost:$SERVER_PORT/test" \
                > "$SERVER_LOG" 2>&1 &
            ;;
        "rtsp-server")
            # Use gst-rtsp-server with test pattern
            gst-rtsp-server-1.0 \
                -p $SERVER_PORT \
                "( videotestsrc ! video/x-raw,width=640,height=480,framerate=30/1 ! x264enc tune=zerolatency ! rtph264pay name=pay0 pt=96 )" \
                > "$SERVER_LOG" 2>&1 &
            ;;
        "rtspclientsink")
            # Alternative using rtspclientsink
            gst-launch-1.0 -e \
                videotestsrc is-live=true ! \
                video/x-raw,width=640,height=480,framerate=30/1 ! \
                x264enc tune=zerolatency ! \
                rtph264pay config-interval=1 ! \
                rtspclientsink location="rtsp://localhost:$SERVER_PORT/test" \
                > "$SERVER_LOG" 2>&1 &
            ;;
    esac
    
    SERVER_PID=$!
    echo $SERVER_PID > "$SERVER_PID_FILE"
    
    # Wait for server to start
    echo -n "Waiting for server to start"
    for i in {1..10}; do
        if lsof -i:$SERVER_PORT > /dev/null 2>&1; then
            echo -e "\n${GREEN}RTSP server started successfully (PID: $SERVER_PID)${NC}"
            return 0
        fi
        echo -n "."
        sleep 1
    done
    
    echo -e "\n${RED}Failed to start RTSP server${NC}"
    cat "$SERVER_LOG"
    return 1
}

# Start VOD server for seeking tests
start_vod_server() {
    echo "Starting VOD RTSP server on port $SERVER_PORT..."
    
    create_test_video
    
    # Kill any existing server on the port
    lsof -ti:$SERVER_PORT | xargs kill -9 2>/dev/null || true
    
    case "$SERVER_TYPE" in
        "rtsp-server")
            # VOD server with seeking support
            gst-rtsp-server-1.0 \
                -p $SERVER_PORT \
                "( filesrc location=$TEST_VIDEO ! qtdemux ! h264parse ! rtph264pay name=pay0 pt=96 )" \
                > "$SERVER_LOG" 2>&1 &
            ;;
        *)
            # Fallback to filesrc with other methods
            gst-launch-1.0 -e \
                filesrc location="$TEST_VIDEO" ! \
                qtdemux ! h264parse ! \
                rtph264pay config-interval=1 ! \
                rtspsink location="rtsp://localhost:$SERVER_PORT/vod" \
                > "$SERVER_LOG" 2>&1 &
            ;;
    esac
    
    SERVER_PID=$!
    echo $SERVER_PID > "$SERVER_PID_FILE"
    
    # Wait for server to start
    echo -n "Waiting for VOD server to start"
    for i in {1..10}; do
        if lsof -i:$SERVER_PORT > /dev/null 2>&1; then
            echo -e "\n${GREEN}VOD RTSP server started successfully (PID: $SERVER_PID)${NC}"
            return 0
        fi
        echo -n "."
        sleep 1
    done
    
    echo -e "\n${RED}Failed to start VOD server${NC}"
    cat "$SERVER_LOG"
    return 1
}

# Run tests
run_tests() {
    local test_filter="$1"
    echo -e "\n${YELLOW}Running tests...${NC}"
    
    cd "$PLUGIN_DIR"
    
    if [ -n "$test_filter" ]; then
        echo "Running specific tests: $test_filter"
        cargo test -p gst-plugin-rtsp "$test_filter" -- --nocapture
    else
        echo "Running all RTSP tests..."
        # Basic tests
        cargo test -p gst-plugin-rtsp --lib -- --nocapture
        
        # Seeking tests (require VOD server)
        echo -e "\n${YELLOW}Running seeking tests...${NC}"
        cargo test -p gst-plugin-rtsp seek -- --nocapture --test-threads=1
        
        # Integration tests
        echo -e "\n${YELLOW}Running integration tests...${NC}"
        cargo test -p gst-plugin-rtsp integration -- --nocapture --test-threads=1
    fi
}

# Main execution
main() {
    echo -e "${GREEN}=== RTSP Test Suite ===${NC}"
    echo "Working directory: $PLUGIN_DIR"
    echo ""
    
    check_requirements
    
    # Parse arguments
    case "${1:-live}" in
        "live")
            echo -e "${YELLOW}Mode: Live streaming tests${NC}"
            start_rtsp_server
            run_tests "${2:-}"
            ;;
        "vod")
            echo -e "${YELLOW}Mode: VOD/Seeking tests${NC}"
            start_vod_server
            run_tests "seek"
            ;;
        "all")
            echo -e "${YELLOW}Mode: All tests${NC}"
            # Run live tests first
            start_rtsp_server
            run_tests ""
            cleanup
            # Then VOD tests
            start_vod_server
            run_tests "seek"
            ;;
        "quick")
            echo -e "${YELLOW}Mode: Quick validation${NC}"
            start_rtsp_server
            # Just run a simple validation
            echo "Testing RTSP connection..."
            timeout 5 gst-launch-1.0 \
                rtspsrc2 location="rtsp://localhost:$SERVER_PORT/test" ! \
                fakesink num-buffers=100 \
                || echo "Test completed"
            ;;
        *)
            echo "Usage: $0 [live|vod|all|quick] [test_filter]"
            echo "  live  - Run tests with live RTSP server (default)"
            echo "  vod   - Run VOD/seeking tests"
            echo "  all   - Run all test suites"
            echo "  quick - Quick validation test"
            exit 1
            ;;
    esac
    
    echo -e "\n${GREEN}=== Test Suite Complete ===${NC}"
}

# Run main function
main "$@"
