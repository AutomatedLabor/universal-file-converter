use crate::traits::{IntermediateRepresentation, ValidationError, ValidationSeverity};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Video Intermediate Representation.
///
/// Covers: MP4, MKV, AVI, MOV, WebM, FLV
/// Note: Video is primarily handled via FFmpeg process plugin.
/// This IR captures metadata and track structure, not raw frames.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoIR {
    pub version: Version,
    pub container: ContainerInfo,
    pub video_tracks: Vec<VideoTrack>,
    pub audio_tracks: Vec<AudioTrackInfo>,
    pub subtitle_tracks: Vec<SubtitleTrack>,
    pub chapters: Vec<VideoChapter>,
    pub metadata: VideoMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub format: String,
    pub duration: Duration,
    pub bitrate: u64,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTrack {
    pub index: u32,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub bitrate: u64,
    pub pixel_format: String,
    pub color_space: Option<String>,
    pub hdr: bool,
    pub rotation: Option<i32>,
    pub frame_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrackInfo {
    pub index: u32,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub bitrate: u64,
    pub language: Option<String>,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub index: u32,
    pub codec: String,
    pub language: Option<String>,
    pub default: bool,
    pub forced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoChapter {
    pub start: Duration,
    pub end: Duration,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub date: Option<String>,
    pub comment: Option<String>,
    pub genre: Option<String>,
    pub encoder: Option<String>,
}

impl VideoIR {
    pub fn new(format: &str, duration: Duration, file_size: u64) -> Self {
        Self {
            version: crate::api_version(),
            container: ContainerInfo {
                format: format.to_string(),
                duration,
                bitrate: if duration.as_secs() > 0 {
                    (file_size * 8) / duration.as_secs()
                } else {
                    0
                },
                file_size,
            },
            video_tracks: Vec::new(),
            audio_tracks: Vec::new(),
            subtitle_tracks: Vec::new(),
            chapters: Vec::new(),
            metadata: VideoMetadata {
                title: None,
                artist: None,
                album: None,
                date: None,
                comment: None,
                genre: None,
                encoder: None,
            },
        }
    }

    pub fn primary_video_track(&self) -> Option<&VideoTrack> {
        self.video_tracks.first()
    }

    pub fn primary_audio_track(&self) -> Option<&AudioTrackInfo> {
        self.audio_tracks.iter().find(|t| t.default).or_else(|| self.audio_tracks.first())
    }
}

impl IntermediateRepresentation for VideoIR {
    fn version(&self) -> Version { self.version.clone() }
    fn ir_type(&self) -> &'static str { "Video" }
    fn memory_usage(&self) -> u64 { 8192 } // metadata only, no raw frames
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.video_tracks.is_empty() {
            errors.push(ValidationError {
                field: "video_tracks".into(),
                message: "No video tracks found".into(),
                severity: ValidationSeverity::Warning,
            });
        }
        errors
    }
    fn to_json(&self) -> Result<String, serde_json::Error> { serde_json::to_string_pretty(self) }
    fn is_empty(&self) -> bool { self.video_tracks.is_empty() }
}
