#!/bin/bash
set -e

# Validation script for gst-plugin-uriplaylistbin Debian package
# This script performs comprehensive validation before release

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PACKAGE_NAME="gst-plugin-uriplaylistbin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Validation results
VALIDATIONS_PASSED=0
VALIDATIONS_FAILED=0
VALIDATIONS_WARNED=0

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_section() {
    echo ""
    echo -e "${BLUE}==== $1 ====${NC}"
}

validation_pass() {
    echo -e "  ${GREEN}✓${NC} $1"
    ((VALIDATIONS_PASSED++))
}

validation_fail() {
    echo -e "  ${RED}✗${NC} $1"
    ((VALIDATIONS_FAILED++))
}

validation_warn() {
    echo -e "  ${YELLOW}⚠${NC} $1"
    ((VALIDATIONS_WARNED++))
}

# Validate Cargo.toml configuration
validate_cargo_toml() {
    print_section "Validating Cargo.toml"
    
    CARGO_TOML="$PROJECT_DIR/Cargo.toml"
    
    if [ ! -f "$CARGO_TOML" ]; then
        validation_fail "Cargo.toml not found"
        return 1
    fi
    
    # Check for required sections
    if grep -q "\[package.metadata.deb\]" "$CARGO_TOML"; then
        validation_pass "Debian metadata section present"
    else
        validation_fail "Missing [package.metadata.deb] section"
    fi
    
    # Check for required fields
    REQUIRED_FIELDS=("name" "maintainer" "copyright" "license-file" "depends" "assets")
    for field in "${REQUIRED_FIELDS[@]}"; do
        if grep -q "^$field = " "$CARGO_TOML"; then
            validation_pass "Field '$field' present"
        else
            validation_fail "Missing required field: $field"
        fi
    done
    
    # Check library configuration
    if grep -q "crate-type.*cdylib" "$CARGO_TOML"; then
        validation_pass "Library configured as cdylib"
    else
        validation_fail "Library must include 'cdylib' in crate-type"
    fi
}

# Validate Debian control files
validate_debian_files() {
    print_section "Validating Debian Control Files"
    
    DEBIAN_DIR="$PROJECT_DIR/debian"
    
    if [ ! -d "$DEBIAN_DIR" ]; then
        validation_fail "debian/ directory not found"
        return 1
    fi
    
    # Check required files
    REQUIRED_FILES=("control" "copyright" "rules" "changelog" "compat" "source/format")
    for file in "${REQUIRED_FILES[@]}"; do
        if [ -f "$DEBIAN_DIR/$file" ]; then
            validation_pass "File debian/$file exists"
        else
            validation_fail "Missing required file: debian/$file"
        fi
    done
    
    # Check optional but recommended files
    OPTIONAL_FILES=("postinst" "postrm" "prerm" "README.Debian")
    for file in "${OPTIONAL_FILES[@]}"; do
        if [ -f "$DEBIAN_DIR/$file" ]; then
            validation_pass "Optional file debian/$file exists"
        else
            validation_warn "Missing optional file: debian/$file"
        fi
    done
    
    # Validate control file syntax
    if [ -f "$DEBIAN_DIR/control" ]; then
        if grep -q "^Source:" "$DEBIAN_DIR/control" && \
           grep -q "^Package:" "$DEBIAN_DIR/control" && \
           grep -q "^Architecture:" "$DEBIAN_DIR/control"; then
            validation_pass "Control file has required fields"
        else
            validation_fail "Control file missing required fields"
        fi
    fi
    
    # Check rules file is executable
    if [ -f "$DEBIAN_DIR/rules" ]; then
        if [ -x "$DEBIAN_DIR/rules" ] || head -n1 "$DEBIAN_DIR/rules" | grep -q "^#!/usr/bin/make"; then
            validation_pass "Rules file is properly configured"
        else
            validation_warn "Rules file might need executable permissions"
        fi
    fi
}

# Validate build artifacts
validate_build_artifacts() {
    print_section "Validating Build Artifacts"
    
    # Check if library builds successfully
    print_info "Attempting to build library..."
    if cargo build --release -p "$PACKAGE_NAME" 2>&1 | grep -q "error"; then
        validation_fail "Build failed"
    else
        validation_pass "Library builds successfully"
        
        # Check for the .so file
        SO_FILE="$PROJECT_DIR/target/release/libgsturiplaylistbin.so"
        if [ -f "$SO_FILE" ]; then
            validation_pass "Shared library created: libgsturiplaylistbin.so"
            
            # Check library exports
            if nm -D "$SO_FILE" 2>/dev/null | grep -q "gst_plugin_"; then
                validation_pass "Library exports GStreamer plugin symbols"
            else
                validation_fail "Library missing GStreamer plugin exports"
            fi
        else
            validation_fail "Shared library not found after build"
        fi
    fi
}

