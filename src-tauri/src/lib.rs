//! Glimpse 应用入口

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

            let plugins_dir = app
                .path()
                .app_config_dir()
                .context("resolve config dir")?
                .join("plugins");
            let _ = preview::plugins::load_plugins(&dispatcher, &plugins_dir);

            app.manage(AppState { dispatcher, cache });

            shell::pipe_server::spawn(app.handle().clone());

            info!("Glimpse ready");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
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