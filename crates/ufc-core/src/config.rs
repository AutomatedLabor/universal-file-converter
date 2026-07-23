use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Maximum concurrent conversions.
    pub max_concurrent: usize,
    /// Maximum memory per conversion (bytes).
    pub max_memory_per_conversion: u64,
    /// Maximum total memory for all conversions (bytes).
    pub max_total_memory: u64,
    /// Auto-retry on failure.
    pub auto_retry: bool,
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Verify output checksums.
    pub verify_output: bool,
    /// Overwrite existing output files.
    pub overwrite_existing: bool,
    /// Temporary directory path.
    pub temp_dir: Option<PathBuf>,
    /// Plugin directory path.
    pub plugin_dir: Option<PathBuf>,
    /// Default output directory (if not specified per conversion).
    pub default_output_dir: Option<PathBuf>,
    /// Log level.
    pub log_level: String,
    /// Enable dark mode (UI setting).
    pub dark_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            max_concurrent: num_cpus::get(),
            max_memory_per_conversion: 512 * 1024 * 1024,  // 512 MB
            max_total_memory: 4 * 1024 * 1024 * 1024,      // 4 GB
            auto_retry: false,
            max_retries: 2,
            verify_output: true,
            overwrite_existing: false,
            temp_dir: None,
            plugin_dir: None,
            default_output_dir: None,
            log_level: "info".to_string(),
            dark_mode: false,
        }
    }
}

impl AppConfig {
    /// Load config from a TOML file, falling back to defaults for missing fields.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to a TOML file.
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the default config file path.
    pub fn default_path() -> PathBuf {
        directories::ProjectDirs::from("com", "ufc", "universal-file-converter")
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("ufc-config.toml"))
    }

    /// Get the default plugin directory.
    pub fn default_plugin_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "ufc", "universal-file-converter")
            .map(|dirs| dirs.data_dir().join("plugins"))
            .unwrap_or_else(|| PathBuf::from("plugins"))
    }

    /// Get the default temp directory.
    pub fn default_temp_dir() -> PathBuf {
        std::env::temp_dir().join("ufc")
    }
}
