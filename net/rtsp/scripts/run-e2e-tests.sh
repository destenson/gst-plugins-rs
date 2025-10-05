#!/bin/bash

# End-to-End Test Runner for RTSP Plugin
# Comprehensive testing script for QA validation

set -e

# Configuration
PLUGIN_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKSPACE_ROOT="$(cd "$PLUGIN_DIR/../../.." && pwd)"
TEST_RESULTS_DIR="$PLUGIN_DIR/e2e_test_results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check dependencies
check_dependencies() {
    print_status "Checking dependencies..."
    
    # Check GStreamer
    if ! command -v gst-launch-1.0 &> /dev/null; then
        print_error "gst-launch-1.0 not found. Install GStreamer development tools."
        exit 1
    fi
    
    if ! command -v gst-inspect-1.0 &> /dev/null; then
        print_error "gst-inspect-1.0 not found. Install GStreamer development tools."
        exit 1
    fi
    
    # Check Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        print_error "cargo not found. Install Rust toolchain."
        exit 1
    fi
    
    # Check required GStreamer plugins
    local required_plugins=("videotestsrc" "fakesink" "rtph264depay" "h264parse")
    for plugin in "${required_plugins[@]}"; do
        if ! gst-inspect-1.0 --exists "$plugin" &> /dev/null; then
            print_warning "GStreamer plugin '$plugin' not available"
        fi
    done
    
    print_success "Dependencies check completed"
}

# Function to build the plugin
build_plugin() {
    print_status "Building RTSP plugin..."
    cd "$WORKSPACE_ROOT"
    
    if ! cargo build -p gst-plugin-rtsp; then
        print_error "Failed to build plugin"
        exit 1
    fi
    
    print_success "Plugin built successfully"
}

# Function to set up test environment
setup_test_env() {
    print_status "Setting up test environment..."
    
    # Create results directory
    mkdir -p "$TEST_RESULTS_DIR"
    
    # Set plugin path
    export GST_PLUGIN_PATH="$WORKSPACE_ROOT/target/debug:$GST_PLUGIN_PATH"
    
    print_status "Plugin path: $GST_PLUGIN_PATH"
    print_success "Test environment ready"
}

# Function to run basic plugin tests
run_basic_tests() {
    print_status "Running basic plugin tests..."
    cd "$PLUGIN_DIR"
    
    # Run unit tests first
    if ! cargo test --lib; then
        print_error "Unit tests failed"
        return 1
    fi
    
    print_success "Basic tests completed"
}

# Function to run E2E plugin tests
run_e2e_plugin_tests() {
    print_status "Running E2E plugin tests..."
    cd "$PLUGIN_DIR"
    
    if ! cargo test --test e2e_plugin_tests -- --nocapture; then
        print_warning "Some E2E plugin tests failed (may be expected)"
        return 1
    fi
    
    print_success "E2E plugin tests completed"
}

# Function to run pipeline tests
run_pipeline_tests() {
    print_status "Running pipeline tests..."
    cd "$PLUGIN_DIR"
    
    if ! cargo test --test e2e_pipeline_tests -- --nocapture; then
        print_warning "Some pipeline tests failed (may be expected for network tests)"
        return 1
    fi
    
    print_success "Pipeline tests completed"
}

# Function to run inspection tests
run_inspection_tests() {
    print_status "Running element inspection tests..."
    cd "$PLUGIN_DIR"
    
    if ! cargo test --test e2e_inspection_tests -- --nocapture; then
        print_warning "Some inspection tests failed"
        return 1
    fi
    
    print_success "Inspection tests completed"
}

# Function to run manual visual tests
run_visual_tests() {
    print_status "Visual tests require manual verification..."
    
    if [[ "${DISPLAY:-}" == "" ]] && [[ "$OSTYPE" == "linux-gnu"* ]]; then
        print_warning "No DISPLAY set - skipping visual tests"
        return 0
    fi
    
    print_status "You can run visual tests manually with:"
    echo "  cargo test --test e2e_visual_tests -- --ignored --nocapture"
    echo ""
    echo "Or run interactively:"
    echo "  cd $PLUGIN_DIR && cargo run --example visual_test_runner"
    
    read -p "Run a quick visual test now? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_status "Running quick visual test (5 seconds)..."
        
        timeout 10s gst-launch-1.0 \
            rtspsrc2 location=rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2 \
            ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink \
            || print_warning "Visual test may have failed (network dependent)"
    fi
}

# Function to test manual gst-launch commands
run_manual_commands() {
    print_status "Testing manual gst-launch commands..."
    
    # Test 1: Basic element existence
    print_status "Testing element existence..."
    if gst-inspect-1.0 --exists rtspsrc2; then
        print_success "✓ rtspsrc2 element found"
    else
        print_error "✗ rtspsrc2 element not found"
        return 1
    fi
    
    # Test 2: Element inspection
    print_status "Testing element inspection..."
    if gst-inspect-1.0 rtspsrc2 > "$TEST_RESULTS_DIR/rtspsrc2_inspection_$TIMESTAMP.txt"; then
        print_success "✓ Element inspection successful"
        print_status "Inspection saved to: $TEST_RESULTS_DIR/rtspsrc2_inspection_$TIMESTAMP.txt"
    else
        print_error "✗ Element inspection failed"
        return 1
    fi
    
    # Test 3: Basic pipeline creation (should fail quickly)
    print_status "Testing basic pipeline creation..."
    timeout 5s gst-launch-1.0 --quiet rtspsrc2 location=rtsp://invalid.test ! fakesink \
        &> "$TEST_RESULTS_DIR/basic_pipeline_test_$TIMESTAMP.txt" || true
    
    if grep -q "rtspsrc2" "$TEST_RESULTS_DIR/basic_pipeline_test_$TIMESTAMP.txt"; then
        print_success "✓ Pipeline creation works (expected failure)"
    else
        print_error "✗ Pipeline creation failed unexpectedly"
        cat "$TEST_RESULTS_DIR/basic_pipeline_test_$TIMESTAMP.txt"
        return 1
    fi
    
    print_success "Manual command tests completed"
}