# Validate package structure
validate_package_structure() {
    print_section "Validating Package Structure"
    
    # Try to build the package
    if ! command -v cargo-deb &> /dev/null; then
        validation_warn "cargo-deb not installed, skipping package build validation"
        return 0
    fi
    
    print_info "Building Debian package..."
    if cargo deb --no-build 2>&1 | grep -q "error"; then
        validation_fail "Package build failed"
        return 1
    else
        validation_pass "Package builds successfully"
    fi
    
    # Find and validate the .deb file
    DEB_FILE=$(find "$PROJECT_DIR/target/debian" -name "*.deb" -type f | head -n 1)
    
    if [ -z "$DEB_FILE" ]; then
        validation_fail "No .deb file generated"
        return 1
    fi
    
    validation_pass "Debian package created: $(basename "$DEB_FILE")"
    
    # Check package contents
    print_info "Checking package contents..."
    
    # Check for the plugin file
    if dpkg-deb --contents "$DEB_FILE" | grep -q "libgsturiplaylistbin.so"; then
        validation_pass "Package contains plugin library"
    else
        validation_fail "Package missing plugin library"
    fi
    
    # Check for documentation
    if dpkg-deb --contents "$DEB_FILE" | grep -q "usr/share/doc"; then
        validation_pass "Package contains documentation"
    else
        validation_warn "Package might be missing documentation"
    fi
    
    # Check package metadata
    print_info "Checking package metadata..."
    
    if dpkg-deb --info "$DEB_FILE" | grep -q "Package: $PACKAGE_NAME"; then
        validation_pass "Package name is correct"
    else
        validation_fail "Package name mismatch"
    fi
    
    if dpkg-deb --info "$DEB_FILE" | grep -q "Depends:.*gstreamer"; then
        validation_pass "Package has GStreamer dependencies"
    else
        validation_warn "Package might be missing GStreamer dependencies"
    fi
}

# Run lintian checks
validate_with_lintian() {
    print_section "Lintian Validation"
    
    if ! command -v lintian &> /dev/null; then
        validation_warn "lintian not installed, skipping policy checks"
        print_info "Install with: sudo apt install lintian"
        return 0
    fi
    
    DEB_FILE=$(find "$PROJECT_DIR/target/debian" -name "*.deb" -type f | head -n 1)
    if [ -z "$DEB_FILE" ]; then
        DEB_FILE=$(find "$PROJECT_DIR" -maxdepth 1 -name "*.deb" -type f | head -n 1)
    fi
    
    if [ -z "$DEB_FILE" ]; then
        validation_warn "No .deb file found for lintian validation"
        return 0
    fi
    
    print_info "Running lintian checks..."
    
    LINTIAN_OUTPUT=$(lintian --no-tag-display-limit "$DEB_FILE" 2>&1 || true)
    
    # Check for errors
    if echo "$LINTIAN_OUTPUT" | grep -q "^E:"; then
        validation_fail "Lintian found errors"
        echo "$LINTIAN_OUTPUT" | grep "^E:" | head -5
    else
        validation_pass "No lintian errors"
    fi
    
    # Check for warnings
    WARNING_COUNT=$(echo "$LINTIAN_OUTPUT" | grep -c "^W:" || true)
    if [ "$WARNING_COUNT" -gt 0 ]; then
        validation_warn "Lintian found $WARNING_COUNT warnings"
        echo "$LINTIAN_OUTPUT" | grep "^W:" | head -5
    else
        validation_pass "No lintian warnings"
    fi
}

# Validate documentation
validate_documentation() {
    print_section "Validating Documentation"
    
    # Check for README
    if [ -f "$PROJECT_DIR/README.md" ]; then
        validation_pass "README.md exists"
        
        # Check README content
        if grep -q "uriplaylistbin" "$PROJECT_DIR/README.md"; then
            validation_pass "README mentions uriplaylistbin"
        else
            validation_warn "README might need updating"
        fi
    else
        validation_fail "README.md not found"
    fi
    
    # Check for LICENSE
    if [ -f "$PROJECT_DIR/LICENSE-MPL-2.0" ]; then
        validation_pass "License file exists"
    else
        validation_fail "License file not found"
    fi
    
    # Check for Debian README
    if [ -f "$PROJECT_DIR/debian/README.Debian" ]; then
        validation_pass "Debian-specific README exists"
    else
        validation_warn "Missing debian/README.Debian"
    fi
}

# Print validation summary
print_summary() {
    print_section "Validation Summary"
    
    echo ""
    echo -e "${GREEN}Passed:${NC} $VALIDATIONS_PASSED"
    echo -e "${YELLOW}Warnings:${NC} $VALIDATIONS_WARNED"
    echo -e "${RED}Failed:${NC} $VALIDATIONS_FAILED"
    echo ""
    
    if [ $VALIDATIONS_FAILED -eq 0 ]; then
        if [ $VALIDATIONS_WARNED -eq 0 ]; then
            echo -e "${GREEN}✓ Package validation completed successfully!${NC}"
        else
            echo -e "${GREEN}✓ Package validation passed with warnings.${NC}"
        fi
        return 0
    else
        echo -e "${RED}✗ Package validation failed. Please fix the errors above.${NC}"
        return 1
    fi
}

# Main execution
main() {
    print_info "Starting validation for $PACKAGE_NAME"
    
    # Change to project directory
    cd "$PROJECT_DIR"
    
    # Run all validations
    validate_cargo_toml
    validate_debian_files
    validate_build_artifacts
    validate_package_structure
    validate_with_lintian
    validate_documentation
    
    # Print summary
    print_summary
}

# Run main function
main "$@"