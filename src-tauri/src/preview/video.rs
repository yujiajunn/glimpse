use anyhow::{Context, Result};
use std::path::Path;

use super::{PreviewHandler, PreviewPayload, PreviewRequest};

/// 视频 / 音频预览：返回本地文件路径，让前端用 HTML5 <video>/<audio> 播放
pub struct VideoPreview;

impl VideoPreview {
    pub fn new() -> Self {
        Self
    }

    fn is_video(ext: &str) -> bool {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v"
        )
    }

    fn is_audio(ext: &str) -> bool {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "mp3" | "flac" | "wav" | "aac" | "ogg" | "m4a" | "opus"
        )
    }
}

impl Default for VideoPreview {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PreviewHandler for VideoPreview {
    fn name(&self) -> &'static str {
        "video"
    }

    fn matches(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| Self::is_video(e) || Self::is_audio(e))
            .unwrap_or(false)
    }

    async fn handle(&self, req: &PreviewRequest) -> Result<PreviewPayload> {
        let ext = req
            .path
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        // 用 ffmpeg-next 取元数据（时长、分辨率）
        // 简化版：直接返回文件 URL，让前端播放
        // 用 file:// URL 让前端 <video> / <audio> 直接播
let url = format!("file:///{}", req.path.replace('\\', "/"));

        // TODO: 调 ffmpeg-next 探针得到 duration / width / height
        let duration_sec = 0.0;
        let width = 0;
        let height = 0;

        if Self::is_video(&ext) {
            Ok(PreviewPayload::Video {
                url,
                duration_sec,
                width,
                height,
            })
        } else {
            Ok(PreviewPayload::Audio {
                url,
                duration_sec,
            })
        }
        .with_context(|| format!("video/audio handler failed for: {}", req.path))
    }
}