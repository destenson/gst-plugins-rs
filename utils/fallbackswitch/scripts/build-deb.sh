#!/bin/bash
# Build script for creating Debian package with cargo-deb
# This script ensures proper build configuration for debian packaging
# Following PRP-002: GStreamer Plugin Installation Path Configuration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building gst-plugin-fallbackswitch Debian package...${NC}"

# Check for required tools
command -v cargo >/dev/null 2>&1 || { echo -e "${RED}cargo is required but not installed.${NC}" >&2; exit 1; }
command -v cargo-c >/dev/null 2>&1 || { echo -e "${RED}cargo-c is required but not installed. Install with: cargo install cargo-c${NC}" >&2; exit 1; }
command -v cargo-deb >/dev/null 2>&1 || { echo -e "${RED}cargo-deb is required but not installed. Install with: cargo install cargo-deb${NC}" >&2; exit 1; }

# Detect architecture and set multiarch path
ARCH=$(dpkg --print-architecture)
DEB_HOST_MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
echo -e "${YELLOW}Building for architecture: ${ARCH}${NC}"
echo -e "${YELLOW}Multiarch path: ${DEB_HOST_MULTIARCH}${NC}"

# Export for cargo-deb to use
export DEB_HOST_MULTIARCH

# Clean previous builds
echo "Cleaning previous builds..."
cargo clean

# Build the plugin library with cargo-c in release mode
echo "Building plugin library with cargo-c..."
cargo cbuild -p gst-plugin-fallbackswitch --release

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
cargo deb --no-build --no-strip

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
    if dpkg-deb --contents "$DEB_FILE" | grep -q "/usr/lib/${DEB_HOST_MULTIARCH}/gstreamer-1.0/"; then
        echo -e "${GREEN}✓ Library will be installed to correct GStreamer plugin path${NC}"
    else
        echo -e "${YELLOW}⚠ Warning: Library path may not be correct for GStreamer discovery${NC}"
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