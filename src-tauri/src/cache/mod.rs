//! 缩略图缓存：sled KV store，value = bincode 编码的 PreviewPayload
//!
//! key  = 路径的 SHA256（路径可能很长且含特殊字符）
//! value = bincode(PreviewPayload)

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use sled::Db;

use crate::preview::PreviewPayload;

pub struct ThumbnailCache {
    db: Arc<Mutex<Db>>,
}

impl ThumbnailCache {
    pub fn open<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let db = sled::open(dir.as_ref())
            .with_context(|| format!("failed to open cache db: {:?}", dir.as_ref()))?;
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
        })
    }

    pub fn lookup(&self, path: &str) -> Result<Option<PreviewPayload>> {
        Self::lookup_static(path)
    }

    pub fn store(&self, path: &str, payload: &PreviewPayload) -> Result<()> {
        Self::store_static(path, payload)
    }

    pub fn lookup_static(path: &str) -> Result<Option<PreviewPayload>> {
        let key = hash_path(path);
        let db = sled::open(
            std::env::var_os("SEER_CACHE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
                .join("thumbs"),
        )
        .ok();
        let Some(db) = db else { return Ok(None) };

        if let Some(ivec) = db.get(key)? {
            let payload: PreviewPayload = bincode::deserialize(&ivec)?;
            Ok(Some(payload))
        } else {
            Ok(None)
        }
    }

    pub fn store_static(path: &str, payload: &PreviewPayload) -> Result<()> {
        let key = hash_path(path);
        let db = sled::open(
            std::env::var_os("SEER_CACHE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
                .join("thumbs"),
        )?;
        let bytes = bincode::serialize(payload)?;
        db.insert(key, bytes)?;
        db.flush()?;
        Ok(())
    }
}

fn hash_path(path: &str) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let h = hasher.finish();

    let mut out = [0u8; 32];
    out[..8].copy_from_slice(&h.to_le_bytes());
    // 重复填充满 32 字节（sled key 无长度限制，但保持一致）
    out[8..].copy_from_slice(&h.to_le_bytes());
    out
}