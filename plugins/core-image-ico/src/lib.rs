use std::any::Any;
use ufc_ir::image::*;
use ufc_plugin_api::*;

pub struct IcoPlugin;
impl IcoPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for IcoPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-ico".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "ICO image decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("image/x-icon", &["ico"], "ICO")],
            output_formats: vec![FormatId::new("image/x-icon", &["ico"], "ICO")],
            capabilities: Capabilities { metadata: MetadataSupport::None, structure: StructureSupport::Flat,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![ColorSpace::Rgb, ColorSpace::Rgba],
                max_dimension: Some((256, 256)), max_bit_depth: Some(32), supports_animation: false,
                supports_transparency: true, supports_multi_page: true },
            dependencies: vec![], priority: 50, fidelity_score: 90, known_limitations: vec!["Max 256x256 pixels".to_string()],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 4).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 4 && header[0] == 0x00 && header[1] == 0x00 && header[2] == 0x01 && header[3] == 0x00 {
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("image/x-icon", &["ico"], "ICO"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not an ICO file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let img = image::load_from_memory(&data).map_err(|e| PluginError::DecodeFailed(format!("ICO decode: {}", e)))?;
        let rgba = img.to_rgba8(); let (w, h) = rgba.dimensions();
        let mut ir = ImageIR::new(w, h, ColorSpace::Rgba, BitDepth::U8);
        ir.pixels = PixelData::Raw(rgba.into_raw()); ir.metadata.format_name = "ICO".to_string();
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let image_ir = ir.downcast_ref::<ImageIR>().ok_or_else(|| PluginError::InvalidInput("Expected ImageIR".to_string()))?;
        let pixels = match &image_ir.pixels { PixelData::Raw(d) => d.clone(), _ => return Err(PluginError::EncodeFailed("Only raw pixel data".to_string())) };
        let img = image::RgbaImage::from_raw(image_ir.dimensions.width, image_ir.dimensions.height, pixels)
            .ok_or_else(|| PluginError::EncodeFailed("Invalid dimensions".to_string()))?;
        let mut ico_data = Vec::new();
        { use std::io::Cursor; let mut c = Cursor::new(&mut ico_data);
          img.write_to(&mut c, image::ImageFormat::Ico).map_err(|e| PluginError::EncodeFailed(format!("ICO encode: {}", e)))?; }
        output.write_all(&ico_data).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&ico_data).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: ico_data.len() as u64, checksum, warnings: vec![], fidelity_estimate: 90 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
