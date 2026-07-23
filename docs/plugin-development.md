# Plugin Development Guide

This guide explains how to create custom format converter plugins for the Universal File Converter.

## Overview

Every format converter is an isolated plugin that implements the `ConverterPlugin` trait. Plugins are compiled to WASM binaries and loaded by the plugin host at runtime.

## Plugin Interface

```rust
pub trait ConverterPlugin: Send + Sync {
    fn manifest(&self) -> PluginManifest;
    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError>;
    fn decode(&self, input: &FileReader, config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError>;
    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError>;
    fn cancel(&self) -> Result<(), PluginError>;
}
```

## Creating a New Plugin

### 1. Create the crate

```bash
cargo new --lib plugins/my-format-decoder
```

### 2. Add dependencies

```toml
[dependencies]
ufc-plugin-api = { path = "../../crates/ufc-plugin-api" }
ufc-ir = { path = "../../crates/ufc-ir" }
serde = { version = "1", features = ["derive"] }
```

### 3. Implement the trait

```rust
use std::any::Any;
use ufc_plugin_api::*;
use ufc_ir::image::ImageIR;

pub struct MyFormatPlugin;

impl ConverterPlugin for MyFormatPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "my-format".to_string(),
            version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0),
            // ... fill in all fields
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        // Read header bytes and check magic numbers
        let header = input.read_slice(0, 8)?;
        if is_my_format(&header) {
            Ok(ProbeResult { confidence: 100, /* ... */ })
        } else {
            Err(PluginError::UnsupportedFormat("Not my format".into()))
        }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all()?;
        // Parse format and build IR
        let ir = parse_my_format(&data)?;
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let image_ir = ir.downcast_ref::<ImageIR>()
            .ok_or_else(|| PluginError::InvalidInput("Expected ImageIR".into()))?;
        // Encode IR to target format
        let bytes = encode_to_my_format(image_ir)?;
        output.write_all(&bytes)?;
        let checksum = blake3::hash(&bytes).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
```

### 4. Register the plugin

Add your plugin to the workspace `Cargo.toml` and register it in the CLI/Tauri app.

## Best Practices

1. **Always check `progress.is_cancelled()`** at natural yield points
2. **Report progress** regularly so the UI can update
3. **Validate input** before processing — return `PluginError::InvalidInput` for bad data
4. **Handle errors gracefully** — never panic, always return `PluginError`
5. **Be memory-conscious** — use streaming for large files
6. **Declare capabilities accurately** in your manifest

## Testing

```bash
# Unit tests
cargo test -p my-format-decoder

# Integration tests (with golden files)
cargo test --test integration
```
