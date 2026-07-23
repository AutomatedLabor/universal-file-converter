use crate::traits::{IntermediateRepresentation, ValidationError, ValidationSeverity};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Table / Structured Data Intermediate Representation.
///
/// Covers: CSV, JSON, XML, YAML, TSV
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableIR {
    pub version: Version,
    pub schema: TableSchema,
    pub rows: Vec<TableRow>,
    pub metadata: TableMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub columns: Vec<ColumnDef>,
    pub primary_key: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub index: usize,
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    Time,
    Binary,
    Json,
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub values: Vec<DataValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataValue {
    Null,
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Date(String),
    DateTime(String),
    Time(String),
    Binary(Vec<u8>),
    Json(serde_json::Value),
}

impl DataValue {
    pub fn to_string_lossy(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::String(s) => s.clone(),
            Self::Integer(i) => i.to_string(),
            Self::Float(f) => f.to_string(),
            Self::Boolean(b) => b.to_string(),
            Self::Date(d) => d.clone(),
            Self::DateTime(d) => d.clone(),
            Self::Time(t) => t.clone(),
            Self::Binary(b) => format!("<{} bytes>", b.len()),
            Self::Json(j) => j.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMetadata {
    pub source_format: String,
    pub row_count: Option<usize>,
    pub encoding: Option<String>,
    pub delimiter: Option<char>,
    pub has_header: bool,
    pub custom: HashMap<String, String>,
}

impl TableIR {
    pub fn new() -> Self {
        Self {
            version: crate::api_version(),
            schema: TableSchema {
                columns: Vec::new(),
                primary_key: None,
            },
            rows: Vec::new(),
            metadata: TableMetadata {
                source_format: String::new(),
                row_count: None,
                encoding: None,
                delimiter: None,
                has_header: true,
                custom: HashMap::new(),
            },
        }
    }

    pub fn column_count(&self) -> usize {
        self.schema.columns.len()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

impl Default for TableIR {
    fn default() -> Self { Self::new() }
}

impl IntermediateRepresentation for TableIR {
    fn version(&self) -> Version { self.version.clone() }
    fn ir_type(&self) -> &'static str { "Table" }
    fn memory_usage(&self) -> u64 {
        let row_bytes: u64 = self.rows.iter().map(|r| {
            r.values.iter().map(|v| match v {
                DataValue::String(s) => s.len() as u64 + 32,
                DataValue::Binary(b) => b.len() as u64 + 32,
                DataValue::Json(j) => j.to_string().len() as u64 + 32,
                _ => 32,
            }).sum::<u64>()
        }).sum();
        row_bytes + (self.schema.columns.len() as u64 * 256) + 1024
    }
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.schema.columns.is_empty() {
            errors.push(ValidationError {
                field: "schema.columns".into(),
                message: "No columns defined".into(),
                severity: ValidationSeverity::Error,
            });
        }
        let col_count = self.schema.columns.len();
        for (i, row) in self.rows.iter().enumerate() {
            if row.values.len() != col_count {
                errors.push(ValidationError {
                    field: format!("rows[{}].values", i),
                    message: format!("Expected {} values, got {}", col_count, row.values.len()),
                    severity: ValidationSeverity::Warning,
                });
            }
        }
        errors
    }
    fn to_json(&self) -> Result<String, serde_json::Error> { serde_json::to_string_pretty(self) }
    fn is_empty(&self) -> bool { self.rows.is_empty() }
}
