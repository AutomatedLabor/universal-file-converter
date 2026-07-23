use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─────────────────────────────────────────────
// Format identification
// ─────────────────────────────────────────────

/// Unique identifier for a file format.
///
/// A format is identified primarily by its MIME type, with file extensions
/// as secondary identifiers for user-facing operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FormatId {
    /// MIME type (primary identifier), e.g. "image/png"
    pub mime: String,
    /// Common file extensions without dots, e.g. ["png", "apng"]
    pub extensions: Vec<String>,
    /// Human-readable name, e.g. "Portable Network Graphics"
    pub display_name: String,
}

impl FormatId {
    pub fn new(mime: &str, extensions: &[&str], display_name: &str) -> Self {
        Self {
            mime: mime.to_string(),
            extensions: extensions.iter().map(|s| s.to_string()).collect(),
            display_name: display_name.to_string(),
        }
    }

    /// Check if this format matches the given extension (case-insensitive).
    pub fn matches_extension(&self, ext: &str) -> bool {
        let ext_lower = ext.to_lowercase();
        self.extensions.iter().any(|e| e.to_lowercase() == ext_lower)
    }

    /// Check if this format matches the given MIME type.
    pub fn matches_mime(&self, mime: &str) -> bool {
        self.mime.eq_ignore_ascii_case(mime)
    }
}

impl std::fmt::Display for FormatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name)
    }
}

// ─────────────────────────────────────────────
// Capabilities
// ─────────────────────────────────────────────

/// What a plugin can preserve during conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub metadata: MetadataSupport,
    pub structure: StructureSupport,
    pub embedded_assets: EmbeddedAssetSupport,
    pub color_spaces: Vec<ColorSpace>,
    pub max_dimension: Option<(u32, u32)>,
    pub max_bit_depth: Option<u8>,
    pub supports_animation: bool,
    pub supports_transparency: bool,
    pub supports_multi_page: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            metadata: MetadataSupport::None,
            structure: StructureSupport::Flat,
            embedded_assets: EmbeddedAssetSupport::None,
            color_spaces: vec![],
            max_dimension: None,
            max_bit_depth: None,
            supports_animation: false,
            supports_transparency: false,
            supports_multi_page: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataSupport {
    None,
    ReadOnly,
    ReadWrite,
    /// Can transform metadata between different format-specific schemas
    ReadWriteTransform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureSupport {
    /// No structure (e.g., plain text)
    Flat,
    /// Sections, headings, nesting
    Hierarchical,
    /// Tables, cross-references
    Relational,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbeddedAssetSupport {
    None,
    /// Can extract but not embed
    Extract,
    /// Can both extract and embed
    ExtractAndEmbed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    Gray,
    GrayAlpha,
    Rgb,
    Rgba,
    Cmyk,
    YCbCr,
    Lab,
    Hsl,
    Hsv,
    Indexed,
}

// ─────────────────────────────────────────────
// Compression
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionInfo {
    pub algorithm: String,
    pub level: Option<u32>,
    pub ratio: Option<f64>,
}

// ─────────────────────────────────────────────
// Plugin manifest
// ─────────────────────────────────────────────

/// Static metadata about a plugin, declared at load time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique identifier, e.g. "core-png-decoder"
    pub id: String,
    /// Plugin version
    pub version: Version,
    /// Plugin API version this plugin was built against
    pub api_version: Version,
    pub author: String,
    pub license: String,
    pub description: String,
    /// Formats this plugin can decode
    pub input_formats: Vec<FormatId>,
    /// Formats this plugin can encode
    pub output_formats: Vec<FormatId>,
    pub capabilities: Capabilities,
    pub dependencies: Vec<Dependency>,
    /// Higher = preferred when multiple plugins handle same format
    pub priority: i32,
    /// Self-declared fidelity score (0–100)
    pub fidelity_score: u8,
    pub known_limitations: Vec<String>,
    pub sandbox_mode: SandboxMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SandboxMode {
    /// Runs in WASM sandbox (preferred for safety)
    Wasm,
    /// Runs in separate process (for plugins needing native libs)
    Process,
    /// Runs in main process (only for trusted core plugins)
    InProcess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_req: semver::VersionReq,
    pub optional: bool,
}

// ─────────────────────────────────────────────
// Probe result
// ─────────────────────────────────────────────

/// Result of probing a file to detect its format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    /// Confidence score 0–100
    pub confidence: u8,
    pub detected_format: FormatId,
    pub format_version: Option<String>,
    pub estimated_size: Option<u64>,
    pub warnings: Vec<String>,
}

// ─────────────────────────────────────────────
// Conversion output
// ─────────────────────────────────────────────

/// Result of encoding an IR into a target format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionOutput {
    pub bytes_written: u64,
    pub checksum: String,
    pub warnings: Vec<String>,
    /// Self-estimated fidelity (0–100)
    pub fidelity_estimate: u8,
}

// ─────────────────────────────────────────────
// EXIF data
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifData {
    pub make: Option<String>,
    pub model: Option<String>,
    pub software: Option<String>,
    pub datetime: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<f64>,
    pub iso_speed: Option<u32>,
    pub focal_length: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub orientation: Option<u16>,
    pub gps: Option<GpsData>,
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsData {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

// ─────────────────────────────────────────────
// Image-specific types
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlendMethod {
    Source,
    Over,
}

// ─────────────────────────────────────────────
// Queue / state types
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    Overwrite,
    Rename,
    Skip,
    Ask,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::Rename
    }
}
