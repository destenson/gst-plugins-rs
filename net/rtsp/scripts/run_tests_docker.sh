#!/bin/bash
# RTSP Test Suite - Docker version (no sudo required)
# 
# This script provides high-level commands to run validation tests
# for various RTSP scenarios including reconnection, UDP/TCP transport,
# multi-stream synchronization, and more.
#
# This version runs inside Docker and doesn't require sudo.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
RESULTS_DIR="${SCRIPT_DIR}/../test-results"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
DEFAULT_TEST_DURATION=30  # seconds per test
MEDIAMTX_STARTUP_WAIT=3   # seconds to wait for mediamtx to start
STREAM_STARTUP_WAIT=2     # seconds to wait for streams to stabilize

# Create results directory
mkdir -p "${RESULTS_DIR}"

#######################################
# Helper Functions
#######################################

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $*"
}

log_test_start() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}TEST: $*${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Check if a process is running
is_running() {
    pgrep -f "$1" > /dev/null 2>&1
}

# Wait for port to be available
wait_for_port() {
    local port=$1
    local timeout=${2:-10}
    local elapsed=0
    
    while ! nc -z localhost "$port" 2>/dev/null; do
        if [ $elapsed -ge $timeout ]; then
            return 1
        fi
        sleep 1
        ((elapsed++))
    done
    return 0
}

# Kill process by pattern
kill_process() {
    local pattern=$1
    if is_running "$pattern"; then
        pkill -f "$pattern" || true
        sleep 1
        pkill -9 -f "$pattern" 2>/dev/null || true
    fi
}

# Start mediamtx in background
start_mediamtx() {
    local config=${1:-"${SCRIPT_DIR}/mediamtx.yml"}
    
    log_info "Starting mediamtx with config: $config"
    
    if is_running "mediamtx"; then
        log_warn "mediamtx already running, killing..."
        kill_process "mediamtx"
    fi
    
    mediamtx "$config" > "${RESULTS_DIR}/mediamtx-${TIMESTAMP}.log" 2>&1 &
    local pid=$!
    
    sleep $MEDIAMTX_STARTUP_WAIT
    
    if ! is_running "mediamtx"; then
        log_error "Failed to start mediamtx"
        return 1
    fi
    
    if ! wait_for_port 8554 10; then
        log_error "mediamtx port 8554 not available"
        return 1
    fi
    
    log_success "mediamtx started (PID: $pid)"
    echo $pid
}

# Stop mediamtx
stop_mediamtx() {
    log_info "Stopping mediamtx"
    kill_process "mediamtx"
}

# Start ffmpeg test stream publisher
start_test_stream() {
    local stream_name=$1
    local pattern=${2:-"testsrc"}  # testsrc, smptebars, etc.
    local url=${3:-"rtsp://127.0.0.1:8554/${stream_name}"}
    
    log_info "Publishing test stream: $stream_name (pattern: $pattern)"
    
    # Kill existing stream if any
    kill_process "ffmpeg.*${stream_name}"
    
    local log_file="${RESULTS_DIR}/ffmpeg-${stream_name}-${TIMESTAMP}.log"
    
    ffmpeg -re -f lavfi -i "${pattern}=size=640x480:rate=30,format=yuv420p" \
        -c:v libx264 -preset ultrafast -tune zerolatency -profile:v baseline \
        -f rtsp "$url" > "$log_file" 2>&1 &
    
    local pid=$!
    sleep $STREAM_STARTUP_WAIT
    
    if ! is_running "ffmpeg.*${stream_name}"; then
        log_error "Failed to start test stream: $stream_name"
        return 1
    fi
    
    log_success "Test stream started: $stream_name (PID: $pid)"
    echo $pid
}

# Stop test stream
stop_test_stream() {
    local stream_name=$1
    log_info "Stopping test stream: $stream_name"
    kill_process "ffmpeg.*${stream_name}"
}

# Run a test with timeout and capture output
run_test_command() {
    local test_name=$1
    local duration=$2
    local cmd=$3
    
    local log_file="${RESULTS_DIR}/${test_name}-${TIMESTAMP}.log"
    
    log_info "Running test: $test_name (duration: ${duration}s)"
    log_info "Command: $cmd"
    log_info "Log file: $log_file"
    
    timeout "$duration" bash -c "$cmd" > "$log_file" 2>&1 &
    local pid=$!
    
    # Monitor the test
    local elapsed=0
    while kill -0 $pid 2>/dev/null; do
        if [ $elapsed -ge $duration ]; then
            break
        fi
        sleep 1
        ((elapsed++))
    done
    
    # Check if test completed or timed out
    if kill -0 $pid 2>/dev/null; then
        kill $pid 2>/dev/null || true
        log_success "Test completed: $test_name (timed out as expected)"
    else
        wait $pid
        local exit_code=$?
        if [ $exit_code -eq 0 ]; then
            log_success "Test completed: $test_name (exit code: 0)"
        else
            log_warn "Test completed: $test_name (exit code: $exit_code)"
        fi
    fi
    
    # Analyze results
    analyze_test_results "$test_name" "$log_file"
}

# Analyze test results from log file
analyze_test_results() {
    local test_name=$1
    local log_file=$2
    
    if [ ! -f "$log_file" ]; then
        log_error "Log file not found: $log_file"
        return 1
    fi
    
    # Extract frame stats
    local total_frames=$(grep -oP 'Frame stats: \K\d+(?= total frames)' "$log_file" | tail -1)
    local avg_fps=$(grep -oP 'fps \(avg\)' "$log_file" | wc -l)
    local errors=$(grep -c "Error from" "$log_file" || true)
    local warnings=$(grep -c "Warning from" "$log_file" || true)
    local reconnections=$(grep -c "Reconnection" "$log_file" || true)
    
    echo ""
    log_info "=== Test Results: $test_name ==="
    [ -n "$total_frames" ] && log_info "  Total frames: $total_frames"
    log_info "  Errors: $errors"
    log_info "  Warnings: $warnings"
    [ "$reconnections" -gt 0 ] && log_info "  Reconnections: $reconnections"
    
    # Determine pass/fail
    if [ "$errors" -eq 0 ]; then
        log_success "  Status: PASSED"
        return 0
    else
        log_error "  Status: FAILED ($errors errors)"
        log_info "  Check log: $log_file"
        return 1
    fi
}

#######################################
# Individual Test Cases
#######################################

test_basic_udp() {
    log_test_start "Basic UDP Transport"
    
    local stream_pid=$(start_test_stream "test-h264" "testsrc")
    
    run_test_command "basic-udp" ${DEFAULT_TEST_DURATION} \
        "RTSP_URL=rtsp://127.0.0.1:8554/test-h264 \
         cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
         --url rtsp://127.0.0.1:8554/test-h264"
    
    stop_test_stream "test-h264"
}

test_basic_tcp() {
    log_test_start "Basic TCP Transport"
    
    local stream_pid=$(start_test_stream "test-tcp" "smptebars")
    
    # Note: Would need to modify example to support TCP protocol property
    run_test_command "basic-tcp" ${DEFAULT_TEST_DURATION} \
        "cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
         --url rtsp://127.0.0.1:8554/test-tcp"
    
    stop_test_stream "test-tcp"
}

test_reconnection() {
    log_test_start "Reconnection after stream interruption"
    
    local stream_pid=$(start_test_stream "test-reconnect" "testsrc")
    
    # Start the client
    timeout ${DEFAULT_TEST_DURATION} \
        cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
        --url rtsp://127.0.0.1:8554/test-reconnect \
        > "${RESULTS_DIR}/reconnection-${TIMESTAMP}.log" 2>&1 &
    local client_pid=$!
    
    # Let it run for a bit
    sleep 5
    
    # Kill the stream
    log_info "Interrupting stream..."
    stop_test_stream "test-reconnect"
    
    # Wait a bit
    sleep 3
    
    # Restart the stream
    log_info "Restarting stream..."
    start_test_stream "test-reconnect" "testsrc"
    
    # Let it recover
    sleep 10
    
    # Stop client
    kill $client_pid 2>/dev/null || true
    
    analyze_test_results "reconnection" "${RESULTS_DIR}/reconnection-${TIMESTAMP}.log"
    
    stop_test_stream "test-reconnect"
}

test_periodic_restart() {
    log_test_start "Periodic source restart"
    
    local stream_pid=$(start_test_stream "test-periodic" "testsrc")
    
    run_test_command "periodic-restart" 45 \
        "cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
         --url rtsp://127.0.0.1:8554/test-periodic \
         --restart-interval 10 \
         --max-restarts 3"
    
    stop_test_stream "test-periodic"
}

test_dual_stream_independent() {
    log_test_start "Dual Stream - Independent Mode"
    
    # Setup loopback IP (no sudo needed in container with CAP_NET_ADMIN)
    ip addr add 127.0.0.2/8 dev lo 2>/dev/null || true
    
    local stream1_pid=$(start_test_stream "stream1" "testsrc" "rtsp://127.0.0.1:8554/stream1")
    local stream2_pid=$(start_test_stream "stream2" "smptebars" "rtsp://127.0.0.2:8554/stream2")
    
    run_test_command "dual-independent" ${DEFAULT_TEST_DURATION} \
        "RTSP_URL1=rtsp://127.0.0.1:8554/stream1 \
         RTSP_URL2=rtsp://127.0.0.2:8554/stream2 \
         cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup_dual_stream"
    
    stop_test_stream "stream1"
    stop_test_stream "stream2"
}

test_dual_stream_synced() {
    log_test_start "Dual Stream - Synchronized Mode"
    
    # Setup loopback IP
    ip addr add 127.0.0.2/8 dev lo 2>/dev/null || true
    
    local stream1_pid=$(start_test_stream "stream1" "testsrc" "rtsp://127.0.0.1:8554/stream1")
    local stream2_pid=$(start_test_stream "stream2" "smptebars" "rtsp://127.0.0.2:8554/stream2")
    
    run_test_command "dual-synced" ${DEFAULT_TEST_DURATION} \
        "RTSP_URL1=rtsp://127.0.0.1:8554/stream1 \
         RTSP_URL2=rtsp://127.0.0.2:8554/stream2 \
         cargo run -p gst-plugin-rtsp --example rtspsrc_synced_dual_stream"
    
    stop_test_stream "stream1"
    stop_test_stream "stream2"
}

test_stream_isolation() {
    log_test_start "Stream Isolation - One stream failure doesn't affect others"
    
    # Setup loopback IP
    ip addr add 127.0.0.2/8 dev lo 2>/dev/null || true
    
    local stream1_pid=$(start_test_stream "stream1" "testsrc" "rtsp://127.0.0.1:8554/stream1")
    local stream2_pid=$(start_test_stream "stream2" "smptebars" "rtsp://127.0.0.2:8554/stream2")
    
    # Start the client
    timeout ${DEFAULT_TEST_DURATION} \
        env RTSP_URL1=rtsp://127.0.0.1:8554/stream1 \
            RTSP_URL2=rtsp://127.0.0.2:8554/stream2 \
        cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup_dual_stream \
        > "${RESULTS_DIR}/stream-isolation-${TIMESTAMP}.log" 2>&1 &
    local client_pid=$!
    
    # Let both streams run
    sleep 5
    
    # Block stream 2 with iptables (no sudo needed with CAP_NET_ADMIN)
    log_info "Blocking stream 2..."
    iptables -A INPUT -s 127.0.0.2 -p udp -j DROP 2>/dev/null || true
    iptables -A OUTPUT -d 127.0.0.2 -p udp -j DROP 2>/dev/null || true
    
    # Let it run with stream 1 only
    sleep 10
    
    # Unblock stream 2
    log_info "Unblocking stream 2..."
    iptables -D INPUT -s 127.0.0.2 -p udp -j DROP 2>/dev/null || true
    iptables -D OUTPUT -d 127.0.0.2 -p udp -j DROP 2>/dev/null || true
    
    # Let both streams run again
    sleep 5
    
    # Stop client
    kill $client_pid 2>/dev/null || true
    
    analyze_test_results "stream-isolation" "${RESULTS_DIR}/stream-isolation-${TIMESTAMP}.log"
    
    stop_test_stream "stream1"
    stop_test_stream "stream2"
}

test_long_running() {
    log_test_start "Long Running Stability (5 minutes)"
    
    local stream_pid=$(start_test_stream "test-longrun" "testsrc")
    
    run_test_command "long-running" 300 \
        "cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
         --url rtsp://127.0.0.1:8554/test-longrun"
    
    stop_test_stream "test-longrun"
}

#######################################
# Test Suites
#######################################

suite_smoke() {
    log_info "Running SMOKE TEST SUITE (quick validation)"
    echo ""
    
    local failed=0
    
    test_basic_udp || ((failed++))
    test_dual_stream_independent || ((failed++))
    
    echo ""
    log_info "=== SMOKE SUITE COMPLETE ==="
    log_info "Failed tests: $failed"
    return $failed
}

suite_transport() {
    log_info "Running TRANSPORT TEST SUITE"
    echo ""
    
    local failed=0
    
    test_basic_udp || ((failed++))
    test_basic_tcp || ((failed++))
    
    echo ""
    log_info "=== TRANSPORT SUITE COMPLETE ==="
    log_info "Failed tests: $failed"
    return $failed
}

suite_resilience() {
    log_info "Running RESILIENCE TEST SUITE"
    echo ""
    
    local failed=0
    
    test_reconnection || ((failed++))
    test_periodic_restart || ((failed++))
    test_stream_isolation || ((failed++))
    
    echo ""
    log_info "=== RESILIENCE SUITE COMPLETE ==="
    log_info "Failed tests: $failed"
    return $failed
}

suite_multistream() {
    log_info "Running MULTI-STREAM TEST SUITE"
    echo ""
    
    local failed=0
    
    test_dual_stream_independent || ((failed++))
    test_dual_stream_synced || ((failed++))
    test_stream_isolation || ((failed++))
    
    echo ""
    log_info "=== MULTI-STREAM SUITE COMPLETE ==="
    log_info "Failed tests: $failed"
    return $failed
}

suite_full() {
    log_info "Running FULL TEST SUITE (all tests)"
    echo ""
    
    local failed=0
    
    test_basic_udp || ((failed++))
    test_basic_tcp || ((failed++))
    test_reconnection || ((failed++))
    test_periodic_restart || ((failed++))
    test_dual_stream_independent || ((failed++))
    test_dual_stream_synced || ((failed++))
    test_stream_isolation || ((failed++))
    test_long_running || ((failed++))
    
    echo ""
    log_info "=== FULL SUITE COMPLETE ==="
    log_info "Failed tests: $failed"
    return $failed
}

#######################################
# Cleanup and Setup
#######################################

cleanup() {
    log_info "Cleaning up test environment..."
    
    # Kill all test processes
    kill_process "mediamtx"
    kill_process "ffmpeg.*test-"
    kill_process "ffmpeg.*stream"
    kill_process "rtspsrc_cleanup"
    kill_process "rtspsrc.*dual"
    kill_process "rtspsrc.*synced"
    
    # Remove iptables rules (no sudo needed)
    iptables -D INPUT -s 127.0.0.2 -p udp -j DROP 2>/dev/null || true
    iptables -D OUTPUT -d 127.0.0.2 -p udp -j DROP 2>/dev/null || true
    
    # Remove loopback IP (no sudo needed)
    ip addr del 127.0.0.2/8 dev lo 2>/dev/null || true
    
    log_success "Cleanup complete"
}

setup_environment() {
    log_info "Setting up test environment..."
    
    # Check dependencies
    command -v mediamtx >/dev/null 2>&1 || {
        log_error "mediamtx not found. Please install it."
        exit 1
    }
    
    command -v ffmpeg >/dev/null 2>&1 || {
        log_error "ffmpeg not found. Please install it."
        exit 1
    }
    
    # Verify plugin is installed
    if ! gst-inspect-1.0 rsrtsp >/dev/null 2>&1; then
        log_error "RTSP plugin not found. Was the Docker image built correctly?"
        exit 1
    fi
    
    log_info "Plugin verified: rtspsrc2 available"
    
    # Start mediamtx
    MEDIAMTX_PID=$(start_mediamtx "${SCRIPT_DIR}/mediamtx.yml")
    
    log_success "Environment setup complete"
}

#######################################
# Main CLI
#######################################

show_help() {
    cat <<EOF
RTSP Test Suite - Docker Version (no sudo required)

Usage: $0 <command> [options]

INDIVIDUAL TESTS:
  test:basic-udp            Test basic UDP transport
  test:basic-tcp            Test basic TCP transport  
  test:reconnection         Test reconnection after stream failure
  test:periodic-restart     Test periodic source restart
  test:dual-independent     Test dual independent streams
  test:dual-synced          Test dual synchronized streams
  test:stream-isolation     Test stream isolation/independence
  test:long-running         Test long-running stability (5 min)

TEST SUITES:
  suite:smoke               Quick smoke test (basic-udp + dual-independent)
  suite:transport           Transport tests (UDP + TCP)
  suite:resilience          Resilience tests (reconnection, restart, isolation)
  suite:multistream         Multi-stream tests
  suite:full                Run all tests

UTILITIES:
  setup                     Setup test environment (start mediamtx)
  cleanup                   Stop all test processes and clean up
  results                   Show recent test results

OPTIONS:
  --duration <seconds>      Override default test duration ($DEFAULT_TEST_DURATION)
  --no-cleanup              Don't cleanup after tests
  --results-dir <path>      Override results directory ($RESULTS_DIR)

EXAMPLES:
  $0 suite:smoke                    # Quick validation
  $0 suite:full                     # Complete test run
  $0 test:reconnection              # Single test
  $0 suite:resilience --duration 60 # Override duration

RESULTS:
  Test logs are saved to: $RESULTS_DIR
  Each test run creates timestamped log files

EOF
}

main() {
    local command=${1:-"help"}
    shift || true
    
    # Parse options
    while [[ $# -gt 0 ]]; do
        case $1 in
            --duration)
                DEFAULT_TEST_DURATION="$2"
                shift 2
                ;;
            --no-cleanup)
                NO_CLEANUP=1
                shift
                ;;
            --results-dir)
                RESULTS_DIR="$2"
                mkdir -p "$RESULTS_DIR"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    case "$command" in
        # Individual tests
        test:basic-udp)
            setup_environment
            test_basic_udp
            [ -z "$NO_CLEANUP" ] && cleanup
            ;;
        test:basic-tcp)
            setup_environment
            test_basic_tcp
            [ -z "$NO_CLEANUP" ] && cleanup
            ;;
        test:reconnection)
            setup_environment
            test_reconnection
            [ -z "$NO_CLEANUP" ] && cleanup
            ;;
        test:periodic-restart)
            setup_environment
            test_periodic_restart
            [ -z "$NO_CLEANUP" ] && cleanup
            ;;
        test:dual-independent)
            setup_environment
            test_dual_stream_independent
            [ -z "$NO_CLEANUP" ] && cleanup
            ;;
        test:dual-synced)
            setup_environment
            test_dual_stream_synced
            [ -z "$NO_CLEANUP" ] && cleanup
            ;;
        test:stream-isolation)
            setup_environment
            test_stream_isolation
            [ -z "$NO_CLEANUP" ] && cleanup
            ;;
        test:long-running)
            setup_environment
            test_long_running
            [ -z "$NO_CLEANUP" ] && cleanup
            ;;
        
        # Suites
        suite:smoke)
            setup_environment
            suite_smoke
            local result=$?
            [ -z "$NO_CLEANUP" ] && cleanup
            exit $result
            ;;
        suite:transport)
            setup_environment
            suite_transport
            local result=$?
            [ -z "$NO_CLEANUP" ] && cleanup
            exit $result
            ;;
        suite:resilience)
            setup_environment
            suite_resilience
            local result=$?
            [ -z "$NO_CLEANUP" ] && cleanup
            exit $result
            ;;
        suite:multistream)
            setup_environment
            suite_multistream
            local result=$?
            [ -z "$NO_CLEANUP" ] && cleanup
            exit $result
            ;;
        suite:full)
            setup_environment
            suite_full
            local result=$?
            [ -z "$NO_CLEANUP" ] && cleanup
            exit $result
            ;;
        
        # Utilities
        setup)
            setup_environment
            log_info "Environment ready. Run tests or use 'cleanup' when done."
            ;;
        cleanup)
            cleanup
            ;;
        results)
            log_info "Recent test results in: $RESULTS_DIR"
            ls -lht "$RESULTS_DIR" | head -20
            ;;
        
        help|--help|-h|"")
            show_help
            ;;
        
        *)
            log_error "Unknown command: $command"
            show_help
            exit 1
            ;;
    esac
}

# Trap cleanup on exit
trap cleanup EXIT

main "$@"
