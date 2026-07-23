use semver::Version;

/// Common trait for all intermediate representations.
pub trait IntermediateRepresentation {
    /// The IR schema version.
    fn version(&self) -> Version;

    /// Human-readable name of this IR type (e.g., "Document", "Image").
    fn ir_type(&self) -> &'static str;

    /// Estimated memory usage in bytes.
    fn memory_usage(&self) -> u64;

    /// Validate the IR data. Returns errors if invalid.
    fn validate(&self) -> Vec<ValidationError>;

    /// Serialize to JSON for debugging/caching.
    fn to_json(&self) -> Result<String, serde_json::Error>;

    /// Returns true if this IR contains meaningful data (not empty).
    fn is_empty(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationSeverity {
    Warning,
    Error,
}