# Function to generate summary report
generate_summary_report() {
    local report_file="$TEST_RESULTS_DIR/e2e_test_summary_$TIMESTAMP.md"
    
    print_status "Generating test summary report..."
    
    cat > "$report_file" << EOF
# RTSP Plugin E2E Test Summary

**Test Date**: $(date)
**Plugin Path**: $GST_PLUGIN_PATH
**Working Directory**: $PLUGIN_DIR

## Test Results

EOF
    
    # Add results based on what was run
    if [[ -f "$TEST_RESULTS_DIR/rtspsrc2_inspection_$TIMESTAMP.txt" ]]; then
        echo "- ✅ Element inspection: PASSED" >> "$report_file"
    fi
    
    echo "" >> "$report_file"
    echo "## Manual Testing Commands" >> "$report_file"
    echo "" >> "$report_file"
    echo "### Basic Element Test" >> "$report_file"
    echo '```bash' >> "$report_file"
    echo "gst-inspect-1.0 rtspsrc2" >> "$report_file"
    echo '```' >> "$report_file"
    echo "" >> "$report_file"
    echo "### Public Stream Test" >> "$report_file"
    echo '```bash' >> "$report_file"
    echo "gst-launch-1.0 rtspsrc2 location=rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2 ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink" >> "$report_file"
    echo '```' >> "$report_file"
    echo "" >> "$report_file"
    echo "### Camera Test Template" >> "$report_file"
    echo '```bash' >> "$report_file"
    echo "gst-launch-1.0 rtspsrc2 location=rtsp://your-camera-ip/stream user-id=admin user-pw=password ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink" >> "$report_file"
    echo '```' >> "$report_file"
    
    print_success "Summary report generated: $report_file"
}

# Function to display usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

End-to-End test runner for RTSP plugin

OPTIONS:
    -h, --help          Show this help message
    -b, --build-only    Only build the plugin
    -t, --test-only     Run tests without building
    -v, --visual        Run visual tests interactively
    -q, --quick         Run only quick tests
    --no-build          Skip building the plugin

EXAMPLES:
    $0                  Run full E2E test suite
    $0 --quick          Run quick validation tests
    $0 --visual         Run interactive visual tests
    $0 --build-only     Only build the plugin
EOF
}

# Main execution function
main() {
    local build_only=false
    local test_only=false
    local visual_only=false
    local quick_mode=false
    local no_build=false
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            -b|--build-only)
                build_only=true
                shift
                ;;
            -t|--test-only)
                test_only=true
                shift
                ;;
            -v|--visual)
                visual_only=true
                shift
                ;;
            -q|--quick)
                quick_mode=true
                shift
                ;;
            --no-build)
                no_build=true
                shift
                ;;
            *)
                print_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
    
    # Header
    echo "========================================"
    echo "  RTSP Plugin E2E Test Runner"
    echo "========================================"
    echo ""
    
    # Check dependencies
    check_dependencies
    echo ""
    
    # Build plugin unless skipped
    if [[ "$no_build" != true ]] && [[ "$test_only" != true ]]; then
        build_plugin
        echo ""
    fi
    
    if [[ "$build_only" == true ]]; then
        print_success "Build completed successfully!"
        exit 0
    fi
    
    # Set up test environment
    setup_test_env
    echo ""
    
    # Run visual tests only
    if [[ "$visual_only" == true ]]; then
        run_visual_tests
        exit 0
    fi
    
    local overall_success=true
    
    # Run tests based on mode
    if [[ "$quick_mode" == true ]]; then
        print_status "Running quick validation tests..."
        run_manual_commands || overall_success=false
    else
        # Full test suite
        print_status "Running comprehensive E2E test suite..."
        
        run_basic_tests || overall_success=false
        echo ""
        
        run_e2e_plugin_tests || overall_success=false
        echo ""
        
        run_inspection_tests || overall_success=false
        echo ""
        
        run_pipeline_tests || overall_success=false
        echo ""
        
        run_manual_commands || overall_success=false
        echo ""
        
        run_visual_tests
        echo ""
    fi
    
    # Generate summary report
    generate_summary_report
    echo ""
    
    # Final status
    if [[ "$overall_success" == true ]]; then
        print_success "🎉 E2E test suite completed successfully!"
        echo ""
        print_status "Next steps for QA:"
        echo "  1. Review test reports in: $TEST_RESULTS_DIR"
        echo "  2. Test with real cameras using the provided commands"
        echo "  3. Run visual tests: cargo test --test e2e_visual_tests -- --ignored"
        echo "  4. Verify plugin works in your target environment"
    else
        print_warning "⚠️  Some tests failed - check the output above"
        echo ""
        print_status "This may be expected if:"
        echo "  - Plugin is not built yet"
        echo "  - Network connectivity issues"
        echo "  - Missing optional GStreamer plugins"
        exit 1
    fi
}

# Run main function
main "$@"
