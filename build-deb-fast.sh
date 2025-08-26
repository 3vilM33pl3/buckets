#!/bin/bash
set -e

# Fast Debian package build for buckets (no clean, reuses build artifacts)
# This script builds a .deb package faster by reusing existing build artifacts

echo "Fast building Debian package for buckets..."

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

# Only remove old packages, keep build artifacts
echo "Cleaning previous packages..."
rm -f ../buckets_*.deb ../buckets_*.dsc ../buckets_*.tar.gz ../buckets_*.buildinfo ../buckets_*.changes

# Build the package without cleaning (faster)
echo "Building package (reusing build artifacts)..."
dpkg-buildpackage -us -uc -b -j$(nproc) --no-pre-clean

echo ""
echo "Package built successfully!"
echo "The .deb file is located at: ../buckets_*.deb"
echo ""
echo "To install the package, run:"
echo "  sudo dpkg -i ../buckets_*.deb"
echo ""
echo "To remove the package later, run:"
echo "  sudo apt-get remove buckets"