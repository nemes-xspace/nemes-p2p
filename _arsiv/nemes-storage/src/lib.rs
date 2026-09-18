use nemes_core::{ContentId, ShardId};
use anyhow::Result;
use rocksdb::{DB, Options, WriteBatch, IteratorMode};
use std::path::Path;
use std::sync::Arc;

pub struct StorageEngine {
    db: Arc<rocksdb::DB>,
}

impl StorageEngine {
    pub fn new(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_max_background_jobs(4);
        opts.set_max_background_compactions(4);
        
        let db = DB::open(&opts, path)?;
        Ok(Self { db: Arc::new(db) })
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db.put(key, value)?;
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.db.get(key)?)
    }

    pub fn delete(&self, key: &[u8]) -> Result<()> {
        self.db.delete(key)?;
        Ok(())
    }

    pub fn put_batch(&self, batch: Vec<(Vec<u8>, Vec<u8>)>) -> Result<()> {
        let mut batch = rocksdb::WriteBatch::default();
        for (k, v) in batch {
            batch.put(k, v);
        }
        self.db.write(batch)?;
        Ok(())
    }

    pub fn get_range(&self, prefix: &[u8]) -> impl Iterator<Item = Result<(Vec<u8>, Vec<u8>)>> {
        let iter = self.db.iterator(rocksdb::IteratorMode::From(
            &[],
            rocksdb::Direction::Forward,
        ));
        iter.filter_map(move |item| {
            item.ok().and_then(|(k, v)| {
                if k.starts_with(&prefix) {
                    Some(Ok((k.to_vec(), v.to_vec())))
                } else {
                    None
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_storage_basic() {
        let dir = tempdir().unwrap();
        let storage = StorageEngine::new(dir.path()).unwrap();
        
        storage.put(b"key1", b"value1").unwrap();
        let value = storage.get(b"key1").unwrap().unwrap();
        assert_eq!(value, b"value1");
        
        storage.delete(b"key1").unwrap();
        assert!(storage.get(b"key1").unwrap().is_none());
    }
}