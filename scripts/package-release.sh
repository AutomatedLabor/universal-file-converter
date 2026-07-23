#!/bin/bash
# Package release builds for all platforms
set -e

VERSION=${1:-"0.1.0"}
echo "Packaging Universal File Converter v${VERSION}..."

# Build release
cargo build --release -p ufc-cli

# Create release directory
mkdir -p "release/ufc-v${VERSION}"

# Copy binaries
cp target/release/ufc "release/ufc-v${VERSION}/"

# Copy plugins
mkdir -p "release/ufc-v${VERSION}/plugins"
cp -r plugins/ "release/ufc-v${VERSION}/plugins/" 2>/dev/null || true

# Copy docs
cp README.md LICENSE "release/ufc-v${VERSION}/"

# Create archive
cd release
tar czf "ufc-v${VERSION}-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]').tar.gz" "ufc-v${VERSION}/"
cd ..

echo "Package created: release/ufc-v${VERSION}-*.tar.gz"
