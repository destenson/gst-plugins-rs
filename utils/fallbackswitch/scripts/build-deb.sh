#!/bin/bash
# Build script for creating Debian package with cargo-deb
# This script ensures proper build configuration for debian packaging

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building gst-plugin-fallbackswitch Debian package...${NC}"

# Check for required tools
command -v cargo >/dev/null 2>&1 || { echo -e "${RED}cargo is required but not installed.${NC}" >&2; exit 1; }
command -v cargo-deb >/dev/null 2>&1 || { echo -e "${RED}cargo-deb is required but not installed. Install with: cargo install cargo-deb${NC}" >&2; exit 1; }

# Detect architecture
ARCH=$(dpkg --print-architecture)
echo -e "${YELLOW}Building for architecture: ${ARCH}${NC}"

# Clean previous builds
echo "Cleaning previous builds..."
cargo clean

# Build the plugin in release mode
echo "Building plugin in release mode..."
cargo build --release --features v1_20

# Strip the binary to reduce size
echo "Stripping binary..."
strip target/release/libgstfallbackswitch.so

# Generate the debian package
echo "Generating Debian package..."
cargo deb --no-build

# Display package info
if [ -f target/debian/*.deb ]; then
    echo -e "${GREEN}Package created successfully!${NC}"
    echo "Package details:"
    dpkg-deb --info target/debian/*.deb
    echo ""
    echo "Package contents:"
    dpkg-deb --contents target/debian/*.deb
else
    echo -e "${RED}Failed to create debian package${NC}"
    exit 1
fi

echo -e "${GREEN}Build complete! Package available in target/debian/${NC}"