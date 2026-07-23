use std::any::Any;
use std::io::Read;
use ufc_ir::archive::*;
use ufc_plugin_api::*;

pub struct ZipPlugin;
impl ZipPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for ZipPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-zip".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "ZIP archive decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("application/zip", &["zip"], "ZIP")],
            output_formats: vec![FormatId::new("application/zip", &["zip"], "ZIP")],
            capabilities: Capabilities { metadata: MetadataSupport::None, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::ExtractAndEmbed, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 100, fidelity_score: 100, known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 4).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 4 && header[0] == 0x50 && header[1] == 0x4B && header[2] == 0x03 && header[3] == 0x04 {
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("application/zip", &["zip"], "ZIP"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not a ZIP file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let cursor = std::io::Cursor::new(&data);
        let archive = zip::ZipArchive::new(cursor).map_err(|e| PluginError::DecodeFailed(format!("ZIP: {}", e)))?;

        let mut ir = ArchiveIR::new("ZIP");
        ir.metadata.total_size = data.len() as u64;
        ir.metadata.total_compressed_size = data.len() as u64;

        for i in 0..archive.len() {
            let file = archive.by_index(i).map_err(|e| PluginError::DecodeFailed(format!("ZIP entry: {}", e)))?;
            let entry_type = if file.is_dir() { EntryType::Directory }
                else if file.is_file() { EntryType::File }
                else { EntryType::File };

            let entry = ArchiveEntry {
                path: file.name().to_string(),
                entry_type,
                size: file.size(),
                compressed_size: Some(file.compressed_size()),
                permissions: file.unix_mode(),
                modified: None,
                created: None,
                data: if file.is_file() && file.size() < 10 * 1024 * 1024 {
                    let mut reader = file;
                    let mut buf = Vec::new();
                    reader.read_to_end(&mut buf).ok();
                    EntryData::Inline(buf)
                } else { EntryData::Empty },
                checksum: None,
            };
            ir.add_entry(entry);

            if i % 100 == 0 {
                progress.update(ProgressState::new(ConversionPhase::Decoding)
                    .with_percent(i as f32 / archive.len() as f32 * 100.0));
            }
        }
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let archive_ir = ir.downcast_ref::<ArchiveIR>().ok_or_else(|| PluginError::InvalidInput("Expected ArchiveIR".to_string()))?;

        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            for (i, entry) in archive_ir.entries.iter().enumerate() {
                match &entry.entry_type {
                    EntryType::Directory => {
                        zip.add_directory(&entry.path, options).map_err(|e| PluginError::IoError(e.to_string()))?;
                    }
                    EntryType::File => {
                        zip.start_file(&entry.path, options).map_err(|e| PluginError::IoError(e.to_string()))?;
                        match &entry.data {
                            EntryData::Inline(data) => zip.write_all(data).map_err(|e| PluginError::IoError(e.to_string()))?,
                            EntryData::Empty => {},
                            EntryData::Reference(path) => {
                                let data = std::fs::read(path).map_err(|e| PluginError::IoError(e.to_string()))?;
                                zip.write_all(&data).map_err(|e| PluginError::IoError(e.to_string()))?;
                            }
                        }
                    }
                    _ => {}
                }
                if i % 100 == 0 {
                    progress.update(ProgressState::new(ConversionPhase::Encoding)
                        .with_percent(i as f32 / archive_ir.entries.len() as f32 * 100.0));
                }
            }
            zip.finish().map_err(|e| PluginError::IoError(e.to_string()))?;
        }

        let zip_bytes = buf.into_inner();
        output.write_all(&zip_bytes).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&zip_bytes).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: zip_bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
