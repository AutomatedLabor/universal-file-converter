use std::any::Any;
use ufc_ir::image::*;
use ufc_plugin_api::*;

pub struct AvifPlugin;
impl AvifPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for AvifPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-avif".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "AVIF image decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("image/avif", &["avif"], "AVIF")],
            output_formats: vec![FormatId::new("image/avif", &["avif"], "AVIF")],
            capabilities: Capabilities { metadata: MetadataSupport::ReadWrite, structure: StructureSupport::Flat,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![ColorSpace::Rgb, ColorSpace::Rgba],
                max_dimension: None, max_bit_depth: Some(12), supports_animation: true,
                supports_transparency: true, supports_multi_page: false },
            dependencies: vec![], priority: 85, fidelity_score: 95, known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        // AVIF uses ISOBMFF container; check for ftyp box with avif brand
        let header = input.read_slice(0, 32).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 12 && &header[4..8] == b"ftyp" {
            let brand = &header[8..12];
            if brand == b"avif" || brand == b"avis" {
                return Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("image/avif", &["avif"], "AVIF"),
                    format_version: None, estimated_size: Some(input.size()), warnings: vec![] });
            }
        }
        Err(PluginError::UnsupportedFormat("Not an AVIF file".to_string()))
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let img = image::load_from_memory(&data).map_err(|e| PluginError::DecodeFailed(format!("AVIF decode: {}", e)))?;
        let rgba = img.to_rgba8(); let (w, h) = rgba.dimensions();
        let mut ir = ImageIR::new(w, h, ColorSpace::Rgba, BitDepth::U8);
        ir.pixels = PixelData::Raw(rgba.into_raw()); ir.metadata.format_name = "AVIF".to_string();
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let image_ir = ir.downcast_ref::<ImageIR>().ok_or_else(|| PluginError::InvalidInput("Expected ImageIR".to_string()))?;
        let pixels = match &image_ir.pixels { PixelData::Raw(d) => d.clone(), _ => return Err(PluginError::EncodeFailed("Only raw pixel data".to_string())) };
        let img = image::RgbaImage::from_raw(image_ir.dimensions.width, image_ir.dimensions.height, pixels)
            .ok_or_else(|| PluginError::EncodeFailed("Invalid dimensions".to_string()))?;
        let mut avif_data = Vec::new();
        { use std::io::Cursor; let mut c = Cursor::new(&mut avif_data);
          img.write_to(&mut c, image::ImageFormat::Avif).map_err(|e| PluginError::EncodeFailed(format!("AVIF encode: {}", e)))?; }
        output.write_all(&avif_data).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&avif_data).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: avif_data.len() as u64, checksum, warnings: vec![], fidelity_estimate: 95 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
