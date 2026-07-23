use std::any::Any;
use ufc_ir::table::*;
use ufc_plugin_api::*;

pub struct CsvPlugin;
impl CsvPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for CsvPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-csv".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "CSV structured data decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("text/csv", &["csv", "tsv"], "CSV")],
            output_formats: vec![FormatId::new("text/csv", &["csv", "tsv"], "CSV")],
            capabilities: Capabilities { metadata: MetadataSupport::None, structure: StructureSupport::Relational,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 100, fidelity_score: 100, known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let ext = input.path().extension().map(|e| e.to_string_lossy().to_lowercase());
        match ext.as_deref() {
            Some("csv") | Some("tsv") => Ok(ProbeResult { confidence: 80,
                detected_format: FormatId::new("text/csv", &["csv", "tsv"], "CSV"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] }),
            _ => Err(PluginError::UnsupportedFormat("Not a CSV file".to_string())),
        }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let content = String::from_utf8_lossy(&data);

        let ext = input.path().extension().map(|e| e.to_string_lossy().to_lowercase());
        let delimiter = if ext.as_deref() == Some("tsv") { b'\t' } else { b',' };

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(true)
            .from_reader(content.as_bytes());

        let mut ir = TableIR::new();
        ir.metadata.source_format = "CSV".to_string();
        ir.metadata.has_header = true;
        ir.metadata.delimiter = Some(delimiter as char);

        // Read headers
        let headers: Vec<String> = reader.headers()
            .map(|h| h.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        for (i, name) in headers.iter().enumerate() {
            ir.schema.columns.push(ColumnDef {
                index: i,
                name: name.clone(),
                data_type: DataType::String,
                nullable: true,
                description: None,
            });
        }

        // Read rows
        for (i, result) in reader.records().enumerate() {
            match result {
                Ok(record) => {
                    let values: Vec<DataValue> = record.iter()
                        .map(|field| {
                            if field.is_empty() { DataValue::Null }
                            else { DataValue::String(field.to_string()) }
                        })
                        .collect();
                    ir.rows.push(TableRow { values });
                }
                Err(_) => continue,
            }
            if i % 1000 == 0 {
                progress.update(ProgressState::new(ConversionPhase::Decoding)
                    .with_percent(i as f32 / 10000.0 * 100.0));
            }
        }

        ir.metadata.row_count = Some(ir.rows.len());
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let table_ir = ir.downcast_ref::<TableIR>().ok_or_else(|| PluginError::InvalidInput("Expected TableIR".to_string()))?;

        let delimiter = table_ir.metadata.delimiter.unwrap_or(',');
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(delimiter as u8)
            .from_writer(Vec::new());

        // Write headers
        let headers: Vec<&str> = table_ir.schema.columns.iter().map(|c| c.name.as_str()).collect();
        wtr.write_record(&headers).map_err(|e| PluginError::IoError(e.to_string()))?;

        // Write rows
        for (i, row) in table_ir.rows.iter().enumerate() {
            let fields: Vec<String> = row.values.iter().map(|v| v.to_string_lossy()).collect();
            wtr.write_record(&fields).map_err(|e| PluginError::IoError(e.to_string()))?;
            if i % 1000 == 0 {
                progress.update(ProgressState::new(ConversionPhase::Encoding)
                    .with_percent(i as f32 / table_ir.rows.len() as f32 * 100.0));
            }
        }

        let csv_bytes = wtr.into_inner().map_err(|e| PluginError::IoError(e.to_string()))?;
        output.write_all(&csv_bytes).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&csv_bytes).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: csv_bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
