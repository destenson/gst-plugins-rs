#!/bin/bash
set -e

# Build script for gst-plugin-uriplaylistbin Debian package
# This script automates the building of the Debian package using cargo-deb

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PACKAGE_NAME="gst-plugin-uriplaylistbin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check for required tools
check_requirements() {
    print_info "Checking build requirements..."
    
    # Check for Rust and Cargo
    if ! command -v cargo &> /dev/null; then
        print_error "cargo is not installed. Please install Rust."
        exit 1
    fi
    
    # Check for cargo-deb
    if ! cargo deb --version &> /dev/null 2>&1; then
        print_warning "cargo-deb is not installed. Installing..."
        cargo install cargo-deb
    fi
    
    # Check for cargo-c (needed for C ABI)
    if ! cargo cbuild --version &> /dev/null 2>&1; then
        print_warning "cargo-c is not installed. Installing..."
        cargo install cargo-c
    fi
    
    print_info "All requirements satisfied."
}

# Clean previous builds
clean_build() {
    print_info "Cleaning previous builds..."
    cd "$PROJECT_DIR"
    cargo clean
    rm -f target/debian/*.deb
}

# Build the Rust project
build_project() {
    print_info "Building $PACKAGE_NAME..."
    cd "$PROJECT_DIR"
    
    # Build in release mode with optimizations
    cargo build --release
    
    if [ ! -f "target/release/libgsturiplaylistbin.so" ]; then
        print_error "Build failed: libgsturiplaylistbin.so not found"
        exit 1
    fi
    
    print_info "Build completed successfully."
}

# Generate Debian package
build_deb_package() {
    print_info "Generating Debian package..."
    cd "$PROJECT_DIR"
    
    # Build the debian package
    # cargo-deb will use the metadata from Cargo.toml
    cargo deb --no-build  # We already built in release mode
    
    # Find the generated .deb file
    DEB_FILE=$(find target/debian -name "*.deb" -type f | head -n 1)
    
    if [ -z "$DEB_FILE" ]; then
        print_error "Failed to generate Debian package"
        exit 1
    fi
    
    print_info "Debian package created: $DEB_FILE"
    
    # Display package info
    print_info "Package information:"
    dpkg-deb --info "$DEB_FILE"
    
    # Display package contents
    print_info "Package contents:"
    dpkg-deb --contents "$DEB_FILE"
    
    # Copy to a convenient location
    cp "$DEB_FILE" "$PROJECT_DIR/"
    print_info "Package copied to: $PROJECT_DIR/$(basename "$DEB_FILE")"
}

# Main execution
main() {
    print_info "Starting Debian package build for $PACKAGE_NAME"
    
    check_requirements
    
    # Parse command line arguments
    CLEAN=0
    for arg in "$@"; do
        case $arg in
            --clean)
                CLEAN=1
                shift
                ;;
            --help)
                echo "Usage: $0 [--clean] [--help]"
                echo "  --clean  Clean build artifacts before building"
                echo "  --help   Show this help message"
                exit 0
                ;;
        esac
    done
    
    if [ $CLEAN -eq 1 ]; then
        clean_build
    fi
    
    build_project
    build_deb_package
    
    print_info "Build process completed successfully!"
    print_info "To install the package, run:"
    print_info "  sudo dpkg -i $(basename "$DEB_FILE")"
    print_info "Or:"
    print_info "  sudo apt install ./$(basename "$DEB_FILE")"
}

# Run main function
main "$@"