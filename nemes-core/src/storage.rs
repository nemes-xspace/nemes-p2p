//! In-memory storage (VPS-less minimal; RocksDB later)

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct StorageEngine {
    inner: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl StorageEngine {
    pub fn new(_path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        Ok(Self::default())
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.inner.write().map_err(|_| anyhow::anyhow!("lock"))?.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.inner.read().map_err(|_| anyhow::anyhow!("lock"))?.get(key).cloned())
    }

    pub fn delete(&self, key: &[u8]) -> anyhow::Result<()> {
        self.inner.write().map_err(|_| anyhow::anyhow!("lock"))?.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_storage_basic() {
        let s = StorageEngine::default();
        s.put(b"key1", b"value1").unwrap();
        assert_eq!(s.get(b"key1").unwrap().unwrap(), b"value1");
        s.delete(b"key1").unwrap();
        assert!(s.get(b"key1").unwrap().is_none());
    }
}
