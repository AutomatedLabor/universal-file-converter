use std::any::Any;
use ufc_ir::image::*;
use ufc_plugin_api::*;

/// PNG decoder and encoder plugin.
pub struct PngPlugin;

impl PngPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl ConverterPlugin for PngPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-png".to_string(),
            version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0),
            author: "UFC Core Team".to_string(),
            license: "MIT".to_string(),
            description: "PNG image decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("image/png", &["png"], "PNG")],
            output_formats: vec![FormatId::new("image/png", &["png"], "PNG")],
            capabilities: Capabilities {
                metadata: MetadataSupport::ReadWrite,
                structure: StructureSupport::Flat,
                embedded_assets: EmbeddedAssetSupport::None,
                color_spaces: vec![
                    ColorSpace::Gray,
                    ColorSpace::GrayAlpha,
                    ColorSpace::Rgb,
                    ColorSpace::Rgba,
                    ColorSpace::Indexed,
                ],
                max_dimension: Some((65535, 65535)),
                max_bit_depth: Some(16),
                supports_animation: false,
                supports_transparency: true,
                supports_multi_page: false,
            },
            dependencies: vec![],
            priority: 100,
            fidelity_score: 100,
            known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input
            .read_slice(0, 8)
            .map_err(|e| PluginError::IoError(e.to_string()))?;

        let is_png = header.len() >= 8
            && header[0] == 0x89
            && header[1] == 0x50
            && header[2] == 0x4E
            && header[3] == 0x47;

        if is_png {
            Ok(ProbeResult {
                confidence: 100,
                detected_format: FormatId::new("image/png", &["png"], "PNG"),
                format_version: Some("1.2".to_string()),
                estimated_size: Some(input.size()),
                warnings: vec![],
            })
        } else {
            Err(PluginError::UnsupportedFormat("Not a PNG file".to_string()))
        }
    }

    fn decode(
        &self,
        input: &FileReader,
        _config: &DecodeConfig,
        progress: &ProgressCallback,
    ) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));

        let data = input
            .read_all()
            .map_err(|e| PluginError::IoError(e.to_string()))?;

        progress.update(
            ProgressState::new(ConversionPhase::Decoding)
                .with_percent(30.0)
                .with_message("Decoding PNG pixels"),
        );

        let img = image::load_from_memory(&data)
            .map_err(|e| PluginError::DecodeFailed(format!("PNG decode error: {}", e)))?;

        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        progress.update(
            ProgressState::new(ConversionPhase::Decoding)
                .with_percent(80.0)
                .with_message("Building ImageIR"),
        );

        let mut ir = ImageIR::new(width, height, ColorSpace::Rgba, BitDepth::U8);
        ir.pixels = PixelData::Raw(rgba.into_raw());
        ir.alpha = AlphaChannel::Straight;
        ir.metadata.format_name = "PNG".to_string();
        ir.metadata.has_transparency = true;

        // Extract PNG metadata
        if let Some(exif_data) = extract_png_metadata(&data) {
            ir.exif = Some(exif_data);
        }

        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));

        Ok(Box::new(ir))
    }

    fn encode(
        &self,
        ir: &(dyn Any + Send + Sync),
        output: &FileWriter,
        config: &EncodeConfig,
        progress: &ProgressCallback,
    ) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));

        let image_ir = ir
            .downcast_ref::<ImageIR>()
            .ok_or_else(|| PluginError::InvalidInput("Expected ImageIR".to_string()))?;

        progress.update(
            ProgressState::new(ConversionPhase::Encoding)
                .with_percent(30.0)
                .with_message("Encoding PNG"),
        );

        // Get pixel data
        let pixels = match &image_ir.pixels {
            PixelData::Raw(d) => d.clone(),
            _ => {
                return Err(PluginError::EncodeFailed(
                    "Only raw pixel data supported for PNG encoding".to_string(),
                ))
            }
        };

        let width = image_ir.dimensions.width;
        let height = image_ir.dimensions.height;

        // Create image buffer
        let img = image::RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| PluginError::EncodeFailed("Invalid pixel data dimensions".to_string()))?;

        progress.update(
            ProgressState::new(ConversionPhase::Encoding)
                .with_percent(60.0)
                .with_message("Writing PNG file"),
        );

        // Encode to PNG
        let mut png_data = Vec::new();
        {
            use std::io::Cursor;
            let mut cursor = Cursor::new(&mut png_data);
            img.write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|e| PluginError::EncodeFailed(format!("PNG encode error: {}", e)))?;
        }

        progress.update(
            ProgressState::new(ConversionPhase::Encoding)
                .with_percent(90.0)
                .with_message("Writing output"),
        );

        // Write to output
        output
            .write_all(&png_data)
            .map_err(|e| PluginError::IoError(e.to_string()))?;

        let checksum = blake3::hash(&png_data).to_hex().to_string();

        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(100.0));

        Ok(ConversionOutput {
            bytes_written: png_data.len() as u64,
            checksum,
            warnings: vec![],
            fidelity_estimate: 100,
        })
    }

    fn cancel(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

/// Extract basic metadata from PNG chunks.
fn extract_png_metadata(data: &[u8]) -> Option<ExifData> {
    // Look for tEXt chunks for basic metadata
    let mut exif = ExifData {
        make: None,
        model: None,
        software: None,
        datetime: None,
        exposure_time: None,
        f_number: None,
        iso_speed: None,
        focal_length: None,
        orientation: None,
        width: None,
        height: None,
        gps: None,
        custom: std::collections::HashMap::new(),
    };

    // Simple PNG chunk parser
    let mut pos = 8; // Skip PNG signature
    while pos + 12 <= data.len() {
        let length = u32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
        let chunk_type = &data[pos+4..pos+8];

        if chunk_type == b"tEXt" && pos + 12 + length <= data.len() {
            let chunk_data = &data[pos+8..pos+8+length];
            if let Some(null_pos) = chunk_data.iter().position(|&b| b == 0) {
                let key = String::from_utf8_lossy(&chunk_data[..null_pos]).to_string();
                let value = String::from_utf8_lossy(&chunk_data[null_pos+1..]).to_string();
                match key.as_str() {
                    "Software" => exif.software = Some(value),
                    "Creation Time" | "Create Date" => exif.datetime = Some(value),
                    "Author" | "Artist" => exif.make = Some(value),
                    "Description" | "Comment" => { exif.custom.insert(key, value); }
                    _ => { exif.custom.insert(key, value); }
                }
            }
        }

        pos += 12 + length;
    }

    Some(exif)
}

#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn ConverterPlugin {
    Box::into_raw(Box::new(PngPlugin::new()))
}
