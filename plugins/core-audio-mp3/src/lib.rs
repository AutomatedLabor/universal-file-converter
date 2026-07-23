use std::any::Any;
use ufc_ir::audio::*;
use ufc_plugin_api::*;

pub struct Mp3Plugin;
impl Mp3Plugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for Mp3Plugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-mp3".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "MP3 audio decoder".to_string(),
            input_formats: vec![FormatId::new("audio/mpeg", &["mp3"], "MP3")],
            output_formats: vec![],
            capabilities: Capabilities { metadata: MetadataSupport::ReadOnly, structure: StructureSupport::Flat,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![],
                max_dimension: None, max_bit_depth: Some(16), supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 100, fidelity_score: 90,
            known_limitations: vec!["Encode not supported (no MP3 encoder)".to_string()],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 4).map_err(|e| PluginError::IoError(e.to_string()))?;
        // MP3 sync word (0xFF 0xFB/0xF3/0xF2) or ID3 tag
        if header.len() >= 3 && header[0] == 0xFF && (header[1] & 0xE0) == 0xE0 {
            Ok(ProbeResult { confidence: 95, detected_format: FormatId::new("audio/mpeg", &["mp3"], "MP3"),
                format_version: Some("MPEG Audio".to_string()), estimated_size: Some(input.size()), warnings: vec![] })
        } else if header.len() >= 3 && &header[0..3] == b"ID3" {
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("audio/mpeg", &["mp3"], "MP3"),
                format_version: Some("ID3v2".to_string()), estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not an MP3 file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let cursor = std::io::Cursor::new(data);
        let hint = symphonia::core::probe::Hint::new().with_extension("mp3");
        let media_source = symphonia::core::io::MediaSourceStream::new(Box::new(cursor), Default::default());
        let format_opts = symphonia::core::formats::FormatOptions::default();
        let metadata_opts = symphonia::core::meta::MetadataOptions::default();
        let codecs = symphonia::default::get_codecs();

        let mut probed = symphonia::default::get_probe()
            .format(&hint, media_source, &format_opts, &metadata_opts)
            .map_err(|e| PluginError::DecodeFailed(format!("MP3 probe: {}", e)))?;

        let track = probed.format.default_track().ok_or_else(|| PluginError::DecodeFailed("No default track".to_string()))?;
        let codec_params = &track.codec_params;
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
        let channel_layout = match channels { 1 => ChannelLayout::Mono, 2 => ChannelLayout::Stereo, _ => ChannelLayout::Stereo };

        let mut decoder = codecs.make(&codec_params, &Default::default())
            .map_err(|e| PluginError::DecodeFailed(format!("MP3 decoder: {}", e)))?;

        let mut all_samples = Vec::new();
        loop {
            match probed.format.next_packet() {
                Ok(packet) => {
                    match decoder.decode(&packet) {
                        Ok(audio_buf) => {
                            let spec = *audio_buf.spec();
                            let frames = audio_buf.frames();
                            for frame_idx in 0..frames {
                                for ch_idx in 0..spec.channels.count() {
                                    all_samples.push(audio_buf.chan(ch_idx)[frame_idx]);
                                }
                            }
                        }
                        Err(_) => continue,
                    }
                }
                Err(symphonia::core::errors::Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(_) => break,
            }
        }

        let duration = std::time::Duration::from_secs_f64(all_samples.len() as f64 / (sample_rate as f64 * channels as f64));
        let mut ir = AudioIR::new(sample_rate, channel_layout, AudioBitDepth::I16);
        ir.samples = SampleData::Interleaved(all_samples);
        ir.metadata.duration = duration;
        ir.metadata.original_format = "MP3".to_string();
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, _ir: &(dyn Any + Send + Sync), _output: &FileWriter, _config: &EncodeConfig, _progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        Err(PluginError::EncodeFailed("MP3 encoding not supported".to_string()))
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
