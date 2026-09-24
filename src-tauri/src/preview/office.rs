//! Office 文档预览：DOCX / XLSX / PPTX
//!
//! - DOCX：解析 XML，提取段落+表格，组装为 HTML
//! - XLSX：calamine 解析为行数据
//! - PPTX：解析 XML 提取每页文本

use anyhow::{Context, Result};
use std::path::Path;

use super::{OfficeContent, OfficeKind, PreviewHandler, PreviewPayload, PreviewRequest, SheetData, SlideData};

pub struct OfficePreview;

impl OfficePreview {
    pub fn new() -> Self {
        Self
    }

    fn kind_of(path: &Path) -> Option<OfficeKind> {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref()
        {
            Some("docx") => Some(OfficeKind::Docx),
            Some("xlsx") => Some(OfficeKind::Xlsx),
            Some("pptx") => Some(OfficeKind::Pptx),
            _ => None,
        }
    }
}

impl Default for OfficePreview {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PreviewHandler for OfficePreview {
    fn name(&self) -> &'static str {
        "office"
    }

    fn matches(&self, path: &Path) -> bool {
        Self::kind_of(path).is_some()
    }

    async fn handle(&self, req: &PreviewRequest) -> Result<PreviewPayload> {
        let kind = Self::kind_of(Path::new(&req.path))
            .context("not an office file")?;
        let title = Path::new(&req.path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let content = match kind {
            OfficeKind::Docx => read_docx(&req.path)?,
            OfficeKind::Xlsx => read_xlsx(&req.path)?,
            OfficeKind::Pptx => read_pptx(&req.path)?,
        };

        Ok(PreviewPayload::Office {
            office_kind: kind,
            content,
            title,
        })
    }
}

fn read_docx(path: &str) -> Result<OfficeContent> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let mut document_xml = String::new();
    zip.by_name("word/document.xml")?
        .read_to_string(&mut document_xml)?;

    // 简易转 HTML：把所有 <w:t> 文本节点包成 <p>
    let mut html = String::from("<div class=\"docx\">");
    let mut in_paragraph = false;
    let mut cursor = 0usize;
    while let Some(start) = document_xml[cursor..].find('<') {
        let abs = cursor + start;
        if let Some(tag_end) = document_xml[abs..].find('>') {
            let tag = &document_xml[abs..abs + tag_end + 1];
            match tag {
                    t if t.starts_with("<w:p ") || t == "<w:p>" => {
                        html.push_str("<p>");
                        in_paragraph = true;
                    }
                    t if t == "</w:p>" => {
                        html.push_str("</p>");
                        in_paragraph = false;
                    }
                    t if t == "<w:tab/>" || t == "<w:tab />" => html.push_str("\t"),
                    t if t == "<w:br/>" || t == "<w:br />" => html.push_str("<br/>"),
                    _ => {}
                }
            cursor = abs + tag_end + 1;
        } else {
            break;
        }
    }
    // 提取所有 <w:t>...</w:t> 之间的纯文本
    let text = extract_text_nodes(&document_xml);
    for line in text.lines() {
        if !line.is_empty() && !html.contains(line) {
            html.push_str(&format!("<p>{}</p>", html_escape(line)));
        }
    }
    html.push_str("</div>");

    Ok(OfficeContent::Html { html })
}

fn read_xlsx(path: &str) -> Result<OfficeContent> {
    use calamine::{open_workbook_auto, Data, Reader};
    let mut wb = open_workbook_auto(path)?;
    let mut result = Vec::new();

    for sheet_name in wb.sheet_names().to_vec() {
        let range = wb.worksheet_range(&sheet_name)?;
        let mut rows = Vec::new();
        for row in range.rows() {
            let row: Vec<String> = row
                .iter()
                .map(|c| match c {
                    Data::Empty => String::new(),
                    Data::String(s) => s.clone(),
                    Data::Float(f) => f.to_string(),
                    Data::Int(i) => i.to_string(),
                    Data::Bool(b) => b.to_string(),
                    Data::DateTime(d) => d.to_string(),
                    Data::DateTimeIso(d) => d.to_string(),
                    Data::DurationIso(d) => d.to_string(),
                    Data::Error(e) => format!("#ERR{e:?}"),
                })
                .collect();
            rows.push(row);
        }
        result.push(SheetData {
            name: sheet_name,
            rows,
        });
    }

    Ok(OfficeContent::Sheets { sheets: result })
}

fn read_pptx(path: &str) -> Result<OfficeContent> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let mut slides = Vec::new();

    for i in 1..=zip.len() {
        let entry_name = format!("ppt/slides/slide{}.xml", i);
        if let Ok(mut entry) = zip.by_name(&entry_name) {
            let mut xml = String::new();
            entry.read_to_string(&mut xml)?;
            let text = extract_text_nodes(&xml);
            let title = text.lines().next().unwrap_or("").to_string();
            slides.push(SlideData {
                index: i as u32,
                title,
                text,
            });
        }
    }

    Ok(OfficeContent::Slides { slides })
}

fn extract_text_nodes(xml: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(start) = xml[cursor..].find("<w:t") {
        let abs = cursor + start;
        if let Some(gt) = xml[abs..].find('>') {
            let after = abs + gt + 1;
            if let Some(end) = xml[after..].find("</w:t>") {
                out.push_str(&xml[after..after + end]);
                out.push('\n');
                cursor = after + end + "</w:t>".len();
                continue;
            }
        }
        break;
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}