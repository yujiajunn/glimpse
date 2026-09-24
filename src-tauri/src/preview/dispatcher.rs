//! 预览调度器：按文件扩展名路由到对应 handler
//!
//! 设计要点：
//! - handlers 用 `Arc<dyn PreviewHandler>` 持有，便于跨 await 持有
//! - 注册表用 `parking_lot::RwLock` 保护
//! - 异步 IO 全部 `async`

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;
use tracing::{debug, warn};

use super::image::ImagePreview;
use super::pdf::PdfPreview;
use super::video::VideoPreview;
use super::text::TextPreview;
use super::html::HtmlPreview;
use super::office::OfficePreview;
use super::font::FontPreview;
use super::archive::ArchivePreview;
use super::directory::DirectoryPreview;
use super::{PreviewPayload, PreviewRequest};

/// 预览 handler 抽象
#[async_trait::async_trait]
pub trait PreviewHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn matches(&self, path: &std::path::Path) -> bool;
    async fn handle(&self, req: &PreviewRequest) -> Result<PreviewPayload>;
}

/// 共享 handler（可跨 await 持有）
pub type SharedHandler = Arc<dyn PreviewHandler>;

/// 调度器：handler 注册表 + 缓存入口
pub struct PreviewDispatcher {
    #[allow(dead_code)]
    cache_dir: PathBuf,
    handlers: RwLock<Vec<SharedHandler>>,
}

impl PreviewDispatcher {
    pub fn new(cache_dir: PathBuf) -> Self {
        // 注册顺序：特异性高的优先（PDF 优先于 image，因为 PDF 也可能伪装成 .img）
        let handlers: Vec<SharedHandler> = vec![
            Arc::new(DirectoryPreview::new()), // 目录最先（其他都是文件）
            Arc::new(ArchivePreview::new()),
            Arc::new(OfficePreview::new()),
            Arc::new(FontPreview::new()),
            Arc::new(PdfPreview::new()),
            Arc::new(VideoPreview::new()),
            Arc::new(HtmlPreview::new()),
            Arc::new(ImagePreview::new()),
            Arc::new(TextPreview::new()),
        ];

        Self {
            cache_dir,
            handlers: RwLock::new(handlers),
        }
    }

    /// 暴露给 IPC 层的高层入口：先查缓存，再分发
    pub async fn dispatch(&self, req: PreviewRequest) -> Result<PreviewPayload> {
        let path = std::path::Path::new(&req.path);

        // 1. 缓存查询
        if let Some(cached) = crate::cache::ThumbnailCache::lookup_static(&req.path)? {
            debug!(path = %req.path, "cache hit");
            return Ok(cached);
        }

        // 2. 找匹配的 handler（克隆 Arc 后释放锁，避免持锁 await）
        let handler = {
            let handlers = self.handlers.read();
            handlers.iter().find(|h| h.matches(path)).cloned()
        };

        let Some(handler) = handler else {
            warn!(path = %req.path, "no handler matched");
            return Ok(PreviewPayload::Unsupported {
                reason: format!("no preview handler matched: {}", req.path),
            });
        };

        // 3. 调用 handler（直接 await）
        let payload = handler.handle(&req).await?;

        // 4. 写缓存
        crate::cache::ThumbnailCache::store_static(&req.path, &payload)?;

        Ok(payload)
    }

    /// 注册一个外部 handler（插件系统用）
    pub fn register(&self, handler: SharedHandler) {
        let name = handler.name();
        let mut handlers = self.handlers.write();
        // 插到头部：新插件优先
        handlers.insert(0, handler);
        tracing::info!(handler = name, "external preview handler registered");
    }

    /// 列出当前已注册的所有 handler 名称
    pub fn list_handlers(&self) -> Vec<&'static str> {
        self.handlers
            .read()
            .iter()
            .map(|h| h.name())
            .collect()
    }
}