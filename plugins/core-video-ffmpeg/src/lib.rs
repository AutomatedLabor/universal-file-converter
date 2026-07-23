use std::any::Any;
use std::process::Command;
use ufc_ir::video::*;
use ufc_plugin_api::*;
use std::time::Duration;

pub struct FFmpegPlugin;
impl FFmpegPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for FFmpegPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-ffmpeg".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "Video decoder and encoder via FFmpeg (process-sandboxed)".to_string(),
            input_formats: vec![
                FormatId::new("video/mp4", &["mp4", "m4v"], "MP4"),
                FormatId::new("video/x-matroska", &["mkv"], "MKV"),
                FormatId::new("video/x-msvideo", &["avi"], "AVI"),
                FormatId::new("video/quicktime", &["mov"], "MOV"),
                FormatId::new("video/webm", &["webm"], "WebM"),
            ],
            output_formats: vec![
                FormatId::new("video/mp4", &["mp4"], "MP4"),
                FormatId::new("video/webm", &["webm"], "WebM"),
                FormatId::new("video/x-matroska", &["mkv"], "MKV"),
            ],
            capabilities: Capabilities { metadata: MetadataSupport::ReadWrite, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::ExtractAndEmbed, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![Dependency { name: "ffmpeg".to_string(), version_req: semver::VersionReq::parse(">=4.0").unwrap(), optional: false }],
            priority: 100, fidelity_score: 95,
            known_limitations: vec!["Requires FFmpeg installed on system".to_string(), "Process-sandboxed".to_string()],
            sandbox_mode: SandboxMode::Process,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let path = input.path();
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
        let supported = matches!(ext.as_deref(), Some("mp4") | Some("m4v") | Some("mkv") | Some("avi") | Some("mov") | Some("webm") | Some("flv"));
        if !supported {
            return Err(PluginError::UnsupportedFormat("Not a supported video format".to_string()));
        }

        // Use ffprobe if available
        let output = Command::new("ffprobe")
            .args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams", path.to_str().unwrap_or("")])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                Ok(ProbeResult { confidence: 100,
                    detected_format: FormatId::new("video/mp4", &[ext.as_deref().unwrap_or("mp4")], "Video"),
                    format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
            }
            _ => Ok(ProbeResult { confidence: 70,
                detected_format: FormatId::new("video/mp4", &[ext.as_deref().unwrap_or("mp4")], "Video"),
                format_version: None, estimated_size: Some(input.size()),
                warnings: vec!["ffprobe not available — format detected by extension only".to_string()] }),
        }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let path = input.path();

        // Use ffprobe to get video metadata
        let output = Command::new("ffprobe")
            .args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams", path.to_str().unwrap_or("")])
            .output()
            .map_err(|e| PluginError::IoError(format!("ffprobe: {}", e)))?;

        if !output.status.success() {
            return Err(PluginError::DecodeFailed("ffprobe failed".to_string()));
        }

        let info: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| PluginError::DecodeFailed(format!("ffprobe JSON: {}", e)))?;

        let format_name = info["format"]["format_name"].as_str().unwrap_or("unknown");
        let duration_secs = info["format"]["duration"].as_str()
            .and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        let file_size = info["format"]["size"].as_str()
            .and_then(|s| s.parse::<u64>().ok()).unwrap_or(input.size());

        let mut ir = VideoIR::new(format_name, Duration::from_secs_f64(duration_secs), file_size);

        // Parse streams
        if let Some(streams) = info["streams"].as_array() {
            for stream in streams {
                let codec_type = stream["codec_type"].as_str().unwrap_or("");
                let index = stream["index"].as_u64().unwrap_or(0) as u32;
                let codec = stream["codec_name"].as_str().unwrap_or("unknown").to_string();

                match codec_type {
                    "video" => {
                        let width = stream["width"].as_u64().unwrap_or(0) as u32;
                        let height = stream["height"].as_u64().unwrap_or(0) as u32;
                        let fps = stream["r_frame_rate"].as_str()
                            .and_then(|s| {
                                let parts: Vec<&str> = s.split('/').collect();
                                if parts.len() == 2 {
                                    let num: f64 = parts[0].parse().ok()?;
                                    let den: f64 = parts[1].parse().ok()?;
                                    if den > 0.0 { Some(num / den) } else { None }
                                } else { s.parse().ok() }
                            }).unwrap_or(30.0);
                        let bitrate = stream["bit_rate"].as_str()
                            .and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);

                        ir.video_tracks.push(VideoTrack {
                            index, codec, width, height, fps, bitrate,
                            pixel_format: stream["pix_fmt"].as_str().unwrap_or("yuv420p").to_string(),
                            color_space: None, hdr: false, rotation: None, frame_count: 0,
                        });
                    }
                    "audio" => {
                        let sample_rate = stream["sample_rate"].as_str()
                            .and_then(|s| s.parse::<u32>().ok()).unwrap_or(44100);
                        let channels = stream["channels"].as_u64().unwrap_or(2) as u16;
                        let bitrate = stream["bit_rate"].as_str()
                            .and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        let language = stream["tags"]["language"].as_str().map(|s| s.to_string());

                        ir.audio_tracks.push(AudioTrackInfo {
                            index, codec, sample_rate, channels, bitrate, language, default: false,
                        });
                    }
                    "subtitle" => {
                        let language = stream["tags"]["language"].as_str().map(|s| s.to_string());
                        ir.subtitle_tracks.push(SubtitleTrack {
                            index, codec, language, default: false, forced: false,
                        });
                    }
                    _ => {}
                }
            }
        }

        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let video_ir = ir.downcast_ref::<VideoIR>().ok_or_else(|| PluginError::InvalidInput("Expected VideoIR".to_string()))?;

        let output_path = output.path();
        let quality = config.quality.to_quality_float();

        // Build FFmpeg encode command
        // For now, this is a placeholder — actual encoding requires the source file path
        // In a real implementation, the VideoIR would reference the source file
        let crf = ((1.0 - quality) * 51.0) as u32;

        // Since we can't actually re-encode without the source file,
        // return an error indicating this requires the full pipeline
        Err(PluginError::EncodeFailed(
            "Video encoding requires source file path — use the full conversion pipeline".to_string()
        ))
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
