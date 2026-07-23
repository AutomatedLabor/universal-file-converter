use std::any::Any;
use std::io::Read;
use ufc_ir::archive::*;
use ufc_plugin_api::*;

pub struct TarPlugin;
impl TarPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for TarPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-tar".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "TAR archive decoder and encoder (supports gzip)".to_string(),
            input_formats: vec![
                FormatId::new("application/x-tar", &["tar"], "TAR"),
                FormatId::new("application/gzip", &["gz", "tgz"], "Gzip"),
            ],
            output_formats: vec![
                FormatId::new("application/x-tar", &["tar"], "TAR"),
                FormatId::new("application/gzip", &["gz", "tgz"], "Gzip"),
            ],
            capabilities: Capabilities { metadata: MetadataSupport::None, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::ExtractAndEmbed, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 90, fidelity_score: 100, known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let ext = input.path().extension().map(|e| e.to_string_lossy().to_lowercase());
        match ext.as_deref() {
            Some("tar") => Ok(ProbeResult { confidence: 90, detected_format: FormatId::new("application/x-tar", &["tar"], "TAR"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] }),
            Some("gz") | Some("tgz") => Ok(ProbeResult { confidence: 90, detected_format: FormatId::new("application/gzip", &["gz", "tgz"], "Gzip"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] }),
            _ => Err(PluginError::UnsupportedFormat("Not a TAR file".to_string())),
        }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;

        let is_gz = input.path().extension().map(|e| e.to_string_lossy().to_lowercase())
            .map(|e| e == "gz" || e == "tgz").unwrap_or(false);

        let mut ir = ArchiveIR::new(if is_gz { "TAR.GZ" } else { "TAR" });
        ir.metadata.compression = if is_gz { Some("gzip".to_string()) } else { None };

        if is_gz {
            let decoder = flate2::read::GzDecoder::new(std::io::Cursor::new(&data));
            let mut archive = tar::Archive::new(decoder);
            for entry in archive.entries().map_err(|e| PluginError::DecodeFailed(format!("TAR: {}", e)))? {
                let mut entry = entry.map_err(|e| PluginError::DecodeFailed(format!("TAR entry: {}", e)))?;
                let path = entry.path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let size = entry.header().size().unwrap_or(0);
                let entry_type = if entry.header().entry_type().is_dir() { EntryType::Directory }
                    else if entry.header().entry_type().is_symlink() { EntryType::Symlink { target: String::new() } }
                    else { EntryType::File };
                let mut buf = Vec::new();
                if entry_type == EntryType::File && size < 10 * 1024 * 1024 {
                    entry.read_to_end(&mut buf).ok();
                }
                ir.add_entry(ArchiveEntry {
                    path, entry_type, size, compressed_size: None,
                    permissions: entry.header().mode().ok(),
                    modified: entry.header().mtime().ok().map(|t| t as u64),
                    created: None, data: if buf.is_empty() { EntryData::Empty } else { EntryData::Inline(buf) },
                    checksum: None,
                });
            }
        } else {
            let mut archive = tar::Archive::new(std::io::Cursor::new(&data));
            for entry in archive.entries().map_err(|e| PluginError::DecodeFailed(format!("TAR: {}", e)))? {
                let mut entry = entry.map_err(|e| PluginError::DecodeFailed(format!("TAR entry: {}", e)))?;
                let path = entry.path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let size = entry.header().size().unwrap_or(0);
                let entry_type = if entry.header().entry_type().is_dir() { EntryType::Directory }
                    else if entry.header().entry_type().is_symlink() { EntryType::Symlink { target: String::new() } }
                    else { EntryType::File };
                let mut buf = Vec::new();
                if entry_type == EntryType::File && size < 10 * 1024 * 1024 {
                    entry.read_to_end(&mut buf).ok();
                }
                ir.add_entry(ArchiveEntry {
                    path, entry_type, size, compressed_size: None,
                    permissions: entry.header().mode().ok(),
                    modified: entry.header().mtime().ok().map(|t| t as u64),
                    created: None, data: if buf.is_empty() { EntryData::Empty } else { EntryData::Inline(buf) },
                    checksum: None,
                });
            }
        }

        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let archive_ir = ir.downcast_ref::<ArchiveIR>().ok_or_else(|| PluginError::InvalidInput("Expected ArchiveIR".to_string()))?;

        let is_gz = archive_ir.metadata.compression.as_deref() == Some("gzip");
        let mut tar_buf = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_buf);
            for entry in &archive_ir.entries {
                match &entry.entry_type {
                    EntryType::Directory => {
                        let mut header = tar::Header::new_gnu();
                        header.set_path(&entry.path).ok();
                        header.set_entry_type(tar::EntryType::Directory);
                        header.set_size(0);
                        header.set_mode(entry.permissions.unwrap_or(0o755));
                        header.set_mtime(entry.modified.unwrap_or(0));
                        builder.append(&header, std::io::empty()).ok();
                    }
                    EntryType::File => {
                        let data = match &entry.data {
                            EntryData::Inline(d) => d.clone(),
                            EntryData::Reference(path) => std::fs::read(path).unwrap_or_default(),
                            EntryData::Empty => Vec::new(),
                        };
                        let mut header = tar::Header::new_gnu();
                        header.set_path(&entry.path).ok();
                        header.set_entry_type(tar::EntryType::Regular);
                        header.set_size(data.len() as u64);
                        header.set_mode(entry.permissions.unwrap_or(0o644));
                        header.set_mtime(entry.modified.unwrap_or(0));
                        builder.append(&header, std::io::Cursor::new(data)).ok();
                    }
                    _ => {}
                }
            }
            builder.finish().ok();
        }

        let final_bytes = if is_gz {
            use flate2::write::GzEncoder;
            use flate2::Compression;
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            std::io::Write::write_all(&mut encoder, &tar_buf).map_err(|e| PluginError::IoError(e.to_string()))?;
            encoder.finish().map_err(|e| PluginError::IoError(e.to_string()))?
        } else {
            tar_buf
        };

        output.write_all(&final_bytes).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&final_bytes).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: final_bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
