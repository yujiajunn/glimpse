//! 目录预览：列出前 N 项 + 大小 + 修改时间

use anyhow::Result;
use std::path::Path;

use super::{DirEntry, PreviewHandler, PreviewPayload, PreviewRequest};

const MAX_ENTRIES: usize = 1000;

pub struct DirectoryPreview;

impl DirectoryPreview {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DirectoryPreview {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PreviewHandler for DirectoryPreview {
    fn name(&self) -> &'static str {
        "directory"
    }

    fn matches(&self, path: &Path) -> bool {
        path.is_dir()
    }

    async fn handle(&self, req: &PreviewRequest) -> Result<PreviewPayload> {
        use std::fs;

        let mut entries = Vec::new();
        let read_dir = fs::read_dir(&req.path)?;

        for entry in read_dir.flatten().take(MAX_ENTRIES) {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);

            entries.push(DirEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir: metadata.is_dir(),
                size: if metadata.is_dir() { 0 } else { metadata.len() },
                modified,
            });
        }

        // 目录排前面，文件按名字
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        let total_count = entries.len() as u32;

        Ok(PreviewPayload::Directory {
            entries,
            total_count,
        })
    }
}