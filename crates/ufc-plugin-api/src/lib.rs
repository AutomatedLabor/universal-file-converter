//! # UFC Plugin API
//!
//! This crate defines the public interface contract for all Universal File Converter plugins.
//! Plugin authors depend **only** on this crate (and `ufc-ir` for the IR types they produce/consume).
//!
//! ## Key types:
//! - [`ConverterPlugin`] — the main trait every plugin implements
//! - [`PluginManifest`] — static metadata about the plugin
//! - [`FormatId`] — unique format identifier (MIME + extensions)
//! - [`Capabilities`] — what the plugin preserves during conversion
//!
//! ## Plugin lifecycle:
//! 1. Host loads plugin and calls `manifest()` to register it
//! 2. On conversion request, host calls `probe()` to confirm format
//! 3. Host calls `decode()` to produce an IR
//! 4. Host calls `encode()` on a target plugin with the IR
//! 5. Host verifies output integrity via checksum

pub mod config;
pub mod error;
pub mod io;
pub mod progress;
pub mod traits;
pub mod types;

pub use config::{DecodeConfig, EncodeConfig, QualityPreset};
pub use error::PluginError;
pub use io::{FileReader, FileWriter};
pub use progress::{ConversionPhase, ProgressCallback, ProgressState};
pub use traits::ConverterPlugin;
pub use types::{
    BlendMethod, BlendMode, Capabilities, ColorSpace, CompressionInfo, ConflictResolution,
    ConversionOutput, Dependency, EmbeddedAssetSupport, ExifData, FormatId, MetadataSupport,
    PluginManifest, ProbeResult, SandboxMode, StructureSupport,
};
