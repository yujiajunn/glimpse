//! 文件解码 → 可绘制的 bitmap
//!
//! 根据扩展名 / 文件头分发到不同的解码器。

use std::path::Path;

use anyhow::{Context, Result};
use windows::Win32::Graphics::Gdi::HBITMAP;

/// 预览内容：解码后的 bitmap（HBITMAP + 原始宽高），或非图像（文本/视频）
pub enum PreviewContent {
    /// 已渲染成 HBITMAP，窗口直接 BitBlt
    Bitmap {
        handle: HBITMAP,
        width: i32,
        height: i32,
        /// 原始文件大小（用于按比例缩放）
        natural_width: i32,
        natural_height: i32,
        title: String,
    },
    /// 纯文本（短文本文档直接画文字）
    Text {
        title: String,
        lines: Vec<String>,
    },
    /// 视频（已截首帧；或调外部播放器）
    Video {
        title: String,
        bitmap: Option<HBITMAP>,
        bitmap_size: Option<(i32, i32)>,
        duration_sec: f32,
        width: i32,
        height: i32,
    },
    /// 不支持的格式
    Unsupported {
        reason: String,
    },
}

/// 按文件扩展名判断 kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Image,
    Pdf,
    Text,
    Code,
    Markdown,
    Font,
    Office,
    Archive,
    Video,
    Audio,
    Html,
    Unknown,
}

pub fn detect_kind(path: &Path) -> Kind {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tif" | "tiff"
        | "heic" | "heif" | "avif" | "raw" | "dng" | "cr2" | "nef" | "arw" => Kind::Image,

        "pdf" => Kind::Pdf,

        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v" => Kind::Video,
        "mp3" | "flac" | "wav" | "aac" | "ogg" | "m4a" | "opus" => Kind::Audio,

        "txt" | "log" => Kind::Text,
        "md" | "markdown" => Kind::Markdown,
        "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "c" | "cpp"
        | "h" | "hpp" | "css" | "sh" | "bash" | "sql" | "json" | "yaml" | "yml"
        | "toml" | "ini" | "conf" => Kind::Code,

        "ttf" | "otf" | "woff" => Kind::Font,
        "docx" | "xlsx" | "pptx" => Kind::Office,
        "zip" | "tar" | "gz" => Kind::Archive,

        "html" | "htm" => Kind::Html,

        _ => Kind::Unknown,
    }
}

/// 解码入口
pub fn decode_for_preview(path: &Path) -> Result<PreviewContent> {
    let kind = detect_kind(path);
    let title = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let result = match kind {
        Kind::Image => decode_image(path, &title),
        Kind::Pdf => decode_pdf(path, &title),
        Kind::Video | Kind::Audio => decode_media(path, &title),
        Kind::Text | Kind::Code => decode_text(path, &title),
        Kind::Markdown => decode_markdown(path, &title),
        Kind::Html => decode_html(path, &title),
        Kind::Font => decode_font(path),
        Kind::Office => decode_office(path),
        Kind::Archive => decode_archive(path),
        Kind::Unknown => decode_unsupported(path),
    };

    if let Ok(PreviewContent::Unsupported { .. }) = &result {
        // 兜底：调系统默认应用
        open_with_default_app(path);
    }
    result
}

// ================================================================
// 图片
// ================================================================

fn decode_image(path: &Path, title: &str) -> Result<PreviewContent> {
    use windows::Win32::Graphics::Gdi::{
        CreateDIBSection, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, DIB_PAL_COLORS,
    };

    let dyn_img = image::open(path)
        .with_context(|| format!("image decode: {}", path.display()))?;

    let (w, h) = (dyn_img.width() as i32, dyn_img.height() as i32);

    // 缩放：保持比例，最大边 ≤ 4096
    let max_dim = 4096;
    let scale = if w > max_dim || h > max_dim {
        let s = max_dim as f32 / w.max(h) as f32;
        s.min(1.0)
    } else {
        1.0
    };
    let (tw, th) = (((w as f32) * scale) as i32, ((h as f32) * scale) as i32);
    let tw = tw.max(1);
    let th = th.max(1);

    let rgba = dyn_img
        .resize_exact(tw as u32, th as u32, image::imageops::Triangle)
        .to_rgba8();
    let pixels = rgba.into_raw();

    unsafe {
        // 创建 DIB section（设备无关位图）
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: tw,
                biHeight: -th, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default(); 1],
        };

        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbm = CreateDIBSection(
            None,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )?;

        // 拷贝 RGBA 数据到位图
        std::ptr::copy_nonoverlapping(pixels.as_ptr() as *const u8, bits as *mut u8, pixels.len());

        Ok(PreviewContent::Bitmap {
            handle: hbm,
            width: tw,
            height: th,
            natural_width: w,
            natural_height: h,
            title: title.to_string(),
        })
    }
}

