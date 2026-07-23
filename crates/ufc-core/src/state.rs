use crate::queue::ConversionQueue;
use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persistent state manager for the conversion queue and history.
///
/// Persists queue state to disk so conversions can resume after app restart.
pub struct StateManager {
    state_path: PathBuf,
    state: AppState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub queue: ConversionQueue,
    pub history: Vec<HistoryEntry>,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub source_format: String,
    pub target_format: String,
    pub success: bool,
    pub bytes_written: Option<u64>,
    pub duration_ms: Option<u64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub error: Option<String>,
}

impl StateManager {
    /// Create a new state manager, loading existing state from disk if available.
    pub fn new(state_path: PathBuf) -> Self {
        let state = Self::load_from_disk(&state_path).unwrap_or(AppState {
            queue: ConversionQueue::default(),
            history: Vec::new(),
            version: 1,
        });
        Self { state_path, state }
    }

    /// Get a reference to the current app state.
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Get a mutable reference to the current app state.
    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    /// Save current state to disk.
    pub fn save(&self) -> Result<(), CoreError> {
        let json = serde_json::to_string_pretty(&self.state)
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.state_path, json)?;
        Ok(())
    }

    /// Add an entry to the conversion history.
    pub fn add_history(&mut self, entry: HistoryEntry) {
        self.state.history.push(entry);
        // Keep only the last 1000 entries
        if self.state.history.len() > 1000 {
            let drain_count = self.state.history.len() - 1000;
            self.state.history.drain(0..drain_count);
        }
    }

    /// Search history by filename or format.
    pub fn search_history(&self, query: &str) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();
        self.state.history.iter().filter(|e| {
            e.input_path.to_string_lossy().to_lowercase().contains(&query_lower)
                || e.output_path.to_string_lossy().to_lowercase().contains(&query_lower)
                || e.source_format.to_lowercase().contains(&query_lower)
                || e.target_format.to_lowercase().contains(&query_lower)
        }).collect()
    }

    /// Clear all history.
    pub fn clear_history(&mut self) {
        self.state.history.clear();
    }

    /// Load state from a JSON file on disk.
    fn load_from_disk(path: &PathBuf) -> Option<AppState> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }
}

/// Default state file path.
pub fn default_state_path() -> PathBuf {
    directories::ProjectDirs::from("com", "ufc", "universal-file-converter")
        .map(|dirs| dirs.data_dir().join("state.json"))
        .unwrap_or_else(|| PathBuf::from("ufc-state.json"))
}
