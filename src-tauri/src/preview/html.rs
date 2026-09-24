//! HTML / Markdown 占位：实际渲染由前端 react-markdown 完成，
//! 后端只返回路径或原文。

use anyhow::Result;
use std::path::Path;

use super::{PreviewHandler, PreviewPayload, PreviewRequest};

pub struct HtmlPreview;

impl HtmlPreview {
    pub fn new() -> Self {
        Self
    }

    fn is_html(ext: &str) -> bool {
        matches!(ext.to_ascii_lowercase().as_str(), "html" | "htm")
    }
}

impl Default for HtmlPreview {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PreviewHandler for HtmlPreview {
    fn name(&self) -> &'static str {
        "html"
    }

    fn matches(&self, path: &Path) -> bool {
        // MD 也归这里（前端统一 markdown 渲染）
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| Self::is_html(e) || matches!(e.to_ascii_lowercase().as_str(), "md" | "markdown"))
            .unwrap_or(false)
    }

    async fn handle(&self, req: &PreviewRequest) -> Result<PreviewPayload> {
        let ext = req
            .path
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        // HTML 文件以 iframe 形式让 WebView2 自己渲染（沙箱）
        if Self::is_html(&ext) {
            // 用沙箱 URL 让前端 <iframe src="...">
            return Ok(PreviewPayload::Text {
                content: format!("iframe:{}", req.path),
                is_markdown: false,
            });
        }

        // MD 文件读出来交给前端 markdown 库
        let bytes = std::fs::read(&req.path)?;
        Ok(PreviewPayload::Text {
            content: String::from_utf8_lossy(&bytes).to_string(),
            is_markdown: true,
        })
    }
}