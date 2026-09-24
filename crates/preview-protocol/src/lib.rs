//! 前后端共享的 IPC 协议

use serde::{Deserialize, Serialize};

/// 前端调用后端的预览请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewRequest {
    pub path: String,
    pub max_width: u32,
    pub max_height: u32,
}

/// 后端返回的预览载荷
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
    Unsupported {
        reason: String,
    },
}

/// Shell Extension DLL → 主进程的命名管道消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellTriggerMessage {
    pub path: String,
    pub hwnd: isize,
    pub rect_x: i32,
    pub rect_y: i32,
    pub rect_w: i32,
    pub rect_h: i32,
}