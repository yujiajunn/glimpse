//! 字体预览：ttf-parser 提取元数据 + 首屏字符表
//!
//! 真实渲染由前端 Canvas 完成；后端只返回字体家族名 + glyph 数

use anyhow::{Context, Result};
use std::path::Path;

use super::{PreviewHandler, PreviewPayload, PreviewRequest};

pub struct FontPreview;

impl FontPreview {
    pub fn new() -> Self {
        Self
    }

    fn matches(path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .as_deref(),
            Some("ttf" | "otf" | "woff" | "woff2")
        )
    }
}

impl Default for FontPreview {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PreviewHandler for FontPreview {
    fn name(&self) -> &'static str {
        "font"
    }

    fn matches(&self, path: &Path) -> bool {
        Self::matches(path)
    }

    async fn handle(&self, req: &PreviewRequest) -> Result<PreviewPayload> {
        let bytes = std::fs::read(&req.path)
            .with_context(|| format!("failed to read font: {}", req.path))?;

        let face = ttf_parser::Face::parse(&bytes, 0)
            .with_context(|| format!("failed to parse font: {}", req.path))?;

        let family = face
            .names()
            .into_iter()
            .find(|n| n.name_id == ttf_parser::name_id::FAMILY)
            .and_then(|n| n.to_string())
            .unwrap_or_else(|| "Unknown".into());

        let style = face
            .names()
            .into_iter()
            .find(|n| n.name_id == ttf_parser::name_id::SUBFAMILY)
            .and_then(|n| n.to_string())
            .unwrap_or_else(|| "Regular".into());

        let glyph_count = face.number_of_glyphs() as u32;

        // 字体预览图（前端 canvas 实时画更佳，后端占位）
        let specimen_png = String::new();

        Ok(PreviewPayload::Font {
            family,
            style,
            glyph_count,
            specimen_png,
        })
    }
}