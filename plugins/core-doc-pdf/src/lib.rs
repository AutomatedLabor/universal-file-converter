use std::any::Any;
use ufc_ir::document::*;
use ufc_plugin_api::*;

pub struct PdfPlugin;
impl PdfPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for PdfPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-pdf".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "PDF document decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("application/pdf", &["pdf"], "PDF")],
            output_formats: vec![FormatId::new("application/pdf", &["pdf"], "PDF")],
            capabilities: Capabilities { metadata: MetadataSupport::ReadWrite, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::Extract, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: true },
            dependencies: vec![], priority: 100, fidelity_score: 80,
            known_limitations: vec!["Complex layouts may not round-trip perfectly".to_string()],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let header = input.read_slice(0, 5).map_err(|e| PluginError::IoError(e.to_string()))?;
        if header.len() >= 4 && &header[0..4] == b"%PDF" {
            let version = String::from_utf8_lossy(&header[0..]).to_string();
            Ok(ProbeResult { confidence: 100, detected_format: FormatId::new("application/pdf", &["pdf"], "PDF"),
                format_version: Some(version), estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not a PDF file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let doc = lopdf::Document::load_mem(&data).map_err(|e| PluginError::DecodeFailed(format!("PDF decode: {}", e)))?;
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(50.0));

        let mut ir = DocumentIR::new();
        ir.metadata.title = doc.trailer.get(b"Info").ok()
            .and_then(|info| info.as_reference().ok())
            .and_then(|ref_id| doc.get_object(ref_id).ok())
            .and_then(|obj| obj.as_dict().ok())
            .and_then(|dict| dict.get(b"Title").ok())
            .and_then(|val| val.as_string().ok())
            .map(|s| String::from_utf8_lossy(s).to_string());

        // Extract text from each page
        let pages: Vec<u32> = doc.get_pages().into_keys().collect();
        for (i, page_id) in pages.iter().enumerate() {
            progress.update(ProgressState::new(ConversionPhase::Decoding)
                .with_percent(50.0 + (i as f32 / pages.len() as f32) * 40.0));
            if let Ok(text) = doc.extract_text(&[*page_id]) {
                if !text.trim().is_empty() {
                    let mut paragraph = Paragraph { style_id: None, runs: Vec::new() };
                    paragraph.runs.push(InlineRun::Text(TextRun::plain(&text)));
                    ir.content.push(Block::Paragraph(paragraph));
                    if i < pages.len() - 1 {
                        ir.content.push(Block::PageBreak);
                    }
                }
            }
        }
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let doc_ir = ir.downcast_ref::<DocumentIR>().ok_or_else(|| PluginError::InvalidInput("Expected DocumentIR".to_string()))?;

        let (doc, _page, layer) = printpdf::PdfDocument::new(
            doc_ir.metadata.title.as_deref().unwrap_or("Untitled"),
            printpdf::Mm(210.0).into(), printpdf::Mm(297.0).into(), "Layer 1"
        );
        let font = doc.add_builtin_font(printpdf::BuiltinFont::Helvetica)
            .map_err(|e| PluginError::EncodeFailed(format!("Font error: {}", e)))?;

        let current_layer = doc.get_page(_page).get_layer(layer);
        let text = doc_ir.plain_text();
        current_layer.use_text(&text, 12.0, printpdf::Mm(10.0), printpdf::Mm(280.0), &font);

        let pdf_bytes = doc.save_to_bytes().map_err(|e| PluginError::EncodeFailed(format!("PDF save: {}", e)))?;
        output.write_all(&pdf_bytes).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&pdf_bytes).to_hex().to_string();
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(100.0));
        Ok(ConversionOutput { bytes_written: pdf_bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 80 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
