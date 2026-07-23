use std::any::Any;
use std::io::Read;
use ufc_ir::table::*;
use ufc_plugin_api::*;
use quick_xml::Reader;
use quick_xml::events::Event;

pub struct XmlPlugin;
impl XmlPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for XmlPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-xml".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "XML structured data decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("application/xml", &["xml"], "XML")],
            output_formats: vec![FormatId::new("application/xml", &["xml"], "XML")],
            capabilities: Capabilities { metadata: MetadataSupport::None, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 90, fidelity_score: 90, known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let ext = input.path().extension().map(|e| e.to_string_lossy().to_lowercase());
        if ext.as_deref() == Some("xml") {
            Ok(ProbeResult { confidence: 80, detected_format: FormatId::new("application/xml", &["xml"], "XML"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not an XML file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let content = String::from_utf8_lossy(&data);

        let mut ir = TableIR::new();
        ir.metadata.source_format = "XML".to_string();

        let mut reader = Reader::from_str(&content);
        let mut buf = Vec::new();
        let mut current_element = String::new();
        let mut depth = 0;
        let mut row_values: Vec<DataValue> = Vec::new();
        let mut columns_set = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if depth == 2 && !columns_set {
                        ir.schema.columns.push(ColumnDef {
                            index: ir.schema.columns.len(), name: name.clone(),
                            data_type: DataType::String, nullable: true, description: None,
                        });
                    }
                    current_element = name;
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape().map(|s| s.to_string()).unwrap_or_default();
                    if depth >= 2 && !text.trim().is_empty() {
                        row_values.push(DataValue::String(text));
                        if !columns_set && depth == 2 {
                            // Column names from first record
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    if depth == 2 && !row_values.is_empty() {
                        ir.rows.push(TableRow { values: std::mem::take(&mut row_values) });
                        columns_set = true;
                    }
                    depth -= 1;
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        ir.metadata.row_count = Some(ir.rows.len());
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let table_ir = ir.downcast_ref::<TableIR>().ok_or_else(|| PluginError::InvalidInput("Expected TableIR".to_string()))?;

        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<records>\n");
        for row in &table_ir.rows {
            xml.push_str("  <record>\n");
            for (col, val) in table_ir.schema.columns.iter().zip(row.values.iter()) {
                let escaped = xml_escape(&val.to_string_lossy());
                xml.push_str(&format!("    <{}>{}</{}>\n", col.name, escaped, col.name));
            }
            xml.push_str("  </record>\n");
        }
        xml.push_str("</records>\n");

        let bytes = xml.as_bytes();
        output.write_all(bytes).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(bytes).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 90 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
