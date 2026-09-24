# Glimpse

类 Seer（人之初）的 Windows 文件预览工具，Rust + Tauri 2 实现。

[![CI](https://github.com/yujiajunn/glimpse/actions/workflows/ci.yml/badge.svg)](https://github.com/yujiajunn/glimpse/actions/workflows/ci.yml)
[![Release](https://github.com/yujiajunn/glimpse/actions/workflows/release.yml/badge.svg)](https://github.com/yujiajunn/glimpse/actions/workflows/release.yml)

> **无需本地装工具链** — 推送 tag 到 GitHub，Action 自动编译 + 出安装包 + 创建 Release。

## 架构

```
┌────────────────────────────────────────────────────────────────┐
│  Windows Explorer                                              │
│  ┌──────────────────────┐                                       │
│  │ ipreviewhandler DLL │ ← IPreviewHandler COM（x86 + x64）   │
│  └──────────┬───────────┘                                       │
└─────────────┼──────────────────────────────────────────────────┘
              │ 命名管道 \\.\pipe\GlimpsePreviewPipe
              ▼
┌────────────────────────────────────────────────────────────────┐
│  src-tauri (主进程, Tauri 2 + WebView2)                        │
│  ┌────────────────────────────────────────────────┐            │
│  │ PreviewDispatcher                              │            │
│  │  ├ DirectoryPreview    目录                    │            │
│  │  ├ ArchivePreview      ZIP/TAR                 │            │
│  │  ├ OfficePreview       DOCX/XLSX/PPTX          │            │
│  │  ├ FontPreview         TTF/OTF                 │            │
│  │  ├ PdfPreview          PDF                     │            │
│  │  ├ VideoPreview        MP4/MKV/MP3/FLAC        │            │
│  │  ├ HtmlPreview         HTML/Markdown           │            │
│  │  ├ ImagePreview        PNG/JPG/HEIC/RAW        │            │
│  │  └ TextPreview         源码/MD/JSON/CSV        │            │
│  │                                                │            │
│  │ ThumbnailCache (sled) 缩略图缓存               │            │
│  │ PluginLoader     扫描 plugins/*.json           │            │
│  │ Shell installer   写注册表 CLSID + PreviewHandler│           │
│  └────────────────────────────────────────────────┘            │
└────────────────────────────────────────────────────────────────┘
```

## 支持的文件

| 类型 | 扩展名 | 库 |
|---|---|---|
| 图片 | png/jpg/webp/gif/bmp/tiff/heic/avif/raw/dng | `image` crate |
| PDF | pdf | `pdfium-render` |
| 视频 | mp4/mkv/avi/mov/webm | ffmpeg / HTML5 `<video>` |
| 音频 | mp3/flac/wav/aac/ogg | HTML5 `<audio>` |
| Office | docx/xlsx/pptx | `docx-rs` + `calamine` |
| 字体 | ttf/otf/woff | `ttf-parser` |
| 压缩包 | zip/tar | `zip` + `tar` |
| 目录 | — | `std::fs` |
| 文本 | txt/log/md/json/源码 | 直接读 + CodeMirror |

## 构建

```bash
# 工具链
rustup default stable
nvm install 20 && nvm use 20
cargo install tauri-cli --version "^2"
rustup target add i686-pc-windows-msvc x86_64-pc-windows-msvc

# 前端依赖
cd ui && npm install && cd ..

# 开发
cd src-tauri && cargo tauri dev

# 生产构建
cd src-tauri && cargo tauri build

# 单独构建 Shell Extension DLL
cd crates/ipreviewhandler
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target i686-pc-windows-msvc
```

## 安装 / 卸载

```bash
glimpse.exe --install     # 注册 Shell Extension
glimpse.exe --uninstall   # 注销
```

## 插件

把 JSON 文件丢到 `%APPDATA%\com.local.glimpse\plugins\`：

```json
{
  "name": "my-custom",
  "version": "1.0",
  "extensions": [
    { "ext": "xyz", "handler": "text" },
    { "ext": "abc", "handler": "image" }
  ]
}
```

可用 `handler` 名称：`image / pdf / video / text / html / office / font / archive / directory`。

## 已知 TODO

| TODO | 文件 | 优先级 |
|---|---|---|
| FFmpeg 取视频真实元数据（duration / 尺寸） | `src-tauri/src/preview/video.rs` | 🟡 中 |
| 字体二进制传给前端用 FontFace 实时渲染 | `src-tauri/src/preview/font.rs` + FontView | 🟡 中 |
| PPTX 真实幻灯片渲染（目前只取文本）| `src-tauri/src/preview/office.rs` | 🟢 低 |
| 7z/RAR 解压支持 | `src-tauri/src/preview/archive.rs` | 🟢 低 |
| 真实 GUID 替换占位 | `crates/ipreviewhandler/src/lib.rs` | 🔴 高 |
| 缩略图缓存按 mtime 失效 | `src-tauri/src/cache/mod.rs` | 🟢 低 |
| 应用图标 | `src-tauri/icons/icon.ico` | 🟡 中 |