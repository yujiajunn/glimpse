//! Tauri IPC 命令：前端 ↔ 后端桥

use tauri::State;

use super::PreviewPayload;
use super::PreviewRequest;
use crate::AppState;

#[tauri::command]
pub async fn cmd_show_preview(
    path: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<PreviewPayload, String> {
    let req = PreviewRequest {
        path,
        max_width: 1600,
        max_height: 1000,
    };

    let payload = state
        .dispatcher
        .dispatch(req)
        .await
        .map_err(|e| format!("{e:#}"))?;

    if let Some(window) = app.get_webview_window("preview") {
        window.show().map_err(stringify)?;
        window.set_focus().map_err(stringify)?;
    }

    Ok(payload)
}

#[tauri::command]
pub async fn cmd_hide_preview(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("preview") {
        window.hide().map_err(stringify)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn cmd_load_image(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(stringify)?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &bytes,
    ))
}

#[tauri::command]
pub async fn cmd_load_pdf_page(path: String, page: u32, dpi: u32) -> Result<String, String> {
    use pdfium_render::prelude::*;

    let pdfium = Pdfium::new(Pdfium::bind_to_statically_linked_library().ok())
        .map_err(stringify)?;
    let doc = pdfium
        .load_pdf_from_file(&path, None)
        .map_err(stringify)?;
    let p = doc.pages().get(page as usize).map_err(stringify)?;
    let bitmap = p
        .render(dpi as f32, dpi as f32, PdfBitmapFormat::BGRA)
        .map_err(stringify)?;
    let raw = bitmap.as_raw_bytes();

    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        raw,
    ))
}

fn stringify<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}