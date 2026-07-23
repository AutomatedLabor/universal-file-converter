use crate::traits::{IntermediateRepresentation, ValidationError, ValidationSeverity};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Archive Intermediate Representation.
///
/// Covers: ZIP, TAR, TAR.GZ, TAR.BZ2, TAR.XZ, 7Z
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveIR {
    pub version: Version,
    pub entries: Vec<ArchiveEntry>,
    pub metadata: ArchiveMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub path: String,
    pub entry_type: EntryType,
    pub size: u64,
    pub compressed_size: Option<u64>,
    pub permissions: Option<u32>,
    pub modified: Option<u64>, // unix timestamp
    pub created: Option<u64>,
    pub data: EntryData,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryType {
    File,
    Directory,
    Symlink { target: String },
    Hardlink { target: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryData {
    Inline(Vec<u8>),
    Reference(String), // path to extracted temp file
    Empty,             // for directories
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    pub format: String,
    pub compression: Option<String>,
    pub total_size: u64,
    pub total_compressed_size: u64,
    pub entry_count: usize,
    pub comment: Option<String>,
    pub encrypted: bool,
}

impl ArchiveIR {
    pub fn new(format: &str) -> Self {
        Self {
            version: crate::api_version(),
            entries: Vec::new(),
            metadata: ArchiveMetadata {
                format: format.to_string(),
                compression: None,
                total_size: 0,
                total_compressed_size: 0,
                entry_count: 0,
                comment: None,
                encrypted: false,
            },
        }
    }

    pub fn add_entry(&mut self, entry: ArchiveEntry) {
        self.metadata.total_size += entry.size;
        self.metadata.total_compressed_size += entry.compressed_size.unwrap_or(entry.size);
        self.metadata.entry_count += 1;
        self.entries.push(entry);
    }

    pub fn file_count(&self) -> usize {
        self.entries.iter().filter(|e| matches!(e.entry_type, EntryType::File)).count()
    }

    pub fn directory_count(&self) -> usize {
        self.entries.iter().filter(|e| matches!(e.entry_type, EntryType::Directory)).count()
    }
}

impl IntermediateRepresentation for ArchiveIR {
    fn version(&self) -> Version { self.version.clone() }
    fn ir_type(&self) -> &'static str { "Archive" }
    fn memory_usage(&self) -> u64 {
        let data_bytes: u64 = self.entries.iter().map(|e| match &e.data {
            EntryData::Inline(d) => d.len() as u64,
            _ => 0,
        }).sum();
        data_bytes + (self.entries.len() as u64 * 512) + 1024
    }
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.entries.is_empty() {
            errors.push(ValidationError {
                field: "entries".into(),
                message: "Archive has no entries".into(),
                severity: ValidationSeverity::Warning,
            });
        }
        errors
    }
    fn to_json(&self) -> Result<String, serde_json::Error> { serde_json::to_string_pretty(self) }
    fn is_empty(&self) -> bool { self.entries.is_empty() }
}
