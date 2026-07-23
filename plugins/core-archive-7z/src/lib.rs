use std::any::Any;
use ufc_ir::archive::*;
use ufc_plugin_api::*;

pub struct SevenZPlugin;
impl SevenZPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for SevenZPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-7z".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "7-Zip archive decoder (read-only)".to_string(),
            input_formats: vec![FormatId::new("application/x-7z-compressed", &["7z"], "7-Zip")],
            output_formats: vec![],
            capabilities: Capabilities { metadata: MetadataSupport::None, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::Extract, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 80, fidelity_score: 95,
            known_limitations: vec!["Encode not supported".to_string()],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 6).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 6 && &header[0..6] == b"7z\xBC\xAF\x27\x1C" {
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("application/x-7z-compressed", &["7z"], "7-Zip"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not a 7z file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let mut ir = ArchiveIR::new("7Z");
        ir.metadata.total_compressed_size = data.len() as u64;

        let sz = sevenz_rust::SevenZReader::new(std::io::Cursor::new(data), data.len() as u64)
            .map_err(|e| PluginError::DecodeFailed(format!("7z: {}", e)))?;

        for entry in sz.entries() {
            let entry_type = if entry.is_directory { EntryType::Directory } else { EntryType::File };
            ir.add_entry(ArchiveEntry {
                path: entry.name.clone(),
                entry_type,
                size: entry.size,
                compressed_size: None,
                permissions: None,
                modified: None,
                created: None,
                data: EntryData::Empty, // Don't decompress by default
                checksum: None,
            });
        }

        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, _ir: &(dyn Any + Send + Sync), _output: &FileWriter, _config: &EncodeConfig, _progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        Err(PluginError::EncodeFailed("7z encoding not supported".to_string()))
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
