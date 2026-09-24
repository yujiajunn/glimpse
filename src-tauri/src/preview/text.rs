use anyhow::Result;
use std::path::Path;

use super::{PreviewHandler, PreviewPayload, PreviewRequest};

/// 文本 / 代码预览：直接读出来，前端用 Monaco / CodeMirror 渲染
pub struct TextPreview;

impl TextPreview {
    pub fn new() -> Self {
        Self
    }

    fn ext_matches(ext: &str) -> bool {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "txt" | "log" | "md" | "markdown" | "json" | "yaml" | "yml"
                | "toml" | "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "go"
                | "java" | "c" | "cpp" | "h" | "hpp" | "css" | "html" | "xml"
                | "sh" | "bash" | "sql" | "ini" | "conf" | "csv"
        )
    }
}

impl Default for TextPreview {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PreviewHandler for TextPreview {
    fn name(&self) -> &'static str {
        "text"
    }

    fn matches(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(Self::ext_matches)
            .unwrap_or(false)
    }

    async fn handle(&self, req: &PreviewRequest) -> Result<PreviewPayload> {
        let ext = req
            .path
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        // 大文件截断（避免 100MB log 直接读进内存）
        let max_bytes = 5 * 1024 * 1024;
        let bytes = std::fs::read(&req.path)?;
        let truncated = bytes.len() > max_bytes;
        let content = if truncated {
            let mut s = String::from_utf8_lossy(&bytes[..max_bytes]).to_string();
            s.push_str("\n\n... [truncated] ...");
            s
        } else {
            String::from_utf8_lossy(&bytes).to_string()
        };

        let is_markdown = matches!(ext.as_str(), "md" | "markdown");

        Ok(PreviewPayload::Text {
            content,
            is_markdown,
        })
    }
}