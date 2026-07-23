use std::any::Any;
use ufc_ir::image::*;
use ufc_plugin_api::*;

pub struct BmpPlugin;
impl BmpPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for BmpPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-bmp".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "BMP image decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("image/bmp", &["bmp"], "BMP")],
            output_formats: vec![FormatId::new("image/bmp", &["bmp"], "BMP")],
            capabilities: Capabilities { metadata: MetadataSupport::None, structure: StructureSupport::Flat,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![ColorSpace::Rgb, ColorSpace::Rgba],
                max_dimension: None, max_bit_depth: Some(32), supports_animation: false,
                supports_transparency: true, supports_multi_page: false },
            dependencies: vec![], priority: 50, fidelity_score: 100, known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 2).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 2 && header[0] == b'B' && header[1] == b'M' {
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("image/bmp", &["bmp"], "BMP"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not a BMP file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let img = image::load_from_memory(&data).map_err(|e| PluginError::DecodeFailed(format!("BMP decode: {}", e)))?;
        let rgba = img.to_rgba8(); let (w, h) = rgba.dimensions();
        let mut ir = ImageIR::new(w, h, ColorSpace::Rgba, BitDepth::U8);
        ir.pixels = PixelData::Raw(rgba.into_raw()); ir.metadata.format_name = "BMP".to_string();
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let image_ir = ir.downcast_ref::<ImageIR>().ok_or_else(|| PluginError::InvalidInput("Expected ImageIR".to_string()))?;
        let pixels = match &image_ir.pixels { PixelData::Raw(d) => d.clone(), _ => return Err(PluginError::EncodeFailed("Only raw pixel data".to_string())) };
        let img = image::RgbaImage::from_raw(image_ir.dimensions.width, image_ir.dimensions.height, pixels)
            .ok_or_else(|| PluginError::EncodeFailed("Invalid dimensions".to_string()))?;
        let mut bmp_data = Vec::new();
        { use std::io::Cursor; let mut c = Cursor::new(&mut bmp_data);
          img.write_to(&mut c, image::ImageFormat::Bmp).map_err(|e| PluginError::EncodeFailed(format!("BMP encode: {}", e)))?; }
        output.write_all(&bmp_data).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&bmp_data).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: bmp_data.len() as u64, checksum, warnings: vec![], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
