//! 压缩包预览：ZIP / TAR
//!
//! 仅列出条目元数据，不展开文件内容

use anyhow::Result;
use std::path::Path;

use super::{ArchiveEntry, PreviewHandler, PreviewPayload, PreviewRequest};

pub struct ArchivePreview;

impl ArchivePreview {
    pub fn new() -> Self {
        Self
    }

    fn kind_of(path: &Path) -> Option<&'static str> {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref()
        {
            Some("zip") => Some("zip"),
            Some("tar") => Some("tar"),
            Some("tgz") | Some("gz") => Some("tar"),
            _ => None,
        }
    }
}

impl Default for ArchivePreview {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PreviewHandler for ArchivePreview {
    fn name(&self) -> &'static str {
        "archive"
    }

    fn matches(&self, path: &Path) -> bool {
        Self::kind_of(path).is_some()
    }

    async fn handle(&self, req: &PreviewRequest) -> Result<PreviewPayload> {
        let kind = Self::kind_of(Path::new(&req.path)).unwrap_or("zip");

        let entries = match kind {
            "zip" => read_zip(&req.path)?,
            "tar" => read_tar(&req.path)?,
            _ => unreachable!(),
        };

        let total_size: u64 = entries.iter().map(|e| e.size).sum();

        Ok(PreviewPayload::Archive {
            entries,
            total_size,
        })
    }
}

fn read_zip(path: &str) -> Result<Vec<ArchiveEntry>> {
    use std::fs::File;
    use std::io::Read;
    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let mut entries = Vec::with_capacity(zip.len());

    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        let modified = entry.last_modified().map(|t| {
            t.to_time_t().unwrap_or(0)
        });
        entries.push(ArchiveEntry {
            name: entry.name().to_string(),
            size: entry.size(),
            is_dir: entry.is_dir(),
            modified,
        });
    }
    Ok(entries)
}

fn read_tar(path: &str) -> Result<Vec<ArchiveEntry>> {
    let file = std::fs::File::open(path)?;
    let mut tar = tar::Archive::new(file);
    let mut entries = Vec::new();

    for entry in tar.entries()? {
        let entry = entry?;
        let header = entry.header();
        let modified = header.mtime().ok();
        entries.push(ArchiveEntry {
            name: entry.path()?.to_string_lossy().to_string(),
            size: entry.size(),
            is_dir: header.entry_type().is_dir(),
            modified,
        });
    }
    Ok(())
}