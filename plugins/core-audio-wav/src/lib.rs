use std::any::Any;
use ufc_ir::audio::*;
use ufc_plugin_api::*;
use std::io::{Cursor, Write, Seek};

pub struct WavPlugin;
impl WavPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for WavPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-wav".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "WAV audio decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("audio/wav", &["wav"], "WAV")],
            output_formats: vec![FormatId::new("audio/wav", &["wav"], "WAV")],
            capabilities: Capabilities { metadata: MetadataSupport::ReadOnly, structure: StructureSupport::Flat,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![],
                max_dimension: None, max_bit_depth: Some(32), supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 100, fidelity_score: 100, known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 12).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WAVE" {
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("audio/wav", &["wav"], "WAV"),
                format_version: Some("RIFF/WAVE".to_string()), estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not a WAV file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;

        // Parse WAV header
        if data.len() < 44 {
            return Err(PluginError::DecodeFailed("WAV file too small".to_string()));
        }
        let channels = u16::from_le_bytes([data[22], data[23]]);
        let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
        let bits_per_sample = u16::from_le_bytes([data[34], data[35]]);

        let channel_layout = match channels {
            1 => ChannelLayout::Mono,
            2 => ChannelLayout::Stereo,
            _ => ChannelLayout::Stereo,
        };

        let bit_depth = match bits_per_sample {
            8 => AudioBitDepth::U8,
            16 => AudioBitDepth::I16,
            24 => AudioBitDepth::I24,
            32 => AudioBitDepth::I32,
            _ => AudioBitDepth::I16,
        };

        // Find data chunk
        let mut data_offset = 36;
        while data_offset + 8 <= data.len() {
            let chunk_id = &data[data_offset..data_offset + 4];
            let chunk_size = u32::from_le_bytes([data[data_offset + 4], data[data_offset + 5], data[data_offset + 6], data[data_offset + 7]]) as usize;
            if chunk_id == b"data" {
                data_offset += 8;
                break;
            }
            data_offset += 8 + chunk_size;
        }

        let audio_data = &data[data_offset..];
        let bytes_per_sample = (bits_per_sample / 8) as usize;
        let total_samples = audio_data.len() / (bytes_per_sample * channels as usize);
        let duration = std::time::Duration::from_secs_f64(total_samples as f64 / sample_rate as f64);

        // Convert to f32 samples
        let mut samples = Vec::with_capacity(total_samples * channels as usize);
        for chunk in audio_data.chunks(bytes_per_sample * channels as usize) {
            for s in chunk.chunks(bytes_per_sample) {
                let sample = match bits_per_sample {
                    8 => (s[0] as f32 / 255.0) * 2.0 - 1.0,
                    16 => {
                        let val = i16::from_le_bytes([s[0], s[1]]);
                        val as f32 / 32768.0
                    }
                    24 => {
                        let val = i32::from_le_bytes([0, s[0], s[1], s[2]]);
                        (val >> 8) as f32 / 8388608.0
                    }
                    32 => {
                        let val = i32::from_le_bytes([s[0], s[1], s[2], s[3]]);
                        val as f32 / 2147483648.0
                    }
                    _ => 0.0,
                };
                samples.push(sample);
            }
        }

        let mut ir = AudioIR::new(sample_rate, channel_layout, bit_depth);
        ir.samples = SampleData::Interleaved(samples);
        ir.metadata.duration = duration;
        ir.metadata.original_format = "WAV".to_string();

        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let audio_ir = ir.downcast_ref::<AudioIR>().ok_or_else(|| PluginError::InvalidInput("Expected AudioIR".to_string()))?;

        let channels = audio_ir.format.channels.channel_count();
        let sample_rate = audio_ir.format.sample_rate;
        let bits_per_sample: u16 = 16;
        let bytes_per_sample = (bits_per_sample / 8) as usize;
        let byte_rate = sample_rate * channels as u32 * bytes_per_sample as u32;
        let block_align = channels * bytes_per_sample as u16;

        let samples = match &audio_ir.samples {
            SampleData::Interleaved(s) => s,
            SampleData::Planar(p) => {
                // Interleave planar samples
                // For now, just use first channel
                if let Some(ch) = p.first() { ch } else { &vec![] }
            }
        };

        let data_size = (samples.len() * bytes_per_sample) as u32;
        let file_size = 36 + data_size;

        let mut wav_data = Vec::with_capacity(44 + data_size as usize);

        // RIFF header
        wav_data.extend_from_slice(b"RIFF");
        wav_data.extend_from_slice(&file_size.to_le_bytes());
        wav_data.extend_from_slice(b"WAVE");

        // fmt chunk
        wav_data.extend_from_slice(b"fmt ");
        wav_data.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        wav_data.extend_from_slice(&1u16.to_le_bytes()); // PCM format
        wav_data.extend_from_slice(&channels.to_le_bytes());
        wav_data.extend_from_slice(&sample_rate.to_le_bytes());
        wav_data.extend_from_slice(&byte_rate.to_le_bytes());
        wav_data.extend_from_slice(&block_align.to_le_bytes());
        wav_data.extend_from_slice(&bits_per_sample.to_le_bytes());

        // data chunk
        wav_data.extend_from_slice(b"data");
        wav_data.extend_from_slice(&data_size.to_le_bytes());

        // Convert f32 to i16
        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let i16_val = (clamped * 32767.0) as i16;
            wav_data.extend_from_slice(&i16_val.to_le_bytes());
        }

        output.write_all(&wav_data).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&wav_data).to_hex().to_string();
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(100.0));
        Ok(ConversionOutput { bytes_written: wav_data.len() as u64, checksum, warnings: vec![], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
