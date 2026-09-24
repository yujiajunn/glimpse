//! 缩略图缓存：sled KV store，value = postcard 编码的 PreviewPayload
//!
//! key   = 路径哈希（sled key 可以直接是字节数组）
//! value = postcard(PreviewPayload)

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use sled::Db;

use crate::preview::PreviewPayload;

/// 全局缓存实例（lazy init；进程内唯一）
static CACHE_DB: once_cell::sync::Lazy<std::io::Result<Db>> =
    once_cell::sync::Lazy::new(|| {
        let dir = default_cache_dir();
        std::fs::create_dir_all(&dir).ok();
        sled::open(&dir).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });

fn default_cache_dir() -> PathBuf {
    std::env::var_os("GLIMPSE_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("thumbs")
}

pub struct ThumbnailCache {
    db: Arc<Mutex<Db>>,
}

impl ThumbnailCache {
    pub fn open<P: AsRef<Path>>(_dir: P) -> Result<Self> {
        // 用全局 lazy 单例，dir 参数保留以兼容旧调用
        let db = CACHE_DB
            .as_ref()
            .map_err(|e| anyhow::anyhow!("failed to open cache: {e}"))?
            .clone();
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
        let db = CACHE_DB
            .as_ref()
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        match db.get(key).context("sled get failed")? {
            Some(ivec) => {
                let payload: PreviewPayload =
                    postcard::from_bytes(&ivec).context("postcard decode failed")?;
                Ok(Some(payload))
            }
            None => Ok(None),
        }
    }

    pub fn store_static(path: &str, payload: &PreviewPayload) -> Result<()> {
        let key = hash_path(path);
        let db = CACHE_DB
            .as_ref()
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let bytes = postcard::to_stdvec(payload)
            .context("postcard encode failed")?;
        db.insert(key, bytes).context("sled insert failed")?;
        db.flush().context("sled flush failed")?;
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
    out[8..16].copy_from_slice(&h.to_le_bytes());
    out[16..24].copy_from_slice(&h.to_le_bytes());
    out[24..].copy_from_slice(&h.to_le_bytes());
    out
}