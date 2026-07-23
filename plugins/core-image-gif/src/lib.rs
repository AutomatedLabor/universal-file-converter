use std::any::Any;
use ufc_ir::image::*;
use ufc_plugin_api::*;

pub struct GifPlugin;
impl GifPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for GifPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-gif".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "GIF image decoder and encoder (animated)".to_string(),
            input_formats: vec![FormatId::new("image/gif", &["gif"], "GIF")],
            output_formats: vec![FormatId::new("image/gif", &["gif"], "GIF")],
            capabilities: Capabilities { metadata: MetadataSupport::None, structure: StructureSupport::Flat,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![ColorSpace::Indexed],
                max_dimension: Some((65535, 65535)), max_bit_depth: Some(8), supports_animation: true,
                supports_transparency: true, supports_multi_page: false },
            dependencies: vec![], priority: 90, fidelity_score: 100,
            known_limitations: vec!["Max 256 colors per frame".to_string()],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 6).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 6 && (&header[0..6] == b"GIF87a" || &header[0..6] == b"GIF89a") {
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("image/gif", &["gif"], "GIF"),
                format_version: Some(String::from_utf8_lossy(&header[0..6]).to_string()),
                estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not a GIF file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let img = image::load_from_memory(&data).map_err(|e| PluginError::DecodeFailed(format!("GIF decode: {}", e)))?;
        let rgba = img.to_rgba8(); let (w, h) = rgba.dimensions();
        let mut ir = ImageIR::new(w, h, ColorSpace::Rgba, BitDepth::U8);
        ir.pixels = PixelData::Raw(rgba.into_raw()); ir.metadata.format_name = "GIF".to_string();
        ir.metadata.has_transparency = true;
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let image_ir = ir.downcast_ref::<ImageIR>().ok_or_else(|| PluginError::InvalidInput("Expected ImageIR".to_string()))?;
        let pixels = match &image_ir.pixels { PixelData::Raw(d) => d.clone(), _ => return Err(PluginError::EncodeFailed("Only raw pixel data".to_string())) };
        let img = image::RgbaImage::from_raw(image_ir.dimensions.width, image_ir.dimensions.height, pixels)
            .ok_or_else(|| PluginError::EncodeFailed("Invalid dimensions".to_string()))?;
        let mut gif_data = Vec::new();
        { use std::io::Cursor; let mut c = Cursor::new(&mut gif_data);
          img.write_to(&mut c, image::ImageFormat::Gif).map_err(|e| PluginError::EncodeFailed(format!("GIF encode: {}", e)))?; }
        output.write_all(&gif_data).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&gif_data).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: gif_data.len() as u64, checksum, warnings: vec![], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
