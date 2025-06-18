#!/bin/bash

# Pre-release check script for xr2280x-hid crate
# This script performs comprehensive checks before releasing a new version

set -e  # Exit immediately if a command exits with a non-zero status
set -u  # Exit if an undefined variable is used

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${BLUE}=== $1 ===${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Function to run a command with error handling
run_check() {
    local description="$1"
    shift
    print_step "$description"

    if "$@"; then
        print_success "$description completed successfully"
        echo
    else
        print_error "$description failed!"
        echo -e "${RED}Command that failed: $*${NC}"
        exit 1
    fi
}

# Function to check and install optional tools
check_and_install_tool() {
    local tool_name="$1"
    local package_name="$2"
    local description="$3"

    # Extract subcommand name from tool name (e.g., "cargo-audit" -> "audit")
    local subcommand="${tool_name#cargo-}"

    # Check if cargo subcommand is available
    if ! cargo "$subcommand" --help >/dev/null 2>&1; then
        echo -e "${YELLOW}Optional tool '$tool_name' is not installed.${NC}"
        echo -e "${BLUE}Description: $description${NC}"
        echo -n -e "${BLUE}Do you want to install it? [Y/n]: ${NC}"
        read -r response

        # Default to 'Y' if user just presses enter
        if [[ -z "$response" || "$response" =~ ^[Yy]$ ]]; then
            echo -e "${BLUE}Installing $package_name...${NC}"
            if cargo install "$package_name"; then
                print_success "$package_name installed successfully"
            else
                print_error "Failed to install $package_name"
                exit 1
            fi
        else
            echo -e "${YELLOW}Skipping installation of $package_name${NC}"
        fi
        echo
    else
        print_success "$tool_name is already installed"
    fi
}

# Get the current version from Cargo.toml
CURRENT_VERSION=$(grep -E '^version\s*=' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')

echo -e "${BLUE}🚀 Pre-release checks for xr2280x-hid v${CURRENT_VERSION}${NC}"
echo

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Cargo.toml not found. Please run this script from the project root."
    exit 1
fi

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    print_warning "Not in a git repository. Some checks will be skipped."
else
    # Check for uncommitted changes
    if ! git diff --quiet; then
        print_warning "You have uncommitted changes. Consider committing them before release."
        git status --porcelain
        echo
    fi

    # Check if current version is already tagged
    if git tag --list | grep -q "^v${CURRENT_VERSION}$"; then
        print_warning "Version v${CURRENT_VERSION} is already tagged in git."
    fi
fi

# Check and install optional tools
print_step "Checking optional tools"
check_and_install_tool "cargo-audit" "cargo-audit" "Security vulnerability scanner for Rust dependencies"
check_and_install_tool "cargo-outdated" "cargo-outdated" "Tool to check for outdated dependencies"

# Check for TODO/FIXME comments in source code
print_step "Checking for TODO/FIXME comments"
if grep -r --include="*.rs" -n "TODO\|FIXME" src/; then
    print_warning "Found TODO/FIXME comments in source code. Consider addressing them before release."
else
    print_success "No TODO/FIXME comments found in source code"
fi
echo

# Run cargo check first (fastest way to catch basic issues)
run_check "Running cargo check" cargo check --all-targets --all-features

# Check code formatting
run_check "Checking code formatting" cargo fmt --check

# Run linting with clippy
run_check "Running clippy linter" cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
run_check "Running all tests" cargo test

# Build examples to ensure they compile
run_check "Building examples" cargo build --examples

# Check documentation generation
run_check "Checking documentation generation" cargo doc --no-deps

# Check if docs can be built with all features
run_check "Checking documentation with all features" cargo doc --no-deps --all-features

# Validate package before publishing
run_check "Validating package (dry run)" cargo publish --dry-run --allow-dirty

# Check for security vulnerabilities
run_check "Running security audit" cargo audit

# Check for outdated dependencies
print_step "Checking for outdated dependencies"
cargo outdated
echo

# Final success message
echo -e "${GREEN}🎉 All pre-release checks passed successfully!${NC}"
echo -e "${GREEN}Version ${CURRENT_VERSION} is ready for release.${NC}"
echo
echo -e "${BLUE}Next steps:${NC}"
echo "1. Review the changes one more time"
echo "2. Update CHANGELOG.md if needed"
echo "3. Commit any final changes"
echo "4. Tag the release: git tag v${CURRENT_VERSION}"
echo "5. Push tags: git push --tags"
echo "6. Publish to crates.io: cargo publish"
echo

exit 0
