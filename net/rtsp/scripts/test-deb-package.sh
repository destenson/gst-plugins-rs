#!/bin/bash
# test-deb-package.sh - Debian Package Testing and Validation Framework
# Following PRP-005: Package Testing Framework

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RTSP_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
PACKAGE_NAME="gst-plugin-rtsp"
TEST_DISTROS=(
    "debian:bookworm"
    "ubuntu:22.04"
    "ubuntu:24.04"
)

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

log_test() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

# Find the package file
find_package() {
    local deb_file
    deb_file=$(find "$RTSP_DIR/target/debian" -name "*.deb" | head -n 1)
    
    if [ -z "$deb_file" ]; then
        log_error "No .deb package found. Run build-deb.sh first."
        exit 1
    fi
    
    echo "$deb_file"
}

# Run lintian checks
test_lintian() {
    local deb_file="$1"
    log_test "Running lintian policy checks..."
    
    if ! command -v lintian >/dev/null 2>&1; then
        log_warn "lintian not available, installing..."
        sudo apt-get update >/dev/null 2>&1
        sudo apt-get install -y lintian >/dev/null 2>&1
    fi
    
    log_info "Checking package compliance..."
    if lintian --no-tag-display-limit "$deb_file"; then
        log_info "✓ Package passes lintian checks"
        return 0
    else
        log_warn "△ Package has lintian issues (may be non-critical)"
        return 1
    fi
}

# Test package metadata
test_package_metadata() {
    local deb_file="$1"
    log_test "Validating package metadata..."
    
    # Check package info
    local pkg_info
    pkg_info=$(dpkg-deb --info "$deb_file")
    
    # Verify required fields
    if echo "$pkg_info" | grep -q "Package: $PACKAGE_NAME"; then
        log_info "✓ Package name correct"
    else
        log_error "✗ Package name incorrect"
        return 1
    fi
    
    if echo "$pkg_info" | grep -q "Maintainer:.*gstreamer-rust"; then
        log_info "✓ Maintainer field present"
    else
        log_error "✗ Maintainer field missing or incorrect"
        return 1
    fi
    
    if echo "$pkg_info" | grep -q "Description:.*RTSP"; then
        log_info "✓ Description contains RTSP"
    else
        log_error "✗ Description missing RTSP reference"
        return 1
    fi
    
    log_info "✓ Package metadata validated"
    return 0
}

# Test package contents
test_package_contents() {
    local deb_file="$1"
    log_test "Checking package contents..."
    
    local contents
    contents=$(dpkg-deb -c "$deb_file")
    
    # Check for library file
    if echo "$contents" | grep -q "usr/lib.*gstreamer-1.0.*libgstrsrtsp"; then
        log_info "✓ Library file present in correct location"
    else
        log_error "✗ Library file missing or in wrong location"
        echo "$contents" | grep "libgstrsrtsp" || echo "No libgstrsrtsp found"
        return 1
    fi
    
    # Check for documentation
    if echo "$contents" | grep -q "usr/share/doc/$PACKAGE_NAME"; then
        log_info "✓ Documentation directory present"
    else
        log_error "✗ Documentation directory missing"
        return 1
    fi
    
    log_info "✓ Package contents validated"
    return 0
}

