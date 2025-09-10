#!/bin/bash
# Build script for creating Debian package with cargo-deb
# This script ensures proper build configuration for debian packaging
# Following PRP-002 and PRP-004: Automated Build Script

set -e

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse command line arguments
TARGET_ARCH=""
VERBOSE=false
CLEAN_BUILD=false

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo "Options:"
    echo "  --arch ARCH      Target architecture (amd64, arm64, armhf)"
    echo "  --verbose        Enable verbose output"
    echo "  --clean          Clean build artifacts before building"
    echo "  --help           Show this help message"
    exit 0
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --arch)
            TARGET_ARCH="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --clean)
            CLEAN_BUILD=true
            shift
            ;;
        --help)
            usage
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            usage
            ;;
    esac
done

echo -e "${GREEN}Building gst-plugin-fallbackswitch Debian package...${NC}"

# Check for required tools
check_tool() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo -e "${RED}$1 is required but not installed.${NC}"
        if [ -n "$2" ]; then
            echo -e "${YELLOW}Install with: $2${NC}"
        fi
        exit 1
    fi
}

check_tool cargo
check_tool cargo-deb "cargo install cargo-deb"
check_tool dpkg
check_tool dpkg-architecture

# Detect architecture and set multiarch path
if [ -z "$TARGET_ARCH" ]; then
    ARCH=$(dpkg --print-architecture)
else
    ARCH="$TARGET_ARCH"
fi

# Map architecture to Rust target
case $ARCH in
    amd64)
        RUST_TARGET="x86_64-unknown-linux-gnu"
        ;;
    arm64|aarch64)
        RUST_TARGET="aarch64-unknown-linux-gnu"
        ;;
    armhf)
        RUST_TARGET="armv7-unknown-linux-gnueabihf"
        ;;
    *)
        echo -e "${RED}Unsupported architecture: $ARCH${NC}"
        echo "Supported: amd64, arm64, armhf"
        exit 1
        ;;
esac

DEB_HOST_MULTIARCH=$(dpkg-architecture -a"$ARCH" -qDEB_HOST_MULTIARCH)
echo -e "${BLUE}Building for architecture: ${ARCH}${NC}"
echo -e "${BLUE}Rust target: ${RUST_TARGET}${NC}"
echo -e "${BLUE}Multiarch path: ${DEB_HOST_MULTIARCH}${NC}"

# Export for cargo-deb to use
export DEB_HOST_MULTIARCH
export CARGO_TARGET_DIR="$PROJECT_DIR/target"

# Change to project directory
cd "$PROJECT_DIR"

# Clean previous builds if requested
if [ "$CLEAN_BUILD" = true ]; then
    echo -e "${YELLOW}Cleaning previous builds...${NC}"
    cargo clean
fi

# Check if cross-compilation is needed
if [ "$ARCH" != "$(dpkg --print-architecture)" ]; then
    echo -e "${YELLOW}Cross-compiling for $ARCH${NC}"
    # Ensure the target is installed
    rustup target add "$RUST_TARGET" 2>/dev/null || true
    BUILD_FLAGS="--target $RUST_TARGET"
else
    BUILD_FLAGS=""
fi

# Build the plugin library with cargo in release mode
echo -e "${BLUE}Building plugin library...${NC}"
if [ "$VERBOSE" = true ]; then
    cargo build -p gst-plugin-fallbackswitch --release $BUILD_FLAGS --verbose
else
    cargo build -p gst-plugin-fallbackswitch --release $BUILD_FLAGS
fi

# Verify the library was built
if [ ! -f "target/release/libgstfallbackswitch.so" ]; then
    echo -e "${RED}Failed to build library${NC}"
    exit 1
fi

# Display library info
echo "Library information:"
file target/release/libgstfallbackswitch.so
ldd target/release/libgstfallbackswitch.so || true

# Strip the binary to reduce size (cargo-deb will do this too if strip=true)
echo "Stripping binary for release..."
strip --strip-unneeded target/release/libgstfallbackswitch.so

# Generate the debian package
echo "Generating Debian package..."
cargo deb --no-build --no-strip --multiarch=same #--output "usr/lib/${DEB_HOST_MULTIARCH}/gstreamer-1.0"

# Display package info
DEB_FILE=$(ls -1 target/debian/*.deb 2>/dev/null | head -n1)
if [ -n "$DEB_FILE" ]; then
    echo -e "${GREEN}Package created successfully!${NC}"
    echo "Package: $DEB_FILE"
    echo ""
    echo "Package details:"
    dpkg-deb --info "$DEB_FILE"
    echo ""
    echo "Package contents (checking GStreamer plugin path):"
    dpkg-deb --contents "$DEB_FILE" | grep -E "(gstreamer|\.so)"
    echo ""
    # Verify the library is in the correct multiarch path
    if dpkg-deb --contents "$DEB_FILE" | grep -q "usr/lib/${DEB_HOST_MULTIARCH}/gstreamer-1.0"; then
        echo -e "${GREEN}✓ Library will be installed to correct GStreamer plugin path${NC}"
    else
        echo -e "${YELLOW}⚠ Warning: Library path may not be correct for GStreamer discovery${NC}: usr/lib/${DEB_HOST_MULTIARCH}/gstreamer-1.0"
    fi
else
    echo -e "${RED}Failed to create debian package${NC}"
    exit 1
fi

echo -e "${GREEN}Build complete! Package available in target/debian/${NC}"
echo ""
echo "To test installation (in a container or VM):"
echo "  sudo dpkg -i $DEB_FILE"
echo "  gst-inspect-1.0 fallbackswitch"