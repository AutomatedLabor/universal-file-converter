use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the decode phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeConfig {
    /// Maximum memory the decoder may use (bytes).
    pub max_memory_bytes: u64,
    /// If true, prefer speed over fidelity (e.g., skip metadata parsing).
    pub prefer_speed_over_quality: bool,
    /// If true, strip all metadata from the decoded IR.
    pub strip_metadata: bool,
    /// Format-specific custom options.
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for DecodeConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB
            prefer_speed_over_quality: false,
            strip_metadata: false,
            custom: HashMap::new(),
        }
    }
}

/// Configuration for the encode phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeConfig {
    /// Quality preset for lossy formats.
    pub quality: QualityPreset,
    /// Maximum memory the encoder may use (bytes).
    pub max_memory_bytes: u64,
    /// Whether to preserve metadata from the IR.
    pub preserve_metadata: bool,
    /// Format-specific custom options.
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for EncodeConfig {
    fn default() -> Self {
        Self {
            quality: QualityPreset::High,
            max_memory_bytes: 512 * 1024 * 1024,
            preserve_metadata: true,
            custom: HashMap::new(),
        }
    }
}

/// Quality presets for lossy encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityPreset {
    /// Lossless encoding (may not be available for all formats)
    Lossless,
    /// High quality (e.g., JPEG 95, Opus 256kbps)
    High,
    /// Medium quality (e.g., JPEG 80, Opus 128kbps)
    Medium,
    /// Low quality (e.g., JPEG 60, Opus 64kbps)
    Low,
    /// Custom quality parameters (format-specific key-value pairs)
    Custom(HashMap<String, f64>),
}

impl QualityPreset {
    /// Returns a numeric quality value for common formats (0.0–1.0).
    pub fn to_quality_float(&self) -> f64 {
        match self {
            QualityPreset::Lossless => 1.0,
            QualityPreset::High => 0.92,
            QualityPreset::Medium => 0.80,
            QualityPreset::Low => 0.60,
            QualityPreset::Custom(_) => 0.80, // fallback
        }
    }

    /// Returns a JPEG-specific quality value (1–100).
    pub fn to_jpeg_quality(&self) -> u8 {
        match self {
            QualityPreset::Lossless => 100,
            QualityPreset::High => 95,
            QualityPreset::Medium => 80,
            QualityPreset::Low => 60,
            QualityPreset::Custom(m) => m.get("jpeg").map(|v| *v as u8).unwrap_or(80),
        }
    }
}
