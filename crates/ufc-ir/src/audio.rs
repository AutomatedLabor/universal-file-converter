use crate::traits::{IntermediateRepresentation, ValidationError, ValidationSeverity};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Audio Intermediate Representation.
///
/// Covers: WAV, FLAC, MP3, AAC, OGG/Vorbis, Opus, WMA, AIFF, ALAC, M4A
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioIR {
    pub version: Version,
    pub format: AudioFormat,
    pub samples: SampleData,
    pub metadata: AudioMetadata,
    pub chapters: Vec<Chapter>,
    pub tags: AudioTags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: ChannelLayout,
    pub bit_depth: AudioBitDepth,
    pub sample_format: SampleFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelLayout {
    Mono,
    Stereo,
    Surround5_1,
    Surround7_1,
    Custom(Vec<ChannelDef>),
}

impl ChannelLayout {
    pub fn channel_count(&self) -> u16 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
            Self::Surround5_1 => 6,
            Self::Surround7_1 => 8,
            Self::Custom(channels) => channels.len() as u16,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDef {
    pub id: String,
    pub position: (f64, f64, f64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioBitDepth {
    U8,
    I16,
    I24,
    I32,
    F32,
    F64,
}

impl AudioBitDepth {
    pub fn bits_per_sample(&self) -> u8 {
        match self {
            Self::U8 => 8,
            Self::I16 => 16,
            Self::I24 => 24,
            Self::I32 => 32,
            Self::F32 => 32,
            Self::F64 => 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleFormat {
    Integer,
    Float,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SampleData {
    /// Interleaved f32 samples (normalized to -1.0..1.0)
    Interleaved(Vec<f32>),
    /// Per-channel storage
    Planar(Vec<Vec<f32>>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub duration: Duration,
    pub original_format: String,
    pub original_bitrate: Option<u32>,
    pub encoder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub comment: Option<String>,
    pub cover_art: Option<Vec<u8>>,
    pub replay_gain: Option<ReplayGain>,
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayGain {
    pub track_gain_db: f64,
    pub track_peak: f64,
    pub album_gain_db: Option<f64>,
    pub album_peak: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub start: Duration,
    pub end: Duration,
    pub title: String,
}

impl AudioIR {
    pub fn new(sample_rate: u32, channels: ChannelLayout, bit_depth: AudioBitDepth) -> Self {
        Self {
            version: crate::api_version(),
            format: AudioFormat {
                sample_rate,
                channels,
                bit_depth,
                sample_format: SampleFormat::Integer,
            },
            samples: SampleData::Interleaved(Vec::new()),
            metadata: AudioMetadata {
                duration: Duration::ZERO,
                original_format: String::new(),
                original_bitrate: None,
                encoder: None,
            },
            chapters: Vec::new(),
            tags: AudioTags {
                title: None,
                artist: None,
                album: None,
                album_artist: None,
                track_number: None,
                disc_number: None,
                year: None,
                genre: None,
                comment: None,
                cover_art: None,
                replay_gain: None,
                custom: HashMap::new(),
            },
        }
    }

    pub fn total_samples(&self) -> u64 {
        match &self.samples {
            SampleData::Interleaved(s) => s.len() as u64,
            SampleData::Planar(p) => p.first().map(|c| c.len() as u64).unwrap_or(0),
        }
    }

    pub fn sample_count_per_channel(&self) -> u64 {
        let ch = self.format.channels.channel_count() as u64;
        match &self.samples {
            SampleData::Interleaved(s) => s.len() as u64 / ch,
            SampleData::Planar(p) => p.first().map(|c| c.len() as u64).unwrap_or(0),
        }
    }
}

impl IntermediateRepresentation for AudioIR {
    fn version(&self) -> Version {
        self.version.clone()
    }

    fn ir_type(&self) -> &'static str {
        "Audio"
    }

    fn memory_usage(&self) -> u64 {
        let sample_bytes = match &self.samples {
            SampleData::Interleaved(s) => s.len() as u64 * 4,
            SampleData::Planar(p) => p.iter().map(|c| c.len() as u64 * 4).sum(),
        };
        let cover_bytes = self.tags.cover_art.as_ref().map(|c| c.len() as u64).unwrap_or(0);
        sample_bytes + cover_bytes + 1024
    }

    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.format.sample_rate == 0 {
            errors.push(ValidationError {
                field: "format.sample_rate".into(),
                message: "Sample rate must be > 0".into(),
                severity: ValidationSeverity::Error,
            });
        }
        if self.format.sample_rate > 768000 {
            errors.push(ValidationError {
                field: "format.sample_rate".into(),
                message: "Sample rate exceeds reasonable maximum (768kHz)".into(),
                severity: ValidationSeverity::Warning,
            });
        }
        errors
    }

    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    fn is_empty(&self) -> bool {
        match &self.samples {
            SampleData::Interleaved(s) => s.is_empty(),
            SampleData::Planar(p) => p.is_empty() || p.iter().all(|c| c.is_empty()),
        }
    }
}
