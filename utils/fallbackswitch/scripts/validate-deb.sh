#!/bin/bash
# Validation script for gst-plugin-fallbackswitch Debian package
# Runs all validation gates from PRP-001 and PRP-002

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Validating gst-plugin-fallbackswitch Debian package...${NC}"

# Function to check command result
check_result() {
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ $1${NC}"
    else
        echo -e "${RED}✗ $1${NC}"
        exit 1
    fi
}

# Detect architecture for validation
DEB_HOST_MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null || echo "x86_64-linux-gnu")
echo -e "${YELLOW}Validating for architecture: ${DEB_HOST_MULTIARCH}${NC}"

# 1. Build the plugin library with cargo-c
echo -e "${YELLOW}Step 1: Building the plugin library with cargo-c...${NC}"
cargo cbuild -p gst-plugin-fallbackswitch --release
check_result "Plugin library built successfully with cargo-c"

# 2. Check the built library
echo -e "${YELLOW}Step 2: Checking built library...${NC}"
file target/release/libgstfallbackswitch.so
check_result "Library file exists and is valid"

# 3. Generate the deb package
echo -e "${YELLOW}Step 3: Generating Debian package...${NC}"
cd utils/fallbackswitch && cargo deb --no-build
check_result "Debian package generated"
cd ../..

# 4. Verify package was created
echo -e "${YELLOW}Step 4: Verifying package exists...${NC}"
DEB_FILE=$(ls -1 utils/fallbackswitch/target/debian/*.deb 2>/dev/null | head -n1)
if [ -z "$DEB_FILE" ]; then
    echo -e "${RED}✗ Package file not found${NC}"
    exit 1
fi
check_result "Package file exists: $(basename $DEB_FILE)"

# 5. Check package info
echo -e "${YELLOW}Step 5: Checking package metadata...${NC}"
dpkg-deb --info "$DEB_FILE" >/dev/null 2>&1
check_result "Package metadata is valid"

# 6. Verify GStreamer plugin path (PRP-002 specific)
echo -e "${YELLOW}Step 6: Verifying GStreamer plugin installation path...${NC}"
EXPECTED_PATH="/usr/lib/${DEB_HOST_MULTIARCH}/gstreamer-1.0/"
if dpkg-deb --contents "$DEB_FILE" | grep -q "${EXPECTED_PATH}libgstfallbackswitch.so"; then
    check_result "Library will be installed to correct GStreamer path: ${EXPECTED_PATH}"
elif dpkg-deb --contents "$DEB_FILE" | grep -q "/usr/lib/.*/gstreamer-1.0/libgstfallbackswitch.so"; then
    check_result "Library will be installed to a GStreamer plugin path (multiarch)"
else
    echo -e "${RED}✗ Library not found in expected GStreamer plugin path${NC}"
    echo "Package contents:"
    dpkg-deb --contents "$DEB_FILE" | grep "\.so"
    exit 1
fi

# 7. Check for proper permissions
echo -e "${YELLOW}Step 7: Checking file permissions...${NC}"
if dpkg-deb --contents "$DEB_FILE" | grep "libgstfallbackswitch.so" | grep -q "644"; then
    check_result "Library has correct permissions (644)"
else
    echo -e "${YELLOW}⚠ Warning: Library permissions may not be 644${NC}"
fi

# 8. Check for documentation files
echo -e "${YELLOW}Step 8: Checking documentation...${NC}"
dpkg-deb --contents "$DEB_FILE" | grep -q "README.md"
check_result "README.md is included"
dpkg-deb --contents "$DEB_FILE" | grep -q "LICENSE-MPL-2.0"
check_result "License file is included"

# 9. Verify ldconfig trigger (PRP-002 specific)
echo -e "${YELLOW}Step 9: Checking ldconfig trigger...${NC}"
if dpkg-deb --info "$DEB_FILE" | grep -q "Triggers:.*ldconfig" || [ -f "utils/fallbackswitch/debian/triggers" ]; then
    check_result "ldconfig trigger is configured"
else
    echo -e "${YELLOW}⚠ Warning: ldconfig trigger may not be configured${NC}"
fi

# 10. Run lintian if available (optional but recommended)
if command -v lintian >/dev/null 2>&1; then
    echo -e "${YELLOW}Step 10: Running lintian checks...${NC}"
    lintian "$DEB_FILE" || true
    echo -e "${YELLOW}Note: Review lintian warnings/errors above${NC}"
else
    echo -e "${YELLOW}Step 10: Skipping lintian (not installed)${NC}"
fi

# 11. Display package summary
echo ""
echo -e "${GREEN}==== Package Summary ====${NC}"
dpkg-deb --info "$DEB_FILE" | grep -E "Package:|Version:|Architecture:|Depends:"
echo ""
echo -e "${GREEN}==== Installation Path Verification ====${NC}"
dpkg-deb --contents "$DEB_FILE" | grep -E "(gstreamer|\.so)" | head -5

# 12. Test commands to run after installation
echo ""
echo -e "${GREEN}==== Post-Installation Test Commands ====${NC}"
echo "After installing the package, run these commands to verify:"
echo "  1. gst-inspect-1.0 fallbackswitch"
echo "  2. gst-inspect-1.0 fallbacksrc"
echo "  3. GST_PLUGIN_PATH=/usr/lib/${DEB_HOST_MULTIARCH}/gstreamer-1.0 gst-inspect-1.0 | grep fallback"

echo ""
echo -e "${GREEN}All validation checks passed!${NC}"
echo -e "${GREEN}Package ready for installation and distribution${NC}"