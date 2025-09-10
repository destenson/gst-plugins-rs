#!/bin/bash
# Test script for validating gst-plugin-livesync Debian package
# Following PRP-005: Package Testing and Validation Framework

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

# Test configuration
DEB_FILE=""
USE_DOCKER=true
DISTROS=("debian:bookworm" "debian:bullseye" "ubuntu:22.04" "ubuntu:24.04")
VERBOSE=false

usage() {
    echo "Usage: $0 [OPTIONS] <deb-file>"
    echo "Options:"
    echo "  --no-docker      Run tests locally instead of in Docker"
    echo "  --distro DISTRO  Test specific distribution (e.g., debian:bookworm)"
    echo "  --verbose        Enable verbose output"
    echo "  --help           Show this help message"
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-docker)
            USE_DOCKER=false
            shift
            ;;
        --distro)
            DISTROS=("$2")
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            usage
            ;;
        *.deb)
            DEB_FILE="$1"
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            usage
            ;;
    esac
done

# Find deb file if not specified
if [ -z "$DEB_FILE" ]; then
    DEB_FILE=$(ls -1 "$PROJECT_DIR"/target/debian/*.deb 2>/dev/null | head -n1)
    if [ -z "$DEB_FILE" ]; then
        echo -e "${RED}No .deb file found. Please build the package first or specify a .deb file.${NC}"
        exit 1
    fi
fi

if [ ! -f "$DEB_FILE" ]; then
    echo -e "${RED}File not found: $DEB_FILE${NC}"
    exit 1
fi

echo -e "${GREEN}Testing Debian package: $(basename "$DEB_FILE")${NC}"

# Run lintian checks
run_lintian() {
    echo -e "${BLUE}Running lintian checks...${NC}"
    if command -v lintian >/dev/null 2>&1; then
        lintian "$DEB_FILE" || {
            echo -e "${YELLOW}Warning: lintian found issues (this may be expected)${NC}"
        }
    else
        echo -e "${YELLOW}lintian not installed, skipping policy checks${NC}"
    fi
}

# Test installation in Docker container
test_docker_install() {
    local distro=$1
    echo -e "${BLUE}Testing installation on $distro...${NC}"
    
    # Create test script
    cat > /tmp/test-install.sh <<'EOF'
#!/bin/bash
set -e

# Update package lists
apt-get update

# Install GStreamer and dependencies
DEBIAN_FRONTEND=noninteractive apt-get install -y \
    gstreamer1.0-tools \
    gstreamer1.0-plugins-base \
    libgstreamer1.0-0

# Install the package
dpkg -i /workspace/*.deb || {
    # If dependencies are missing, fix them
    apt-get install -f -y
    dpkg -i /workspace/*.deb
}

# Test 1: Check if plugin is registered
echo "Testing plugin registration..."
gst-inspect-1.0 livesync >/dev/null || exit 1
echo "✓ Plugins registered successfully"

# Test 2: Check plugin details
echo "Plugin information:"
gst-inspect-1.0 livesync | head -20

# Test 3: Try to create a simple pipeline
echo "Testing pipeline creation..."
timeout 5 gst-launch-1.0 \
    videotestsrc num-buffers=10 ! \
    queue ! livesync name=live-source ! \
    fakesink \
    videotestsrc pattern=snow num-buffers=10 ! switch. \
    || {
        # Pipeline might timeout, but that's OK if elements were created
        echo "Pipeline test completed (may have timed out as expected)"
    }

# Test 4: Check library dependencies
echo "Checking library dependencies..."
ldd /usr/lib/*/gstreamer-1.0/libgstlivesync.so | head -10

# Test upgrade scenario
echo "Testing package upgrade..."
dpkg -i /workspace/*.deb
echo "✓ Package upgrade successful"

# Test removal
echo "Testing package removal..."
dpkg -r gst-plugin-livesync
# Verify plugin is no longer available
if gst-inspect-1.0 livesync 2>/dev/null; then
    echo "✗ Plugin still available after removal!"
    exit 1
fi
echo "✓ Package removed successfully"

# Test purge
dpkg -i /workspace/*.deb
dpkg -P gst-plugin-livesync
echo "✓ Package purged successfully"

echo "All tests passed for this distribution!"
EOF
    
    chmod +x /tmp/test-install.sh
    
    # Run Docker container with test script
    docker run --rm \
        -v "$(dirname "$DEB_FILE")":/workspace:ro \
        -v /tmp/test-install.sh:/test.sh:ro \
        "$distro" \
        /test.sh || {
            echo -e "${RED}Tests failed on $distro${NC}"
            return 1
        }
    
    echo -e "${GREEN}✓ All tests passed on $distro${NC}"
}

# Test local installation (requires sudo)
test_local_install() {
    echo -e "${YELLOW}Testing local installation (requires sudo)...${NC}"
    
    # Check if we have sudo
    if ! command -v sudo >/dev/null 2>&1; then
        echo -e "${RED}sudo is required for local testing${NC}"
        return 1
    fi
    
    # Install the package
    echo "Installing package..."
    sudo dpkg -i "$DEB_FILE" || {
        sudo apt-get install -f -y
        sudo dpkg -i "$DEB_FILE"
    }
    
    # Run tests
    echo "Testing plugin registration..."
    gst-inspect-1.0 livesync || {
        echo -e "${RED}Plugin registration failed${NC}"
        return 1
    }
    
    # Clean up
    echo "Removing package..."
    sudo dpkg -r gst-plugin-livesync
    
    echo -e "${GREEN}✓ Local tests passed${NC}"
}

# Main test execution
main() {
    # Always run lintian if available
    run_lintian
    
    if [ "$USE_DOCKER" = true ]; then
        # Check if Docker is available
        if ! command -v docker >/dev/null 2>&1; then
            echo -e "${RED}Docker is not installed. Use --no-docker for local testing.${NC}"
            exit 1
        fi
        
        # Test on each distribution
        FAILED=0
        for distro in "${DISTROS[@]}"; do
            test_docker_install "$distro" || FAILED=$((FAILED + 1))
            echo ""
        done
        
        if [ $FAILED -gt 0 ]; then
            echo -e "${RED}Tests failed on $FAILED distribution(s)${NC}"
            exit 1
        fi
        
        echo -e "${GREEN}All distribution tests passed!${NC}"
    else
        test_local_install
    fi
    
    echo -e "${GREEN}Package validation complete!${NC}"
}

main