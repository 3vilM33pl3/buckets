#!/bin/bash
set -e

# Build Debian package for buckets
# This script builds a .deb package that can be installed on Debian/Ubuntu systems

echo "Building Debian package for buckets..."

# Check if required tools are installed
command -v dpkg-buildpackage >/dev/null 2>&1 || { 
    echo "Error: dpkg-buildpackage not found. Please install dpkg-dev package:"
    echo "  sudo apt-get install dpkg-dev debhelper devscripts"
    exit 1
}

# Check if the binary exists
if [ ! -f "target/release/buckets" ]; then
    echo "Error: Binary not found at target/release/buckets"
    echo "Please build the binary first with: cargo build --release"
    exit 1
fi

# Clean any previous packages
echo "Cleaning previous packages..."
rm -f ../buckets_*.deb ../buckets_*.dsc ../buckets_*.tar.gz ../buckets_*.buildinfo ../buckets_*.changes

# Build the package (binary is already built)
echo "Creating Debian package..."
dpkg-buildpackage -us -uc -b -j$(nproc)

echo ""
echo "Package built successfully!"
echo "The .deb file is located at: ../buckets_*.deb"
echo ""
echo "To install the package, run:"
echo "  sudo dpkg -i ../buckets_*.deb"
echo ""
echo "To remove the package later, run:"
echo "  sudo apt-get remove buckets"