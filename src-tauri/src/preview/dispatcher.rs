//! 预览调度器：按文件扩展名路由到对应 handler
//!
//! 设计要点：
//! - handlers 用 `parking_lot::RwLock` 保护（读多写少，几乎只读）
//! - 缓存查找走 `dispatcher.dispatch()` 内部，避免外部重复查
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

type BoxedHandler = Box<dyn PreviewHandler>;

/// 调度器：handler 注册表 + 缓存入口
pub struct PreviewDispatcher {
    #[allow(dead_code)]
    cache_dir: PathBuf,
    handlers: RwLock<Vec<BoxedHandler>>,
}

impl PreviewDispatcher {
    pub fn new(cache_dir: PathBuf) -> Self {
        // 注册顺序：特异性高的优先（PDF 优先于 image，因为 PDF 也可能伪装成 .img）
        let handlers: Vec<BoxedHandler> = vec![
            Box::new(DirectoryPreview::new()), // 目录最先（其他都是文件）
            Box::new(ArchivePreview::new()),
            Box::new(OfficePreview::new()),
            Box::new(FontPreview::new()),
            Box::new(PdfPreview::new()),
            Box::new(VideoPreview::new()),
            Box::new(HtmlPreview::new()),
            Box::new(ImagePreview::new()),
            Box::new(TextPreview::new()),
        ];

        Self {
            cache_dir,
            handlers: RwLock::new(handlers),
        }
    }

    /// 暴露给 IPC 层的高层入口：先查缓存，再分发
    pub fn dispatch(&self, req: PreviewRequest) -> Result<PreviewPayload> {
        let path = std::path::Path::new(&req.path);

        // 1. 缓存查询
        if let Some(cached) = crate::cache::ThumbnailCache::lookup_static(&req.path)? {
            debug!(path = %req.path, "cache hit");
            return Ok(cached);
        }

        // 2. 找匹配的 handler
        let handlers = self.handlers.read();
        let handler = handlers.iter().find(|h| h.matches(path));

        let Some(handler) = handler else {
            warn!(path = %req.path, "no handler matched");
            return Ok(PreviewPayload::Unsupported {
                reason: format!("no preview handler matched: {}", req.path),
            });
        };

        // 3. 调用 handler（注意 handler 内部是 async；这里用 block_on）
        let payload = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(handler.handle(&req))
        })?;

        // 4. 写缓存
        crate::cache::ThumbnailCache::store_static(&req.path, &payload)?;

        Ok(payload)
    }

    /// 供前端异步调用的便捷方法（包装 dispatch）
    pub fn dispatch_arc(self: &Arc<Self>, req: PreviewRequest) -> Result<PreviewPayload> {
        self.dispatch(req)
    }

    /// 注册一个外部 handler（插件系统用）
    pub fn register(&self, handler: BoxedHandler) {
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