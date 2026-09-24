use anyhow::{Context, Result};
use std::path::Path;

use super::{PreviewHandler, PreviewPayload, PreviewRequest};

/// 图片预览：image crate 解码 → 自动缩放到目标尺寸 → 编码为 base64
pub struct ImagePreview;

impl ImagePreview {
    pub fn new() -> Self {
        Self
    }

    fn ext_matches(ext: &str) -> bool {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tiff" | "tif"
                | "heic" | "heif" | "avif" | "raw" | "dng" | "cr2" | "nef" | "arw"
        )
    }
}

impl Default for ImagePreview {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PreviewHandler for ImagePreview {
    fn name(&self) -> &'static str {
        "image"
    }

    fn matches(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(Self::ext_matches)
            .unwrap_or(false)
    }

    async fn handle(&self, req: &PreviewRequest) -> Result<PreviewPayload> {
        let img = image::open(&req.path)
            .with_context(|| format!("failed to decode image: {}", req.path))?;

        // 等比缩放（限制最大边长）
        let (w, h) = (img.width(), img.height());
        let scale_w = req.max_width as f32 / w as f32;
        let scale_h = req.max_height as f32 / h as f32;
        let scale = scale_w.min(scale_h).min(1.0);

        let resized = if scale < 1.0 {
            let new_w = ((w as f32) * scale).round() as u32;
            let new_h = ((h as f32) * scale).round() as u32;
            img.resize(new_w, new_h, image::imageops::FilterType::Triangle)
        } else {
            img
        };

        // 编码为 PNG（也可换 WebP 体积更小）
        let mut buf = Vec::new();
        resized
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .context("failed to encode PNG")?;

        Ok(PreviewPayload::Image {
            data_base64: base64_encode(&buf),
            width: resized.width(),
            height: resized.height(),
            mime: "image/png".to_string(),
        })
    }
}

/// 简化版 base64（实际项目用 base64 crate）
fn base64_encode(data: &[u8]) -> String {
    use std::io::Write;
    // 实际应当用 `base64` crate；省去依赖故写占位
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = match chunk.len() {
            1 => [chunk[0], 0, 0],
            2 => [chunk[0], chunk[1], 0],
            _ => [chunk[0], chunk[1], chunk[2]],
        };
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | (b[2] as u32);
        const T: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let c0 = T[((n >> 18) & 0x3F) as usize];
        let c1 = T[((n >> 12) & 0x3F) as usize];
        let c2 = if chunk.len() > 1 { T[((n >> 6) & 0x3F) as usize] } else { b'=' };
        let c3 = if chunk.len() > 2 { T[(n & 0x3F) as usize] } else { b'=' };
        out.write_all(&[c0, c1, c2, c3]).unwrap();
    }
    out
}