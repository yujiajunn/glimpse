//! Glimpse 应用入口（最小化版本 — 等 CI 跑通再补回功能）

use tracing::info;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    info!("Glimpse starting");

    tauri::Builder::default()
        .setup(|_app| {
            info!("Start marker");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running glimpse");
}