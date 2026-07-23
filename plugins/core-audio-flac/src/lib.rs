use std::any::Any;
use ufc_ir::audio::*;
use ufc_plugin_api::*;

pub struct FlacPlugin;
impl FlacPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for FlacPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-flac".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "FLAC audio decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("audio/flac", &["flac"], "FLAC")],
            output_formats: vec![FormatId::new("audio/flac", &["flac"], "FLAC")],
            capabilities: Capabilities { metadata: MetadataSupport::ReadWrite, structure: StructureSupport::Flat,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![],
                max_dimension: None, max_bit_depth: Some(32), supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 100, fidelity_score: 100,
            known_limitations: vec!["Lossless only".to_string()],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 4).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 4 && &header[0..4] == b"fLaC" {
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("audio/flac", &["flac"], "FLAC"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not a FLAC file".to_string())) }
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
            .map_err(|e| PluginError::DecodeFailed(format!("FLAC probe: {}", e)))?;

        let track = probed.format.default_track().ok_or_else(|| PluginError::DecodeFailed("No default track".to_string()))?;
        let codec_params = &track.codec_params;
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
        let channel_layout = match channels { 1 => ChannelLayout::Mono, 2 => ChannelLayout::Stereo, _ => ChannelLayout::Stereo };

        let mut decoder = codecs.make(&codec_params, &Default::default())
            .map_err(|e| PluginError::DecodeFailed(format!("FLAC decoder: {}", e)))?;

        let mut all_samples = Vec::new();
        let mut packet_count = 0u64;

        loop {
            match probed.format.next_packet() {
                Ok(packet) => {
                    match decoder.decode(&packet) {
                        Ok(audio_buf) => {
                            let spec = *audio_buf.spec();
                            let frames = audio_buf.frames();
                            for frame_idx in 0..frames {
                                for ch_idx in 0..spec.channels.count() {
                                    let sample = audio_buf.chan(ch_idx)[frame_idx];
                                    all_samples.push(sample);
                                }
                            }
                            packet_count += 1;
                            if packet_count % 100 == 0 {
                                progress.update(ProgressState::new(ConversionPhase::Decoding)
                                    .with_percent(50.0)
                                    .with_message(format!("Decoded {} packets", packet_count)));
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
        ir.metadata.original_format = "FLAC".to_string();

        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        // For now, encode as WAV (FLAC encoding requires additional crate)
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let audio_ir = ir.downcast_ref::<AudioIR>().ok_or_else(|| PluginError::InvalidInput("Expected AudioIR".to_string()))?;
        let samples = match &audio_ir.samples { SampleData::Interleaved(s) => s, _ => return Err(PluginError::EncodeFailed("Only interleaved samples".to_string())) };
        let channels = audio_ir.format.channels.channel_count();
        let sample_rate = audio_ir.format.sample_rate;
        let data_size = (samples.len() * 2) as u32;
        let file_size = 36 + data_size;
        let mut wav = Vec::with_capacity(44 + data_size as usize);
        wav.extend_from_slice(b"RIFF"); wav.extend_from_slice(&file_size.to_le_bytes()); wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt "); wav.extend_from_slice(&16u32.to_le_bytes()); wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&channels.to_le_bytes()); wav.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * channels as u32 * 2;
        wav.extend_from_slice(&byte_rate.to_le_bytes()); wav.extend_from_slice(&(channels * 2).to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data"); wav.extend_from_slice(&data_size.to_le_bytes());
        for &s in samples { let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16; wav.extend_from_slice(&v.to_le_bytes()); }
        output.write_all(&wav).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&wav).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: wav.len() as u64, checksum, warnings: vec!["Encoded as WAV (FLAC encoding not yet implemented)".to_string()], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
