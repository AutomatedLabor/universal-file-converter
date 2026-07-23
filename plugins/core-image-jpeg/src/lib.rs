use std::any::Any;
use ufc_ir::image::*;
use ufc_plugin_api::*;

pub struct JpegPlugin;

impl JpegPlugin {
    pub fn new() -> Self { Self }
}

impl ConverterPlugin for JpegPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-jpeg".to_string(),
            version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0),
            author: "UFC Core Team".to_string(),
            license: "MIT".to_string(),
            description: "JPEG image decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("image/jpeg", &["jpg", "jpeg"], "JPEG")],
            output_formats: vec![FormatId::new("image/jpeg", &["jpg", "jpeg"], "JPEG")],
            capabilities: Capabilities {
                metadata: MetadataSupport::ReadWriteTransform,
                structure: StructureSupport::Flat,
                embedded_assets: EmbeddedAssetSupport::None,
                color_spaces: vec![ColorSpace::Rgb, ColorSpace::Rgba, ColorSpace::Cmyk, ColorSpace::YCbCr],
                max_dimension: Some((65535, 65535)),
                max_bit_depth: Some(8),
                supports_animation: false,
                supports_transparency: false,
                supports_multi_page: false,
            },
            dependencies: vec![],
            priority: 100,
            fidelity_score: 90,
            known_limitations: vec!["Lossy compression — not suitable for pixel-perfect conversion".to_string()],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 3).map_err(|e| PluginError::IoError(e.to_string()))?;
        let is_jpeg = header.len() >= 3 && header[0] == 0xFF && header[1] == 0xD8 && header[2] == 0xFF;
        if is_jpeg {
            Ok(ProbeResult {
                confidence: 100,
                detected_format: FormatId::new("image/jpeg", &["jpg", "jpeg"], "JPEG"),
                format_version: Some("JFIF/EXIF".to_string()),
                estimated_size: Some(input.size()),
                warnings: vec![],
            })
        } else {
            Err(PluginError::UnsupportedFormat("Not a JPEG file".to_string()))
        }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(30.0));
        let img = image::load_from_memory(&data).map_err(|e| PluginError::DecodeFailed(format!("JPEG decode error: {}", e)))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(80.0));
        let mut ir = ImageIR::new(width, height, ColorSpace::Rgba, BitDepth::U8);
        ir.pixels = PixelData::Raw(rgba.into_raw());
        ir.alpha = AlphaChannel::None;
        ir.metadata.format_name = "JPEG".to_string();
        ir.metadata.has_transparency = false;
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let image_ir = ir.downcast_ref::<ImageIR>().ok_or_else(|| PluginError::InvalidInput("Expected ImageIR".to_string()))?;
        let pixels = match &image_ir.pixels {
            PixelData::Raw(d) => d.clone(),
            _ => return Err(PluginError::EncodeFailed("Only raw pixel data supported".to_string())),
        };
        let width = image_ir.dimensions.width;
        let height = image_ir.dimensions.height;
        let img = image::RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| PluginError::EncodeFailed("Invalid pixel data dimensions".to_string()))?;
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(50.0));
        // Convert RGBA to RGB for JPEG (no alpha)
        let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
        let quality = config.quality.to_jpeg_quality();
        let mut jpeg_data = Vec::new();
        {
            use std::io::Cursor;
            let mut cursor = Cursor::new(&mut jpeg_data);
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
            rgb_img.write_with_encoder(encoder)
                .map_err(|e| PluginError::EncodeFailed(format!("JPEG encode error: {}", e)))?;
        }
        output.write_all(&jpeg_data).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&jpeg_data).to_hex().to_string();
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(100.0));
        Ok(ConversionOutput { bytes_written: jpeg_data.len() as u64, checksum, warnings: vec![], fidelity_estimate: 90 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
