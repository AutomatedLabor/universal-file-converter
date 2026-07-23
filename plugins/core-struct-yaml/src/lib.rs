use std::any::Any;
use ufc_ir::table::*;
use ufc_plugin_api::*;

pub struct YamlPlugin;
impl YamlPlugin { pub fn new() -> Self { Self } }

impl ConverterPlugin for YamlPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "core-yaml".to_string(), version: semver::Version::new(1, 0, 0),
            api_version: semver::Version::new(1, 0, 0), author: "UFC Core Team".to_string(),
            license: "MIT".to_string(), description: "YAML structured data decoder and encoder".to_string(),
            input_formats: vec![FormatId::new("application/x-yaml", &["yaml", "yml"], "YAML")],
            output_formats: vec![FormatId::new("application/x-yaml", &["yaml", "yml"], "YAML")],
            capabilities: Capabilities { metadata: MetadataSupport::None, structure: StructureSupport::Hierarchical,
                embedded_assets: EmbeddedAssetSupport::None, color_spaces: vec![],
                max_dimension: None, max_bit_depth: None, supports_animation: false,
                supports_transparency: false, supports_multi_page: false },
            dependencies: vec![], priority: 90, fidelity_score: 100, known_limitations: vec![],
            sandbox_mode: SandboxMode::Wasm,
        }
    }

    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError> {
        let ext = input.path().extension().map(|e| e.to_string_lossy().to_lowercase());
        match ext.as_deref() {
            Some("yaml") | Some("yml") => Ok(ProbeResult { confidence: 80,
                detected_format: FormatId::new("application/x-yaml", &["yaml", "yml"], "YAML"),
                format_version: None, estimated_size: Some(input.size()), warnings: vec![] }),
            _ => Err(PluginError::UnsupportedFormat("Not a YAML file".to_string())),
        }
    }

    fn decode(&self, input: &FileReader, _config: &DecodeConfig, progress: &ProgressCallback) -> Result<Box<dyn Any + Send + Sync>, PluginError> {
        progress.update(ProgressState::new(ConversionPhase::Decoding).with_percent(0.0));
        let data = input.read_all().map_err(|e| PluginError::IoError(e.to_string()))?;
        let content = String::from_utf8_lossy(&data);

        let value: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| PluginError::DecodeFailed(format!("YAML parse: {}", e)))?;

        let mut ir = TableIR::new();
        ir.metadata.source_format = "YAML".to_string();

        match value {
            serde_yaml::Value::Sequence(seq) => {
                if let Some(serde_yaml::Value::Mapping(first)) = seq.first() {
                    for (i, (key, _)) in first.iter().enumerate() {
                        ir.schema.columns.push(ColumnDef {
                            index: i, name: key.as_str().unwrap_or(&format!("col_{}", i)).to_string(),
                            data_type: DataType::String, nullable: true, description: None,
                        });
                    }
                }
                for item in &seq {
                    if let serde_yaml::Value::Mapping(map) = item {
                        let values: Vec<DataValue> = ir.schema.columns.iter()
                            .map(|col| map.get(&serde_yaml::Value::String(col.name.clone()))
                                .map(|v| DataValue::Json(serde_json::to_value(v).unwrap_or(serde_json::Value::Null)))
                                .unwrap_or(DataValue::Null))
                            .collect();
                        ir.rows.push(TableRow { values });
                    }
                }
            }
            serde_yaml::Value::Mapping(map) => {
                for (i, (key, val)) in map.iter().enumerate() {
                    ir.schema.columns.push(ColumnDef {
                        index: i, name: key.as_str().unwrap_or(&format!("key_{}", i)).to_string(),
                        data_type: DataType::String, nullable: true, description: None,
                    });
                    ir.rows.push(TableRow { values: vec![
                        DataValue::Json(serde_json::to_value(val).unwrap_or(serde_json::Value::Null))
                    ] });
                }
            }
            _ => {
                ir.schema.columns.push(ColumnDef { index: 0, name: "value".to_string(), data_type: DataType::String, nullable: false, description: None });
                ir.rows.push(TableRow { values: vec![DataValue::String(serde_yaml::to_string(&value).unwrap_or_default())] });
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
            let mut map = serde_yaml::Mapping::new();
            for (col, val) in table_ir.schema.columns.iter().zip(row.values.iter()) {
                let yaml_val = match val {
                    DataValue::Null => serde_yaml::Value::Null,
                    DataValue::String(s) => serde_yaml::Value::String(s.clone()),
                    DataValue::Integer(i) => serde_yaml::Value::Number((*i).into()),
                    DataValue::Float(f) => serde_yaml::Value::Real(f.to_string()),
                    DataValue::Boolean(b) => serde_yaml::Value::Bool(*b),
                    _ => serde_yaml::Value::String(val.to_string_lossy()),
                };
                map.insert(serde_yaml::Value::String(col.name.clone()), yaml_val);
            }
            records.push(serde_yaml::Value::Mapping(map));
        }

        let yaml_str = serde_yaml::to_string(&records)
            .map_err(|e| PluginError::EncodeFailed(format!("YAML serialize: {}", e)))?;
        let bytes = yaml_str.as_bytes();
        output.write_all(bytes).map_err(|e| PluginError::IoError(e.to_string()))?;
        let checksum = blake3::hash(bytes).to_hex().to_string();
        Ok(ConversionOutput { bytes_written: bytes.len() as u64, checksum, warnings: vec![], fidelity_estimate: 100 })
    }

    fn cancel(&self) -> Result<(), PluginError> { Ok(()) }
}
