//! Content Store (SHA256-addressed)
//!
//! Message bodies are stored separately from the columnar tables.
//! Bodies are large, opaque, and read-once — columnar compression doesn't help.
//! Content-addressing provides deduplication.
//!
//! ```text
//! put(data) → SHA256 hash
//! get(hash) → data
//! ```

use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// SHA256-addressed content store for message bodies
pub struct ContentStore {
    path: PathBuf,
}

impl ContentStore {
    /// Open or create content store at path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    /// Store content, returns SHA256 hex hash
    pub fn put(&self, data: &[u8]) -> Result<String> {
        let hash = Self::hash(data);
        let file_path = self.content_path(&hash);

        if !file_path.exists() {
            // Two-level directory structure to avoid too many files in one dir
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&file_path, data)?;
        }

        Ok(hash)
    }

    /// Retrieve content by SHA256 hex hash
    pub fn get(&self, hash: &str) -> Result<Vec<u8>> {
        let file_path = self.content_path(hash);
        std::fs::read(&file_path).map_err(|_| Error::NotFound(format!("content:{}", hash)))
    }

    /// Check if content exists
    pub fn exists(&self, hash: &str) -> bool {
        self.content_path(hash).exists()
    }

    /// Compute SHA256 hex hash of data
    pub fn hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Path for a given hash (two-level: ab/cdef...)
    fn content_path(&self, hash: &str) -> PathBuf {
        let prefix = &hash[..2.min(hash.len())];
        self.path.join(prefix).join(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = ContentStore::open(dir.path()).unwrap();

        let data = b"Hello, Spear!";
        let hash = store.put(data).unwrap();

        assert!(store.exists(&hash));
        let retrieved = store.get(&hash).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_deduplication() {
        let dir = tempfile::tempdir().unwrap();
        let store = ContentStore::open(dir.path()).unwrap();

        let data = b"duplicate content";
        let hash1 = store.put(data).unwrap();
        let hash2 = store.put(data).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let store = ContentStore::open(dir.path()).unwrap();
        assert!(store.get("nonexistent").is_err());
    }
}
