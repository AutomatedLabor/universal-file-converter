# Universal File Converter

A cross-platform desktop application and CLI tool for converting files across formats. Runs entirely offline with no LLMs, no cloud APIs, no telemetry, and no paid services.

## Features

- **60+ format support** across images, documents, audio, video, archives, and structured data
- **Plugin architecture** — every format converter is an isolated, independently addable plugin
- **Batch conversion** — convert hundreds of files at once
- **Integrity verification** — Blake3 checksums ensure output correctness
- **Offline** — no internet required, no data leaves your machine
- **Cross-platform** — Windows, macOS, Linux

## Quick Start

### CLI

```bash
# Convert a single file
ufc convert photo.png --format jpg

# Batch convert
ufc batch "*.png" --output-dir ./converted --format webp

# Detect format
ufc detect mystery-file

# List supported conversions
ufc list
```

### Desktop App

```bash
# Run the Tauri desktop app
cargo run -p ufc-tauri
```

## Supported Formats

| Category | Formats |
|----------|---------|
| **Images** | PNG, JPEG, WebP, BMP, TIFF, GIF, AVIF, ICO |
| **Documents** | PDF, DOCX, HTML, Markdown, RTF, Plain Text |
| **Audio** | WAV, FLAC, MP3, AAC, OGG Vorbis, Opus |
| **Video** | MP4, MKV, AVI, MOV, WebM (via FFmpeg) |
| **Archives** | ZIP, TAR, TAR.GZ, 7-Zip |
| **Data** | CSV, JSON, XML, YAML |

## Architecture

```
Source File → Format Detector → Conversion Router → Decoder Plugin → IR → Encoder Plugin → Output File
```

The system uses **Intermediate Representations (IRs)** to avoid N×M pairwise converters:
- **ImageIR** — pixels, color spaces, metadata, animation
- **DocumentIR** — blocks, styles, tables, embedded resources
- **AudioIR** — samples, channels, tags
- **VideoIR** — tracks, chapters, metadata
- **ArchiveIR** — directory tree, compression
- **TableIR** — schema, rows, types

## Building

```bash
# Build everything
cargo build --workspace

# Build CLI only
cargo build --release -p ufc-cli

# Run tests
cargo test --workspace

# Run clippy
cargo clippy --workspace -- -D warnings
```

## Plugin Development

Plugins implement the `ConverterPlugin` trait from `ufc-plugin-api`:

```rust
use ufc_plugin_api::*;

pub struct MyPlugin;

impl ConverterPlugin for MyPlugin {
    fn manifest(&self) -> PluginManifest { /* ... */ }
    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> { /* ... */ }
    fn decode(&self, input: &FileReader, config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> { /* ... */ }
    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> { /* ... */ }
    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
```

See `docs/plugin-development.md` for the full guide.

## License

MIT OR Apache-2.0