# Test installation in Docker container
test_installation() {
    local deb_file="$1"
    local distro="$2"
    
    log_test "Testing installation on $distro..."
    
    # Ensure Docker is available
    if ! command -v docker >/dev/null 2>&1; then
        log_warn "Docker not available, skipping container tests"
        return 0
    fi
    
    # Copy package to temporary location for Docker
    local temp_deb="/tmp/$(basename "$deb_file")"
    cp "$deb_file" "$temp_deb"
    
    # Create test script
    local test_script="/tmp/test-install-$PACKAGE_NAME.sh"
    cat > "$test_script" <<'EOF'
#!/bin/bash
set -e

# Update package cache
apt-get update >/dev/null 2>&1

# Install GStreamer tools for testing
apt-get install -y gstreamer1.0-tools >/dev/null 2>&1

# Install the package
echo "Installing package..."
dpkg -i /tmp/*.deb || apt-get install -f -y

# Verify installation
echo "Verifying installation..."

# Check if plugin is loadable
if gst-inspect-1.0 rtspsrc >/dev/null 2>&1; then
    echo "✓ rtspsrc element found"
else
    echo "✗ rtspsrc element not found"
    exit 1
fi

# Check plugin registry
if gst-inspect-1.0 | grep -q rsrtsp; then
    echo "✓ Plugin appears in registry" 
else
    echo "✗ Plugin not in registry"
    exit 1
fi

echo "✓ Installation test passed"
EOF
    
    chmod +x "$test_script"
    
    # Run test in container
    if docker run --rm \
        -v "$temp_deb:/tmp/$(basename "$deb_file")" \
        -v "$test_script:/test.sh" \
        "$distro" \
        /test.sh; then
        log_info "✓ Installation test passed on $distro"
        local result=0
    else
        log_error "✗ Installation test failed on $distro"
        local result=1
    fi
    
    # Cleanup
    rm -f "$temp_deb" "$test_script"
    
    return $result
}

# Test removal scenarios
test_removal() {
    local deb_file="$1"
    local distro="$2"
    
    log_test "Testing package removal on $distro..."
    
    if ! command -v docker >/dev/null 2>&1; then
        log_warn "Docker not available, skipping removal tests"
        return 0
    fi
    
    local temp_deb="/tmp/$(basename "$deb_file")"
    cp "$deb_file" "$temp_deb"
    
    # Create removal test script
    local test_script="/tmp/test-removal-$PACKAGE_NAME.sh"
    cat > "$test_script" <<'EOF'
#!/bin/bash
set -e

# Install prerequisites
apt-get update >/dev/null 2>&1
apt-get install -y gstreamer1.0-tools >/dev/null 2>&1

# Install package
dpkg -i /tmp/*.deb || apt-get install -f -y

# Verify installation
gst-inspect-1.0 rtspsrc >/dev/null 2>&1 || exit 1

# Remove package
dpkg -r gst-plugin-rtsp

# Verify removal
if gst-inspect-1.0 rtspsrc >/dev/null 2>&1; then
    echo "✗ Plugin still available after removal"
    exit 1
fi

# Check for leftover files
if find /usr -name "*rsrtsp*" 2>/dev/null | grep -v "/var/lib/dpkg"; then
    echo "✗ Leftover files found after removal"
    exit 1
fi

echo "✓ Removal test passed"
EOF
    
    chmod +x "$test_script"
    
    if docker run --rm \
        -v "$temp_deb:/tmp/$(basename "$deb_file")" \
        -v "$test_script:/test.sh" \
        "$distro" \
        /test.sh; then
        log_info "✓ Removal test passed on $distro"
        local result=0
    else
        log_error "✗ Removal test failed on $distro"
        local result=1
    fi
    
    rm -f "$temp_deb" "$test_script"
    return $result
}

# Generate test report
generate_report() {
    local results="$1"
    local report_file="$RTSP_DIR/test-results.txt"
    
    log_info "Generating test report: $report_file"
    
    {
        echo "Debian Package Test Report for $PACKAGE_NAME"
        echo "Generated: $(date)"
        echo "========================================="
        echo ""
        echo "$results"
        echo ""
        echo "Test completed at $(date)"
    } > "$report_file"
    
    log_info "Test report saved to: $report_file"
}

# Main test runner
run_tests() {
    local deb_file
    deb_file=$(find_package)
    
    log_info "Running comprehensive package tests"
    log_info "Package: $deb_file"
    log_info "Package size: $(du -h "$deb_file" | cut -f1)"
    
    local results=""
    local overall_result=0
    
    # Basic validation tests
    results+="=== BASIC VALIDATION ===\n"
    
    if test_package_metadata "$deb_file"; then
        results+="Package metadata: PASS\n"
    else
        results+="Package metadata: FAIL\n"
        overall_result=1
    fi
    
    if test_package_contents "$deb_file"; then
        results+="Package contents: PASS\n"
    else
        results+="Package contents: FAIL\n"
        overall_result=1
    fi
    
    if test_lintian "$deb_file"; then
        results+="Lintian checks: PASS\n"
    else
        results+="Lintian checks: WARN\n"
    fi
    
    # Docker-based tests (if available)
    if command -v docker >/dev/null 2>&1; then
        results+="\n=== INSTALLATION TESTS ===\n"
        
        for distro in "${TEST_DISTROS[@]}"; do
            if test_installation "$deb_file" "$distro"; then
                results+="$distro installation: PASS\n"
            else
                results+="$distro installation: FAIL\n"
                overall_result=1
            fi
            
            if test_removal "$deb_file" "$distro"; then
                results+="$distro removal: PASS\n"
            else
                results+="$distro removal: FAIL\n"
                overall_result=1
            fi
        done
    else
        results+="\n=== INSTALLATION TESTS ===\n"
        results+="Skipped (Docker not available)\n"
    fi
    
    # Generate report
    generate_report "$results"
    
    if [ $overall_result -eq 0 ]; then
        log_info "🎉 All tests passed!"
        return 0
    else
        log_error "❌ Some tests failed!"
        return 1
    fi
}

# Help text
show_help() {
    cat <<EOF
Usage: $0 [OPTIONS]

Test and validate Debian package for gst-plugin-rtsp

Options:
    --quick         Run only basic validation tests (skip Docker)
    --help, -h      Show this help message

Examples:
    $0                  # Run all tests
    $0 --quick         # Run basic tests only

Requirements:
    - Package must be built first (run build-deb.sh)
    - Docker (optional, for distribution testing)
    - lintian (will be installed if missing)

EOF
}

# Parse arguments
QUICK_MODE=0

while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            QUICK_MODE=1
            shift
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Disable Docker tests in quick mode
if [ $QUICK_MODE -eq 1 ]; then
    log_info "Quick mode enabled - skipping Docker tests"
    TEST_DISTROS=()
fi

# Run the tests
run_tests