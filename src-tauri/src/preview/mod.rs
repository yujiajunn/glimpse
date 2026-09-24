//! 全格式预览载荷（与 ui/src/types.ts 保持一致）
//!
//! 每种 kind 对应一个 handler + 一个前端组件：
//! image  → ImagePreview  + ImageView
//! pdf    → PdfPreview    + PdfView
//! video  → VideoPreview  + VideoView
//! audio  → VideoPreview  + VideoView
//! text   → TextPreview   + TextView
//! html   → HtmlPreview   + TextView (markdown)
//! office → OfficePreview + OfficeView
//! font   → FontPreview   + FontView
//! archive→ ArchivePreview+ ArchiveView
//! directory → DirectoryPreview + DirectoryView
//! unsupported → 默认占位

mod dispatcher;
mod html;
mod pdf;
mod text;
mod video;
mod image;
mod font;
mod office;
mod archive;
mod directory;
mod ipc;
pub mod plugins;

pub use dispatcher::{PreviewDispatcher, PreviewHandler};
pub use ipc::{
    cmd_show_preview, cmd_hide_preview, cmd_load_image, cmd_load_pdf_page,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewRequest {
    pub path: String,
    pub max_width: u32,
    pub max_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreviewPayload {
    Image {
        data_base64: String,
        width: u32,
        height: u32,
        mime: String,
    },
    Pdf {
        data_base64: String,
        page_count: u32,
    },
    Video {
        url: String,
        duration_sec: f32,
        width: u32,
        height: u32,
    },
    Audio {
        url: String,
        duration_sec: f32,
    },
    Text {
        content: String,
        is_markdown: bool,
    },
    Office {
        office_kind: OfficeKind,
        content: OfficeContent,
        title: String,
    },
    Font {
        family: String,
        style: String,
        glyph_count: u32,
        /// base64 PNG 字符映射表
        specimen_png: String,
    },
    Archive {
        entries: Vec<ArchiveEntry>,
        total_size: u64,
    },
    Directory {
        entries: Vec<DirEntry>,
        total_count: u32,
    },
    Unsupported {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficeKind {
    Docx,
    Xlsx,
    Pptx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum OfficeContent {
    Html { html: String },
    Sheets { sheets: Vec<SheetData> },
    Slides { slides: Vec<SlideData> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetData {
    pub name: String,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideData {
    pub index: u32,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<i64>,
}