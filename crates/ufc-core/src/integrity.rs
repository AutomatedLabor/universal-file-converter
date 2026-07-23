use crate::error::CoreError;
use std::path::Path;

/// Integrity checker for verifying converted files.
///
/// Uses Blake3 for fast, collision-resistant checksums.
pub struct IntegrityChecker {
    verify_output: bool,
}

impl IntegrityChecker {
    pub fn new(verify_output: bool) -> Self {
        Self { verify_output }
    }

    /// Compute the Blake3 checksum of a file.
    pub fn checksum_file(&self, path: &Path) -> Result<String, CoreError> {
        let data = std::fs::read(path)?;
        Ok(self.checksum_bytes(&data))
    }

    /// Compute the Blake3 checksum of a byte slice.
    pub fn checksum_bytes(&self, data: &[u8]) -> String {
        let hash = blake3::hash(data);
        hash.to_hex().to_string()
    }

    /// Verify that a file matches the expected checksum.
    pub fn verify_file(&self, path: &Path, expected: &str) -> Result<bool, CoreError> {
        if !self.verify_output {
            return Ok(true);
        }
        let actual = self.checksum_file(path)?;
        Ok(actual == expected)
    }

    /// Verify that a byte slice matches the expected checksum.
    pub fn verify_bytes(&self, data: &[u8], expected: &str) -> bool {
        if !self.verify_output {
            return true;
        }
        let actual = self.checksum_bytes(data);
        actual == expected
    }

    /// Compute SHA-256 checksum (for formats that use SHA-256).
    pub fn sha256_file(&self, path: &Path) -> Result<String, CoreError> {
        use sha2::{Sha256, Digest};
        let data = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Compare two files byte-by-byte.
    pub fn files_equal(&self, path_a: &Path, path_b: &Path) -> Result<bool, CoreError> {
        let a = std::fs::read(path_a)?;
        let b = std::fs::read(path_b)?;
        if a.len() != b.len() {
            return Ok(false);
        }
        // Constant-time comparison
        Ok(a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0)
    }
}

impl Default for IntegrityChecker {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_checksum_consistency() {
        let checker = IntegrityChecker::new(true);
        let data = b"hello world";
        let hash1 = checker.checksum_bytes(data);
        let hash2 = checker.checksum_bytes(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_checksum_different_data() {
        let checker = IntegrityChecker::new(true);
        let hash1 = checker.checksum_bytes(b"hello");
        let hash2 = checker.checksum_bytes(b"world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_file_checksum() {
        let checker = IntegrityChecker::new(true);
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test data").unwrap();
        let hash = checker.checksum_file(file.path()).unwrap();
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_verify_file() {
        let checker = IntegrityChecker::new(true);
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test data").unwrap();
        let hash = checker.checksum_file(file.path()).unwrap();
        assert!(checker.verify_file(file.path(), &hash).unwrap());
        assert!(!checker.verify_file(file.path(), "wrong_hash").unwrap());
    }
}
