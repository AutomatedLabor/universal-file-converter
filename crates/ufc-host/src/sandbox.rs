use serde::{Deserialize, Serialize};
use ufc_plugin_api::SandboxMode;

/// Resource limits for plugin execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Maximum CPU time in milliseconds.
    pub max_cpu_time_ms: u64,
    /// Maximum disk I/O in bytes.
    pub max_disk_io_bytes: u64,
    /// Maximum output file size in bytes.
    pub max_output_bytes: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024,  // 512 MB
            max_cpu_time_ms: 300_000,               // 5 minutes
            max_disk_io_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            max_output_bytes: 10 * 1024 * 1024 * 1024,  // 10 GB
        }
    }
}

/// Sandbox manager handles plugin isolation.
///
/// For WASM plugins: manages wasmtime instances with memory/timeout limits.
/// For process plugins: manages child processes with resource limits.
/// For in-process plugins: applies timeout limits only.
pub struct SandboxManager {
    default_limits: ResourceLimits,
}

impl SandboxManager {
    pub fn new(default_limits: ResourceLimits) -> Self {
        Self { default_limits }
    }

    /// Get the default resource limits.
    pub fn default_limits(&self) -> &ResourceLimits {
        &self.default_limits
    }

    /// Get resource limits for a specific sandbox mode.
    pub fn limits_for_mode(&self, mode: &SandboxMode) -> ResourceLimits {
        match mode {
            SandboxMode::Wasm => ResourceLimits {
                max_memory_bytes: self.default_limits.max_memory_bytes.min(256 * 1024 * 1024),
                ..self.default_limits.clone()
            },
            SandboxMode::Process => self.default_limits.clone(),
            SandboxMode::InProcess => ResourceLimits {
                max_memory_bytes: self.default_limits.max_memory_bytes,
                max_cpu_time_ms: self.default_limits.max_cpu_time_ms,
                max_disk_io_bytes: self.default_limits.max_disk_io_bytes,
                max_output_bytes: self.default_limits.max_output_bytes,
            },
        }
    }

    /// Validate that a plugin's declared sandbox mode is acceptable.
    pub fn validate_sandbox_mode(&self, mode: &SandboxMode, trusted: bool) -> bool {
        match mode {
            SandboxMode::Wasm => true,
            SandboxMode::Process => true,
            SandboxMode::InProcess => trusted,
        }
    }
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new(ResourceLimits::default())
    }
}
