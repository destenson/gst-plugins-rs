#!/bin/bash
set -e

# Test script for gst-plugin-uriplaylistbin Debian package
# This script validates the package installation and functionality

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PACKAGE_NAME="gst-plugin-uriplaylistbin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_test_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
}

print_test_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
}

# Test package installation
test_installation() {
    print_info "Testing package installation..."
    
    # Find the .deb file
    DEB_FILE=$(find "$PROJECT_DIR" -maxdepth 1 -name "*.deb" -type f | head -n 1)
    if [ -z "$DEB_FILE" ]; then
        DEB_FILE=$(find "$PROJECT_DIR/target/debian" -name "*.deb" -type f | head -n 1)
    fi
    
    if [ -z "$DEB_FILE" ]; then
        print_error "No .deb file found. Please run build-deb.sh first."
        return 1
    fi
    
    # Check if package is already installed
    if dpkg -l | grep -q "$PACKAGE_NAME"; then
        print_warning "Package is already installed. Testing existing installation."
        print_test_pass "Package installation check"
    else
        print_info "Package not installed. Please install with:"
        print_info "  sudo dpkg -i $DEB_FILE"
        print_info "Or:"
        print_info "  sudo apt install $DEB_FILE"
        print_test_fail "Package not installed"
        return 1
    fi
}

# Test plugin file existence
test_plugin_file() {
    print_info "Testing plugin file existence..."
    
    # Check for the plugin in standard locations
    PLUGIN_PATHS=(
        "/usr/lib/gstreamer-1.0/libgsturiplaylistbin.so"
        "/usr/lib/x86_64-linux-gnu/gstreamer-1.0/libgsturiplaylistbin.so"
        "/usr/lib/aarch64-linux-gnu/gstreamer-1.0/libgsturiplaylistbin.so"
        "/usr/lib/arm-linux-gnueabihf/gstreamer-1.0/libgsturiplaylistbin.so"
    )
    
    FOUND=0
    for path in "${PLUGIN_PATHS[@]}"; do
        if [ -f "$path" ]; then
            print_test_pass "Plugin file found at: $path"
            FOUND=1
            
            # Check file permissions
            if [ -r "$path" ]; then
                print_test_pass "Plugin file is readable"
            else
                print_test_fail "Plugin file is not readable"
            fi
            break
        fi
    done
    
    if [ $FOUND -eq 0 ]; then
        print_test_fail "Plugin file not found in any standard location"
        return 1
    fi
}

# Test GStreamer plugin registration
test_gst_registration() {
    print_info "Testing GStreamer plugin registration..."
    
    # Test if gst-inspect-1.0 can find the plugin
    if gst-inspect-1.0 uriplaylistbin &> /dev/null; then
        print_test_pass "GStreamer can find uriplaylistbin element"
        
        # Get detailed information
        print_info "Element details:"
        gst-inspect-1.0 uriplaylistbin | grep -E "^(Factory Details|Plugin Details):" -A 3
    else
        print_test_fail "GStreamer cannot find uriplaylistbin element"
        print_info "Trying to update plugin registry..."
        gst-inspect-1.0 --gst-disable-registry-fork &> /dev/null || true
        
        # Retry after registry update
        if gst-inspect-1.0 uriplaylistbin &> /dev/null; then
            print_test_pass "Element found after registry update"
        else
            print_test_fail "Element still not found after registry update"
            return 1
        fi
    fi
}

# Test basic pipeline functionality
test_pipeline() {
    print_info "Testing basic pipeline functionality..."
    
    # Create test files if they don't exist
    TEST_FILE1="/tmp/test1.txt"
    TEST_FILE2="/tmp/test2.txt"
    echo "Test content 1" > "$TEST_FILE1"
    echo "Test content 2" > "$TEST_FILE2"
    
    # Test with fakesink (no actual playback)
    PIPELINE="uriplaylistbin uris=\"file://$TEST_FILE1,file://$TEST_FILE2\" iterations=1 ! fakesink"
    
    print_info "Testing pipeline: $PIPELINE"
    
    # Run pipeline with timeout
    if timeout 5 gst-launch-1.0 $PIPELINE 2>&1 | grep -q "ERROR"; then
        print_test_fail "Pipeline execution failed"
        return 1
    else
        print_test_pass "Basic pipeline execution successful"
    fi
    
    # Clean up test files
    rm -f "$TEST_FILE1" "$TEST_FILE2"
}

