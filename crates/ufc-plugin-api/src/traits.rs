use crate::config::{DecodeConfig, EncodeConfig};
use crate::error::PluginError;
use crate::io::{FileReader, FileWriter};
use crate::progress::ProgressCallback;
use crate::types::{ConversionOutput, PluginManifest, ProbeResult};
use std::any::Any;

/// The core trait every converter plugin must implement.
///
/// # Lifecycle
///
/// 1. Host calls `manifest()` to get plugin metadata
/// 2. On conversion, host calls `probe()` to confirm the plugin can handle the file
/// 3. Host calls `decode()` to convert source file → IR
/// 4. Host calls `encode()` on a target plugin to convert IR → target format
///
/// # Thread Safety
///
/// Plugins must be `Send + Sync` because the host may run multiple conversions
/// concurrently across different threads.
///
/// # Cancellation
///
/// Plugins should check `progress.is_cancelled()` at natural yield points
/// (per-row, per-frame, per-tile) and return `PluginError::Cancelled` if set.
pub trait ConverterPlugin: Send + Sync {
    /// Returns static metadata about this plugin.
    /// Called once during plugin registration.
    fn manifest(&self) -> PluginManifest;

    /// Probe a file to confirm this plugin can decode it.
    ///
    /// Returns a confidence score (0–100) and detected format details.
    /// The host may call `probe()` on multiple plugins and pick the highest
    /// confidence match.
    fn probe(&self, input: &FileReader) -> Result<ProbeResult, PluginError>;

    /// Decode the source file into an intermediate representation.
    ///
    /// The IR type depends on the format category:
    /// - Images → `ImageIR`
    /// - Documents → `DocumentIR`
    /// - Audio → `AudioIR`
    /// - etc.
    ///
    /// Plugins return the IR as `Box<dyn Any>` because the plugin API crate
    /// cannot depend on `ufc-ir` (to avoid circular dependencies). The host
    /// downcasts to the concrete IR type.
    fn decode(
        &self,
        input: &FileReader,
        config: &DecodeConfig,
        progress: &ProgressCallback,
    ) -> Result<Box<dyn Any + Send + Sync>, PluginError>;

    /// Encode an intermediate representation into the target format.
    ///
    /// The `ir` parameter is `Box<dyn Any>` — plugins downcast to the IR type
    /// they expect. If the IR type is incompatible, return
    /// `PluginError::InvalidInput`.
    fn encode(
        &self,
        ir: &(dyn Any + Send + Sync),
        output: &FileWriter,
        config: &EncodeConfig,
        progress: &ProgressCallback,
    ) -> Result<ConversionOutput, PluginError>;

    /// Cancel a running conversion.
    ///
    /// Must be safe to call from any thread, at any time.
    /// After calling `cancel()`, the next `decode()` or `encode()` check
    /// on the progress callback should return `PluginError::Cancelled`.
    fn cancel(&self) -> Result<(), PluginError>;

    /// Returns true if this plugin can decode the given format.
    fn can_decode(&self, format: &crate::types::FormatId) -> bool {
        self.manifest()
            .input_formats
            .iter()
            .any(|f| f.mime == format.mime)
    }

    /// Returns true if this plugin can encode the given format.
    fn can_encode(&self, format: &crate::types::FormatId) -> bool {
        self.manifest()
            .output_formats
            .iter()
            .any(|f| f.mime == format.mime)
    }
}
