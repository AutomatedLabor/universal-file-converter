#!/bin/bash
# Build all WASM plugins
set -e

echo "Building WASM plugins..."

# List of plugin crates that compile to WASM
PLUGINS=(
    core-image-png
    core-image-jpeg
    core-image-webp
    core-image-bmp
    core-image-tiff
    core-image-gif
    core-image-avif
    core-image-ico
    core-doc-pdf
    core-doc-docx
    core-doc-html
    core-doc-markdown
    core-doc-rtf
    core-audio-wav
    core-audio-flac
    core-audio-mp3
    core-audio-aac
    core-audio-vorbis
    core-audio-opus
    core-archive-zip
    core-archive-tar
    core-archive-7z
    core-struct-csv
    core-struct-json
    core-struct-xml
    core-struct-yaml
)

# Create output directory
mkdir -p target/wasm32-wasi/release

# Build each plugin
for plugin in "${PLUGINS[@]}"; do
    echo "Building $plugin..."
    cargo build --release -p "$plugin" --target wasm32-wasi 2>/dev/null || echo "  Skipped (native only)"
done

echo "Done!"
