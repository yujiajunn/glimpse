use anyhow::{Context, Result};
use pdfium_render::prelude::*;
use std::path::Path;

use super::{PreviewHandler, PreviewPayload, PreviewRequest};

/// PDF 预览：pdfium-render 把整文件读出来交给前端 pdf.js 渲染
pub struct PdfPreview {
    doc: Option<Pdfium<'static>>,
}

impl PdfPreview {
    pub fn new() -> Self {
        // 编译期绑定的 pdfium 静态库 → 直接构造
        let doc = Pdfium::bind_to_statically_linked_library()
            .ok()
            .and_then(|bindings| Pdfium::new(bindings).ok());
        Self { doc }
    }
}

impl Default for PdfPreview {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PreviewHandler for PdfPreview {
    fn name(&self) -> &'static str {
        "pdf"
    }

    fn matches(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false)
    }

    async fn handle(&self, req: &PreviewRequest) -> Result<PreviewPayload> {
        let pdfium = self
            .doc
            .as_ref()
            .context("pdfium library not loaded")?;

        let document = pdfium
            .load_pdf_from_file(&req.path, None)
            .with_context(|| format!("failed to open PDF: {}", req.path))?;

        let page_count = document.pages().len();

        // 读全文 → base64 → 前端 pdf.js 渲染
        let bytes = std::fs::read(&req.path)
            .with_context(|| format!("failed to read PDF: {}", req.path))?;

        Ok(PreviewPayload::Pdf {
            data_base64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes),
            page_count: page_count as u32,
        })
    }
}