# Test package dependencies
test_dependencies() {
    print_info "Testing package dependencies..."
    
    # Check if all required dependencies are satisfied
    if dpkg -s "$PACKAGE_NAME" 2>/dev/null | grep -q "Status: install ok installed"; then
        DEPS=$(dpkg -s "$PACKAGE_NAME" | grep "^Depends:" | cut -d: -f2-)
        print_info "Package dependencies: $DEPS"
        
        # Check if GStreamer base packages are installed
        if dpkg -l | grep -q "libgstreamer1.0-0"; then
            print_test_pass "GStreamer core library is installed"
        else
            print_test_fail "GStreamer core library is missing"
        fi
        
        if dpkg -l | grep -q "gstreamer1.0-plugins-base"; then
            print_test_pass "GStreamer base plugins are installed"
        else
            print_test_fail "GStreamer base plugins are missing"
        fi
    else
        print_test_fail "Cannot check dependencies - package not properly installed"
    fi
}

# Test documentation
test_documentation() {
    print_info "Testing documentation installation..."
    
    DOC_DIR="/usr/share/doc/$PACKAGE_NAME"
    
    if [ -d "$DOC_DIR" ]; then
        print_test_pass "Documentation directory exists: $DOC_DIR"
        
        # Check for specific files
        if [ -f "$DOC_DIR/README.md" ] || [ -f "$DOC_DIR/README.md.gz" ]; then
            print_test_pass "README file installed"
        else
            print_test_fail "README file not found"
        fi
        
        if [ -f "$DOC_DIR/LICENSE-MPL-2.0" ] || [ -f "$DOC_DIR/LICENSE-MPL-2.0.gz" ]; then
            print_test_pass "License file installed"
        else
            print_test_fail "License file not found"
        fi
        
        if [ -f "$DOC_DIR/changelog.Debian.gz" ] || [ -f "$DOC_DIR/changelog.Debian" ]; then
            print_test_pass "Debian changelog installed"
        else
            print_test_fail "Debian changelog not found"
        fi
    else
        print_test_fail "Documentation directory not found"
    fi
}

# Run validation with lintian
test_lintian() {
    print_info "Running lintian validation..."
    
    if ! command -v lintian &> /dev/null; then
        print_warning "lintian is not installed. Skipping validation."
        print_info "Install with: sudo apt install lintian"
        return 0
    fi
    
    # Find the .deb file
    DEB_FILE=$(find "$PROJECT_DIR" -maxdepth 1 -name "*.deb" -type f | head -n 1)
    if [ -z "$DEB_FILE" ]; then
        DEB_FILE=$(find "$PROJECT_DIR/target/debian" -name "*.deb" -type f | head -n 1)
    fi
    
    if [ -z "$DEB_FILE" ]; then
        print_warning "No .deb file found for lintian validation"
        return 0
    fi
    
    print_info "Running lintian on $DEB_FILE..."
    if lintian --no-tag-display-limit "$DEB_FILE" 2>&1 | grep -E "^(E:|W:)"; then
        print_test_fail "Lintian found errors or warnings"
    else
        print_test_pass "Lintian validation passed"
    fi
}

# Print test summary
print_summary() {
    echo ""
    echo "========================================="
    echo "           TEST SUMMARY"
    echo "========================================="
    echo -e "${GREEN}Tests Passed:${NC} $TESTS_PASSED"
    echo -e "${RED}Tests Failed:${NC} $TESTS_FAILED"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}All tests passed successfully!${NC}"
        return 0
    else
        echo -e "${RED}Some tests failed. Please review the output above.${NC}"
        return 1
    fi
}

# Main execution
main() {
    print_info "Starting package testing for $PACKAGE_NAME"
    echo ""
    
    # Run tests
    test_installation
    test_plugin_file
    test_gst_registration
    test_pipeline
    test_dependencies
    test_documentation
    test_lintian
    
    # Print summary
    print_summary
}

# Run main function
main "$@"