// ================================================================
// PDF
// ================================================================

fn decode_pdf(path: &Path, title: &str) -> Result<PreviewContent> {
    use windows::Win32::Graphics::Gdi::{
        CreateDIBSection, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use pdfium_render::prelude::*;

    let pdfium = Pdfium::new(
        Pdfium::bind_to_system_library()
            .ok()
            .ok_or_else(|| anyhow::anyhow!("pdfium library"))?
    );

    let document = pdfium
        .load_pdf_from_file(path, None)
        .with_context(|| format!("open pdf: {}", path.display()))?;

    let page = document.pages().get(0).context("first page")?;
    let (w, h) = (page.width().value as i32, page.height().value as i32);
    let dpi = 96.0_f32;

    let bitmap = page
        .render(dpi, dpi, PdfBitmapFormat::BGRA)
        .map_err(|e| anyhow::anyhow!("pdf render: {e:?}"))?;

    let raw: &[u8] = bitmap.as_raw_bytes();
    let (bw, bh) = (bitmap.width() as i32, bitmap.height() as i32);

    unsafe {
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bw,
                biHeight: -bh,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default(); 1],
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbm = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)?;
        std::ptr::copy_nonoverlapping(raw.as_ptr() as *const u8, bits as *mut u8, raw.len());

        Ok(PreviewContent::Bitmap {
            handle: hbm,
            width: bw,
            height: bh,
            natural_width: w,
            natural_height: h,
            title: title.to_string(),
        })
    }
}

// ================================================================
// 视频 / 音频（用 ShellExecute 调系统默认播放器：Media Player、wmplayer 等）
// ================================================================

fn decode_media(path: &Path, title: &str, _kind: Kind) -> Result<PreviewContent> {
    open_with_default_app(&path.to_path_buf());
    // 视频/音频无法在我们窗口内嵌播放，告知用户已转交系统
    Ok(PreviewContent::Unsupported {
        reason: format!(
            "{} 已在系统默认播放器中打开（视频/音频暂时调外部播放器）",
            title
        ),
    })
}

// ================================================================
// 文本 / 代码
// ================================================================

fn decode_text(path: &Path, title: &str) -> Result<PreviewContent> {
    let bytes = std::fs::read(path).with_context(|| format!("read: {}", path.display()))?;
    let s = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = s.lines().take(500).map(String::from).collect();
    Ok(PreviewContent::Text {
        title: title.to_string(),
        lines,
    })
}

fn decode_markdown(path: &Path, title: &str) -> Result<PreviewContent> {
    // 简化：当作文本显示，前缀 "[MD] "
    let mut c = decode_text(path, title)?;
    if let PreviewContent::Text { title, mut lines } = c {
        lines.insert(0, "[Markdown 内容，纯文本预览]".into());
        c = PreviewContent::Text { title, lines };
    }
    Ok(c)
}

fn decode_html(path: &Path, title: &str) -> Result<PreviewContent> {
    let mut c = decode_text(path, title)?;
    if let PreviewContent::Text { title, mut lines } = c {
        // 极简 HTML → 文本：去掉 tags
        let stripped: Vec<String> = lines
            .iter()
            .map(|l| {
                let mut s = l.clone();
                // 简单 tag 剥离
                let mut out = String::new();
                let mut in_tag = false;
                for c in s.chars() {
                    if c == '<' {
                        in_tag = true;
                    } else if c == '>' {
                        in_tag = false;
                    } else if !in_tag {
                        out.push(c);
                    }
                }
                out
            })
            .collect();
        c = PreviewContent::Text {
            title,
            lines: vec!["[HTML 内容，纯文本预览]".into()]
                .into_iter()
                .chain(stripped)
                .take(500)
                .collect(),
        };
    }
    Ok(c)
}

// ================================================================
// 字体 — 调系统字体查看器
// ================================================================

fn decode_font(path: &Path) -> Result<PreviewContent> {
    decode_unsupported(path)
}

// ================================================================
// Office — 调 LibreOffice / Word 转 PDF 再渲染，或直接调默认
// ================================================================

fn decode_office(path: &Path) -> Result<PreviewContent> {
    decode_unsupported(path)
}

// ================================================================
// 压缩包 — 调 7-Zip / 系统
// ================================================================

fn decode_archive(path: &Path) -> Result<PreviewContent> {
    decode_unsupported(path)
}

fn open_with_default_app(path: &Path) {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
    };
    let path_str = path.to_string_lossy().to_string();
    unsafe {
        let verb = "open\0".encode_utf16().collect::<Vec<u16>>();
        let file = (path_str + "\0").encode_utf16().collect::<Vec<u16>>();
        let info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            nShow: 1,
            ..Default::default()
        };
        let _ = ShellExecuteExW(&info);
    }
}