#!/bin/bash
# Docker wrapper for RTSP test suite
# Builds and runs tests in an isolated container environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

# Get git commit hash and dirty status
get_git_version() {
    cd "$PROJECT_ROOT"
    local commit_hash=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
    local dirty=""
    
    # Check if there are uncommitted changes in net/rtsp/src directory
    if ! git diff --quiet HEAD -- net/rtsp/src 2>/dev/null; then
        dirty="-dirty"
    fi
    
    echo "${commit_hash}${dirty}"
}

GIT_VERSION=$(get_git_version)
IMAGE_NAME="gst-rtsp-test:${GIT_VERSION}"
CONTAINER_NAME="gst-rtsp-test-runner-${GIT_VERSION}"

show_help() {
    cat <<EOF
Docker RTSP Test Suite Wrapper

Usage: $0 <command> [test-args...]

COMMANDS:
  build                 Build the Docker test image
  run <test-args>       Run tests in Docker container
  shell                 Open interactive shell in container
  clean                 Remove test container and image
  clean-all             Remove all gst-rtsp-test images
  logs                  Show test results from last run
  version               Show current git version

EXAMPLES:
  $0 build                      # Build test image
  $0 run suite:smoke            # Run smoke tests
  $0 run suite:full             # Run all tests
  $0 run test:basic-udp         # Run single test
  $0 shell                      # Interactive debugging
  $0 clean                      # Clean up current version
  $0 clean-all                  # Remove all test images

VERSIONING:
  Images are tagged with git commit hash: gst-rtsp-test:<commit>[-dirty]
  Current version: $GIT_VERSION
  Current image: $IMAGE_NAME

TEST ARGUMENTS:
  All arguments after 'run' are passed to run_tests.sh inside the container.
  See './run_tests.sh help' for available tests.

EOF
}

build_image() {
    log_info "Building Docker test image: $IMAGE_NAME"
    
    if [ ! -f "$SCRIPT_DIR/Dockerfile.test" ]; then
        log_error "Dockerfile.test not found in $SCRIPT_DIR"
        exit 1
    fi
    
    # Build the plugin first (release mode for Docker)
    log_info "Building gst-plugin-rtsp in release mode..."
    cd "$PROJECT_ROOT"
    cargo build --release -p gst-plugin-rtsp
    
    if [ $? -ne 0 ]; then
        log_error "Failed to build plugin"
        exit 1
    fi
    
    if [ ! -f "$PROJECT_ROOT/target/release/libgstrsrtsp.so" ]; then
        log_error "Plugin binary not found at target/release/libgstrsrtsp.so"
        exit 1
    fi
    
    log_success "Plugin built successfully"
    
    # Build Docker image with the plugin
    log_info "Building Docker image with pre-built plugin..."
    docker build -t "$IMAGE_NAME" -f "$SCRIPT_DIR/Dockerfile.test" "$PROJECT_ROOT"
    
    if [ $? -eq 0 ]; then
        log_success "Docker image built: $IMAGE_NAME"
    else
        log_error "Failed to build Docker image"
        exit 1
    fi
}

run_tests() {
    local test_args="$*"
    
    # Check if image exists
    if ! docker image inspect "$IMAGE_NAME" >/dev/null 2>&1; then
        log_info "Image not found, building..."
        build_image
    fi
    
    # Remove old container if exists
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
    
    log_info "Running tests in Docker container..."
    log_info "Test arguments: ${test_args:-suite:smoke}"
    
    # Run container with necessary capabilities for network operations
    docker run --rm \
        --name "$CONTAINER_NAME" \
        --cap-add=NET_ADMIN \
        --cap-add=NET_RAW \
        -v "$PROJECT_ROOT:/workspace" \
        -w /workspace/net/rtsp \
        "$IMAGE_NAME" \
        bash -c "cd /workspace/net/rtsp && ./scripts/run_tests_docker.sh ${test_args:-suite:smoke}"
    
    local exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        log_success "Tests completed successfully"
    else
        log_error "Tests failed with exit code: $exit_code"
    fi
    
    return $exit_code
}

run_shell() {
    # Check if image exists
    if ! docker image inspect "$IMAGE_NAME" >/dev/null 2>&1; then
        log_info "Image not found, building..."
        build_image
    fi
    
    log_info "Starting interactive shell in container..."
    
    docker run --rm -it \
        --name "$CONTAINER_NAME" \
        --cap-add=NET_ADMIN \
        --cap-add=NET_RAW \
        -v "$PROJECT_ROOT:/workspace" \
        -w /workspace/net/rtsp \
        "$IMAGE_NAME" \
        bash
}

clean() {
    log_info "Cleaning up Docker resources for version: $GIT_VERSION"
    
    # Stop and remove container
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
    
    # Remove image
    docker rmi "$IMAGE_NAME" 2>/dev/null || true
    
    log_success "Cleanup complete"
}

clean_all() {
    log_info "Cleaning up all gst-rtsp-test Docker images..."
    
    # Stop and remove all test containers
    docker ps -a | grep "gst-rtsp-test-runner" | awk '{print $1}' | xargs -r docker rm -f 2>/dev/null || true
    
    # Remove all test images
    docker images | grep "gst-rtsp-test" | awk '{print $1":"$2}' | xargs -r docker rmi 2>/dev/null || true
    
    log_success "Cleanup complete"
}

show_logs() {
    local results_dir="$SCRIPT_DIR/test-results"
    
    if [ ! -d "$results_dir" ]; then
        log_error "No test results found"
        exit 1
    fi
    
    log_info "Recent test results:"
    ls -lht "$results_dir" | head -20
    
    echo ""
    log_info "To view a specific log:"
    echo "  cat $results_dir/<log-file>"
}

show_version() {
    echo "Git version: $GIT_VERSION"
    echo "Image name: $IMAGE_NAME"
    echo "Container name: $CONTAINER_NAME"
    echo ""
    
    # Show if image exists
    if docker image inspect "$IMAGE_NAME" >/dev/null 2>&1; then
        echo "Image status: EXISTS"
        docker images "$IMAGE_NAME" --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}"
    else
        echo "Image status: NOT BUILT (run './docker-test.sh build' to create)"
    fi
    
    echo ""
    echo "All gst-rtsp-test images:"
    docker images | grep -E "REPOSITORY|gst-rtsp-test" || echo "  (none found)"
}

main() {
    local command=${1:-"help"}
    shift || true
    
    case "$command" in
        build)
            build_image
            ;;
        run)
            run_tests "$@"
            ;;
        shell)
            run_shell
            ;;
        clean)
            clean
            ;;
        clean-all)
            clean_all
            ;;
        logs)
            show_logs
            ;;
        version)
            show_version
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

main "$@"
