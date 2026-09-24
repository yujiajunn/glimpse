//! Glimpse 应用入口
//!
//! 进程模型：
//! - 主进程（Tauri + WebView2）→ UI、预览调度、缓存
//! - Shell Extension DLL（独立进程）→ 注入 Explorer，接收 IPreviewHandler 调用
//! - 进程间通信：命名管道 `\\.\pipe\GlimpsePreviewPipe`

mod cache;
mod preview;
mod shell;

use std::sync::Arc;

use anyhow::Context;
use tauri::Manager;
use thiserror::Error;
use tracing::{error, info};

use crate::cache::ThumbnailCache;
use crate::preview::PreviewDispatcher;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("cache init failed: {0}")]
    CacheInit(#[from] anyhow::Error),
    #[error("tauri runtime: {0}")]
    Tauri(#[from] tauri::Error),
}

/// 全局应用状态（注入到 Tauri context）
pub struct AppState {
    pub dispatcher: Arc<PreviewDispatcher>,
    pub cache: Arc<ThumbnailCache>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    if let Some(code) = handle_cli_args() {
        std::process::exit(code);
    }

    if let Err(e) = run_app() {
        error!("fatal: {e:#}");
        std::process::exit(1);
    }
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("glimpse=info")),
        )
        .with_target(false)
        .with_level(true)
        .init();
}

/// 解析 CLI 参数；返回 Some(exit_code) 表示该退出，否则进入 GUI
fn handle_cli_args() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    let Some(cmd) = args.get(1).map(String::as_str) else {
        return None;
    };

    let result = match cmd {
        "--install" => {
            info!("installing shell extension");
            shell::install()
        }
        "--uninstall" => {
            info!("uninstalling shell extension");
            shell::uninstall()
        }
        "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => {
            eprintln!("unknown command: {cmd}");
            print_help();
            std::process::exit(2);
        }
    };

    match result {
        Ok(()) => Some(0),
        Err(e) => {
            error!("{cmd} failed: {e:#}");
            Some(1)
        }
    }
}

fn print_help() {
    println!(
        "Glimpse — file preview utility (Windows)\n\n\
         USAGE:\n  glimpse [COMMAND]\n\n\
         COMMANDS:\n  --install    register shell extension\n  \
         --uninstall  unregister shell extension\n  \
         (no args)     launch GUI"
    );
}

fn run_app() -> Result<(), AppError> {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let cache_dir = app
                .path()
                .app_cache_dir()
                .context("resolve cache dir")?;
            std::fs::create_dir_all(&cache_dir).context("create cache dir")?;

            let dispatcher = Arc::new(PreviewDispatcher::new(cache_dir.clone()));
            let cache = Arc::new(ThumbnailCache::open(cache_dir.join("thumbs"))?);

            // 装载 plugins/*.json（用户级 + 应用级）
            let plugins_dir = app
                .path()
                .app_config_dir()
                .context("resolve config dir")?
                .join("plugins");
            let _ = preview::plugins::load_plugins(&dispatcher, &plugins_dir);

            app.manage(AppState { dispatcher, cache });

            // 启动命名管道服务器，接收 Shell Extension DLL 的触发事件
            shell::pipe_server::spawn(app.handle().clone());

            info!("Glimpse ready");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 关闭仅隐藏，进程常驻以便下次空格秒开
                window.hide().ok();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            preview::cmd_show_preview,
            preview::cmd_hide_preview,
            preview::cmd_load_image,
            preview::cmd_load_pdf_page,
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}