#!/bin/bash
# build-deb.sh - Automated Debian Package Generation Script for gst-plugin-rtsp

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RTSP_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$(dirname "$RTSP_DIR")")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
PACKAGE_NAME="gst-plugin-rtsp"
ARCH="${1:-$(dpkg --print-architecture 2>/dev/null || echo 'amd64')}"
VERBOSE="${VERBOSE:-0}"

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Cleanup function
cleanup() {
    log_info "[SKIP] Cleaning up build artifacts..."
    # cd "$RTSP_DIR"
    # cargo clean 2>/dev/null || true
}

# Trap cleanup on exit
trap cleanup EXIT

# Check dependencies
check_dependencies() {
    log_info "Checking build dependencies..."
    
    local missing_deps=()
    
    # Check for required tools
    command -v cargo >/dev/null 2>&1 || missing_deps+=("cargo (Rust toolchain)")
    command -v cargo-deb >/dev/null 2>&1 || missing_deps+=("cargo-deb")
    command -v pkg-config >/dev/null 2>&1 || missing_deps+=("pkg-config")
    
    # Check for GStreamer development files
    if ! pkg-config --exists gstreamer-1.0; then
        missing_deps+=("libgstreamer1.0-dev")
    fi
    
    if ! pkg-config --exists gstreamer-net-1.0; then
        missing_deps+=("libgstreamer-plugins-base1.0-dev")
    fi
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        log_error "Missing dependencies:"
        for dep in "${missing_deps[@]}"; do
            echo "  - $dep"
        done
        log_info "Install missing dependencies:"
        echo "  sudo apt-get update"
        echo "  sudo apt-get install -y build-essential pkg-config libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev"
        echo "  cargo install cargo-deb"
        exit 1
    fi
    
    log_info "All dependencies satisfied"
}

# Architecture validation
validate_architecture() {
    log_info "Validating target architecture: $ARCH"
    
    case "$ARCH" in
        amd64|arm64|armhf|i386)
            log_info "Architecture $ARCH is supported"
            ;;
        *)
            log_warn "Architecture $ARCH may not be fully tested"
            ;;
    esac
}

# Build the plugin
build_plugin() {
    log_info "Building gst-plugin-rtsp (release mode)..."
    cd "$RTSP_DIR"
    
    # Set build flags for optimization
    export CARGO_PROFILE_RELEASE_LTO=true
    export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
    
    if [ "$VERBOSE" = "1" ]; then
        cargo build --release --verbose
    else
        cargo build --release
    fi
    
    # Verify library was built
    local lib_file
    if [ "$(uname)" = "Linux" ]; then
        lib_file="${PROJECT_ROOT}/target/release/libgstrsrtsp.so"
    else
        lib_file="${PROJECT_ROOT}/target/release/gstrsrtsp.dll"
    fi
    
    if [ ! -f "$lib_file" ]; then
        log_error "Library file not found: $lib_file"
        exit 1
    fi
    
    log_info "Plugin built successfully: $lib_file"
}

# Generate Debian package
generate_package() {
    log_info "Generating Debian package..."
    cd "$RTSP_DIR"
    
    if [ "$VERBOSE" = "1" ]; then
        cargo deb --verbose --no-strip --no-build --multiarch=same
    else
        cargo deb --no-strip --no-build --multiarch=same
    fi
    
    # Find the generated package
    local deb_file
    deb_file=$(find ${PROJECT_ROOT}/target/debian -name "*.deb" | head -n 1)
    
    if [ -z "$deb_file" ]; then
        log_error "No .deb file found in ${PROJECT_ROOT}/target/debian/"
        exit 1
    fi
    
    log_info "Package generated: $deb_file"
    
    # Basic package validation
    validate_package "$deb_file"
}

# Validate the generated package
validate_package() {
    local deb_file="$1"
    log_info "Validating package: $deb_file"
    
    # Check package contents
    log_info "Package contents:"
    dpkg-deb -c "$deb_file" | head -20
    
    # Check package info
    log_info "Package information:"
    dpkg-deb --info "$deb_file"
    
    # Run lintian if available
    if command -v lintian >/dev/null 2>&1; then
        log_info "Running lintian checks..."
        lintian "$deb_file" || log_warn "Lintian found issues (may be non-critical)"
    else
        log_warn "lintian not available, skipping package policy checks"
    fi
}

# Main execution
main() {
    log_info "Starting Debian package build for $PACKAGE_NAME"
    log_info "Target architecture: $ARCH"
    log_info "Working directory: $RTSP_DIR"
    
    check_dependencies
    validate_architecture
    build_plugin
    generate_package
    
    log_info "Build completed successfully!"
    
    # Show final package location
    local final_deb
    final_deb=$(find "${PROJECT_ROOT}/target/debian" -name "*.deb" | head -n 1)
    if [ -n "$final_deb" ]; then
        log_info "Final package: $final_deb"
        log_info "Package size: $(du -h "$final_deb" | cut -f1)"
    fi
}

# Help text
show_help() {
    cat <<EOF
Usage: $0 [ARCHITECTURE]

Build Debian package for gst-plugin-rtsp

Arguments:
    ARCHITECTURE    Target architecture (amd64, arm64, armhf, i386)
                   Default: auto-detected or amd64

Environment Variables:
    VERBOSE=1       Enable verbose output

Examples:
    $0                  # Build for current architecture
    $0 arm64           # Cross-build for ARM64
    VERBOSE=1 $0       # Build with verbose output

EOF
}

# Parse command line arguments
case "${1:-}" in
    -h|--help|help)
        show_help
        exit 0
        ;;
    *)
        main "$@"
        ;;
esac
