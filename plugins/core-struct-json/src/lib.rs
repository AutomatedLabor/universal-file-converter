use std::any::Any;
use ufc_ir::table::*;
use ufc_plugin_api::*;

pub struct JsonPlugin;
impl JsonPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for JsonPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-json".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "JSON structured data decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("application/json", &["json"], "JSON")],
            output_formats: vec![FormatId::new("application/json", &["json"], "JSON")],
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
        if ext.as_deref() == Some("json") {
            Ok(ProbeResult { confidence: 80, detected_format: FormatId::new("application/json", &["json"], "JSON"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] })
        } else { Err(PluginError::UnsupportedFormat("Not a JSON file".to_string())) }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let value: serde_json::Value = serde_json::from_slice(&data)
            .map_err(|e| PluginError::DecodeFailed(format!("JSON parse: {}", e)))?;

        let mut ir = TableIR::new();
        ir.metadata.source_format = "JSON".to_string();

        match value {
            serde_json::Value::Array(arr) => {
                if let Some(first) = arr.first() {
                    if let serde_json::Value::Object(obj) = first {
                        for (i, (key, _)) in obj.iter().enumerate() {
                            ir.schema.columns.push(ColumnDef {
                                index: i, name: key.clone(), data_type: DataType::Json,
                                nullable: true, description: None,
                            });
                        }
                    }
                }
                for item in &arr {
                    if let serde_json::Value::Object(obj) = item {
                        let values: Vec<DataValue> = ir.schema.columns.iter()
                            .map(|col| obj.get(&col.name)
                                .map(|v| DataValue::Json(v.clone()))
                                .unwrap_or(DataValue::Null))
                            .collect();
                        ir.rows.push(TableRow { values });
                    }
                }
            }
            serde_json::Value::Object(obj) => {
                for (i, (key, val)) in obj.iter().enumerate() {
                    ir.schema.columns.push(ColumnDef {
                        index: i, name: key.clone(), data_type: DataType::Json,
                        nullable: true, description: None,
                    });
                    ir.rows.push(TableRow { values: vec![DataValue::Json(val.clone())] });
                }
            }
            _ => {
                ir.schema.columns.push(ColumnDef { index: 0, name: "value".to_string(), data_type: DataType::Json, nullable: false, description: None });
                ir.rows.push(TableRow { values: vec![DataValue::Json(value)] });
            }
        }

        ir.metadata.row_count = Some(ir.rows.len());
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(100.0));
        Ok(Box::new(ir))
    }

    fn encode(&self, ir: &(dyn Any + Send + Sync), output: &FileWriter, _config: &EncodeConfig, progress: &ProgressCallback) -> Result<ConversionOutput, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Encoding).with_percent(0.0));
        let table_ir = ir.downcast_ref::<TableIR>().ok_or_else(|| PluginError::InvalidInput("Expected TableIR".to_string()))?;

        let mut records = Vec::new();
        for row in &table_ir.rows {
            let mut obj = serde_json::Map::new();
            for (col, val) in table_ir.schema.columns.iter().zip(row.values.iter()) {
                let json_val = match val {
                    DataValue::Null => serde_json::Value::Null,
                    DataValue::String(s) => serde_json::Value::String(s.clone()),
                    DataValue::Integer(i) => serde_json::Value::Number((*i).into()),
                    DataValue::Float(f) => serde_json::json!(f),
                    DataValue::Boolean(b) => serde_json::Value::Bool(*b),
                    DataValue::Json(v) => v.clone(),
                    _ => serde_json::Value::String(val.to_string_lossy()),
                };
                obj.insert(col.name.clone(), json_val);
            }
            records.push(serde_json::Value::Object(obj));
        }

        let json_bytes = serde_json::to_vec_pretty(&records)
            .map_err(|e| PluginError::EncodeFailed(format!("JSON serialize: {}", e)))?;
        output.write_all(&json_bytes).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(&json_bytes).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: json_bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
