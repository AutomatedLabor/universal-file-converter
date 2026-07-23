use std::path::{Path, PathBuf};
use crate::error::CoreError;

/// Manages temporary files and directories used during conversion.
///
/// Ensures temp files are cleaned up after conversion completes or fails.
pub struct TempManager {
    base_dir: PathBuf,
    active_dirs: Vec<PathBuf>,
}

impl TempManager {
    pub fn new(base_dir: Option<PathBuf>) -> Result<Self, CoreError> {
        let base_dir = base_dir.unwrap_or_else(|| {
            std::env::temp_dir().join("ufc")
        });
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self {
            base_dir,
            active_dirs: Vec::new(),
        })
    }

    /// Create a new temporary directory for a conversion.
    pub fn create_session_dir(&mut self) -> Result<PathBuf, CoreError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let dir = self.base_dir.join(&session_id);
        std::fs::create_dir_all(&dir)?;
        self.active_dirs.push(dir.clone());
        Ok(dir)
    }

    /// Create a temporary file path within a session directory.
    pub fn temp_file(&self, session_dir: &Path, name: &str) -> PathBuf {
        session_dir.join(name)
    }

    /// Clean up a specific session directory.
    pub fn cleanup(&mut self, session_dir: &Path) -> Result<(), CoreError> {
        if session_dir.exists() {
            std::fs::remove_dir_all(session_dir)?;
        }
        self.active_dirs.retain(|d| d != session_dir);
        Ok(())
    }

    /// Clean up all active session directories.
    pub fn cleanup_all(&mut self) -> Result<(), CoreError> {
        for dir in self.active_dirs.drain(..) {
            if dir.exists() {
                std::fs::remove_dir_all(&dir).ok();
            }
        }
        Ok(())
    }

    /// Get the base temp directory.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Get the number of active sessions.
    pub fn active_session_count(&self) -> usize {
        self.active_dirs.len()
    }
}

impl Drop for TempManager {
    fn drop(&mut self) {
        self.cleanup_all().ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_cleanup() {
        let mut manager = TempManager::new(None).unwrap();
        let dir = manager.create_session_dir().unwrap();
        assert!(dir.exists());
        assert_eq!(manager.active_session_count(), 1);

        manager.cleanup(&dir).unwrap();
        assert!(!dir.exists());
        assert_eq!(manager.active_session_count(), 0);
    }
}
