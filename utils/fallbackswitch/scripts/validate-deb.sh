#!/bin/bash
# Validation script for gst-plugin-fallbackswitch Debian package
# Runs all validation gates from PRP-001

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

# 1. Build the plugin first
echo -e "${YELLOW}Step 1: Building the plugin...${NC}"
cargo build -p gst-plugin-fallbackswitch --release
check_result "Plugin built successfully"

# 2. Generate the deb package
echo -e "${YELLOW}Step 2: Generating Debian package...${NC}"
cd utils/fallbackswitch && cargo deb --no-build
check_result "Debian package generated"

# 3. Verify package was created
echo -e "${YELLOW}Step 3: Verifying package exists...${NC}"
ls target/debian/*.deb >/dev/null 2>&1
check_result "Package file exists"

# 4. Check package info
echo -e "${YELLOW}Step 4: Checking package metadata...${NC}"
dpkg-deb --info target/debian/*.deb >/dev/null 2>&1
check_result "Package metadata is valid"

# 5. Verify package contents
echo -e "${YELLOW}Step 5: Verifying package contents...${NC}"
dpkg-deb --contents target/debian/*.deb | grep -q "libgstfallbackswitch.so"
check_result "Plugin library is included in package"

# 6. Check for documentation files
echo -e "${YELLOW}Step 6: Checking documentation...${NC}"
dpkg-deb --contents target/debian/*.deb | grep -q "README.md"
check_result "README.md is included"
dpkg-deb --contents target/debian/*.deb | grep -q "LICENSE-MPL-2.0"
check_result "License file is included"

# 7. Run lintian if available (optional but recommended)
if command -v lintian >/dev/null 2>&1; then
    echo -e "${YELLOW}Step 7: Running lintian checks...${NC}"
    lintian target/debian/*.deb || true
    echo -e "${YELLOW}Note: Review lintian warnings/errors above${NC}"
else
    echo -e "${YELLOW}Step 7: Skipping lintian (not installed)${NC}"
fi

# 8. Display package summary
echo -e "${GREEN}==== Package Summary ====${NC}"
dpkg-deb --info target/debian/*.deb | grep -E "Package:|Version:|Architecture:|Depends:"

echo -e "${GREEN}All validation checks passed!${NC}"
echo -e "${GREEN}Package ready for installation and distribution${NC}"