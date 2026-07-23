use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that plugins can return.
///
/// These are caught by the host and translated into user-facing messages.
/// Plugin errors never crash the host application.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum PluginError {
    #[error("Decode failed: {0}")]
    DecodeFailed(String),

    #[error("Encode failed: {0}")]
    EncodeFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Conversion cancelled")]
    Cancelled,

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Out of memory (limit: {limit_mb}MB, requested: {requested_mb}MB)")]
    OutOfMemory { limit_mb: u64, requested_mb: u64 },

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Plugin crashed: {0}")]
    Crashed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Dependency missing: {name} (required: {version_req})")]
    DependencyMissing {
        name: String,
        version_req: String,
    },
}

/// Validation error for IR data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Warning,
    Error,
}
