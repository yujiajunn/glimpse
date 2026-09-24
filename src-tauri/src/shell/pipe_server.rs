//! 命名管道服务器：监听 Shell Extension DLL 发来的触发消息
//!
//! 协议：`\\.\pipe\GlimpsePreviewPipe`，每条消息 = 一个 JSON 行
//!
//! ```json
//! { "path": "C:\\foo.png", "hwnd": 12345, "rect_x": 100, ... }
//! ```

use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tracing::{debug, error, info, warn};

const PIPE_NAME: &str = r"\\.\pipe\GlimpsePreviewPipe";

/// DLL → 主进程的触发消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerMessage {
    pub path: String,
    pub hwnd: isize,
    pub rect_x: i32,
    pub rect_y: i32,
    pub rect_w: i32,
    pub rect_h: i32,
}

/// 启动命名管道服务器（每收到一个连接就 spawn 一个 task）
pub async fn serve(app: AppHandle) -> Result<()> {
    info!(pipe = %PIPE_NAME, "starting preview pipe server");

    loop {
        match ServerOptions::new()
            .create(NamedPipeServer::new, PIPE_NAME)
            .await
        {
            Ok(server) => {
                let app = app.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(server, app).await {
                        warn!("pipe connection error: {e:#}");
                    }
                });
            }
            Err(e) => {
                error!("failed to create pipe instance: {e:#}");
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }
}

async fn handle_connection(server: NamedPipeServer, app: AppHandle) -> Result<()> {
    // 等客户端连上
    server
        .connect()
        .await
        .context("pipe connect failed")?;

    let reader = BufReader::new(server);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        match serde_json::from_str::<TriggerMessage>(&line) {
            Ok(msg) => {
                debug!(?msg, "received trigger");
                dispatch(&app, msg);
            }
            Err(e) => warn!(error = %e, line = %line, "malformed pipe message"),
        }
    }
    Ok(())
}

/// 把消息转成 IPC 事件投给前端
fn dispatch(app: &AppHandle, msg: TriggerMessage) {
    if msg.path.is_empty() {
        warn!("trigger missing path");
        return;
    }

    // 找预览窗口并定位
    if let Some(window) = app.get_webview_window("preview") {
        // 设置窗口位置 + 大小（对齐 Explorer 中的预览区域）
        let _ = window.set_position(tauri::PhysicalPosition::new(msg.rect_x, msg.rect_y));
        let _ = window.set_size(tauri::PhysicalSize::new(
            msg.rect_w.max(200) as u32,
            msg.rect_h.max(150) as u32,
        ));
        let _ = window.show();
        let _ = window.set_focus();

        // 通知前端
        let emit_result = app.emit_to("preview", "shell-trigger", msg.path.clone());
        if let Err(e) = emit_result {
            error!("emit shell-trigger failed: {e:#}");
        }
    } else {
        warn!("preview window not registered");
    }
}

/// 应用启动时确保管道服务端被持有
pub fn spawn(app: AppHandle) {
    tokio::spawn(async move {
        let app = Arc::new(app);
        if let Err(e) = serve((*app).clone()).await {
            error!("pipe server crashed: {e:#}");
        }
    });
}