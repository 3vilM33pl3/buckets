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

command -v cargo >/dev/null 2>&1 || { 
    echo "Error: cargo not found. Please install Rust:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
}

# Clean any previous builds
echo "Cleaning previous builds..."
cargo clean 2>/dev/null || true
rm -f ../buckets_*.deb ../buckets_*.dsc ../buckets_*.tar.gz ../buckets_*.buildinfo ../buckets_*.changes

# Build the package
echo "Building package (this may take several minutes on first run)..."
echo "Downloading and compiling dependencies..."
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