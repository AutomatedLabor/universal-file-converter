use std::any::Any;
use ufc_ir::audio::*;
use ufc_plugin_api::*;

pub struct VorbisPlugin;
impl VorbisPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for VorbisPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-vorbis".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "OGG Vorbis audio decoder".to_string(),
            input_formats: vec![FormatId::new("audio/ogg", &["ogg", "oga"], "OGG Vorbis")],
            output_formats: vec![],
            capabilities: Capabilities { metadata: MetadataSupport::ReadOnly, structure: StructureSupport::Flat,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![],
                max_dimension: None, max_bit_depth: Some(16), supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 90, fidelity_score: 90,
            known_limitations: vec!["Encode not supported".to_string()],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 4).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 4 && &header[0..4] == b"OggS" {
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("audio/ogg", &["ogg", "oga"], "OGG Vorbis"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not an OGG file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let cursor = std::io::Cursor::new(data);
        let hint = symphonia::core::probe::Hint::new();
        let media_source = symphonia::core::io::MediaSourceStream::new(Box::new(cursor), Default::default());
        let format_opts = symphonia::core::formats::FormatOptions::default();
        let metadata_opts = symphonia::core::meta::MetadataOptions::default();
        let codecs = symphonia::default::get_codecs();

        let mut probed = symphonia::default::get_probe()
            .format(&hint, media_source, &format_opts, &metadata_opts)
            .map_err(|e| PluginError::DecodeFailed(format!("OGG probe: {}", e)))?;

        let track = probed.format.default_track().ok_or_else(|| PluginError::DecodeFailed("No default track".to_string()))?;
        let codec_params = &track.codec_params;
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
        let channel_layout = match channels { 1 => ChannelLayout::Mono, 2 => ChannelLayout::Stereo, _ => ChannelLayout::Stereo };

        let mut decoder = codecs.make(&codec_params, &Default::default())
            .map_err(|e| PluginError::DecodeFailed(format!("Vorbis decoder: {}", e)))?;

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
        ir.metadata.original_format = "OGG Vorbis".to_string();
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, _ir: &(dyn Any + Send + Sync), _output: &FileWriter, _config: &EncodeConfig, _progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        Err(PluginError::EncodeFailed("Vorbis encoding not supported".to_string()))
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
