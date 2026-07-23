use ufc_plugin_api::PluginError;
use thiserror::Error;

/// Core engine errors.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Format detection failed: {reason}")]
    DetectionFailed { reason: String },

    #[error("Unsupported conversion: {source} → {target}")]
    UnsupportedConversion { source: String, target: String },

    #[error("No plugin found for format: {0}")]
    NoPluginFound(String),

    #[error("Plugin error in {plugin_id}: {kind}")]
    PluginError { plugin_id: String, kind: PluginError },

    #[error("IR validation failed: {errors:?}")]
    IrValidationFailed { errors: Vec<String> },

    #[error("Output validation failed: checksum mismatch (expected {expected}, got {actual})")]
    IntegrityCheckFailed { expected: String, actual: String },

    #[error("Resource limit exceeded: {resource} ({limit})")]
    ResourceLimitExceeded { resource: String, limit: String },

    #[error("Conversion cancelled")]
    Cancelled,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Queue error: {0}")]
    QueueError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<PluginError> for CoreError {
    fn from(e: PluginError) -> Self {
        match e {
            PluginError::Cancelled => Self::Cancelled,
            other => Self::PluginError {
                plugin_id: "unknown".to_string(),
                kind: other,
            },
        }
    }
}
