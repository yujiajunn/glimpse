//! Glimpse 预览窗口应用
//!
//! 进程：单实例，常驻后台
//! - 监听命名管道 \\.\pipe\GlimpsePreviewPipe
//! - 收到路径 → 创建 Win32 窗口 → 解码文件 → BitBlt 显示
//! - Esc / 窗口失焦 → 关闭窗口（进程不退出）

mod preview;
mod window;

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

const PIPE_NAME: &str = r"\\.\pipe\GlimpsePreviewPipe";

/// DLL → App 的触发消息
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TriggerMessage {
    path: String,
    #[serde(default)]
    hwnd: isize,
    #[serde(default)]
    rect_x: i32,
    #[serde(default)]
    rect_y: i32,
    #[serde(default)]
    rect_w: i32,
    #[serde(default)]
    rect_h: i32,
}

/// 全局预览状态：保证同时只有一个预览窗口
/// 用 parking_lot::Mutex（不是 tokio）因为 HWND 含裸指针不是 Send/Sync
static ACTIVE_PREVIEW: once_cell::sync::Lazy<parking_lot::Mutex<Option<window::PreviewWindow>>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(None));

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    init_tracing();
    info!("Glimpse preview app starting");

    // 处理 CLI：--install / --uninstall（写注册表，让 Explorer 加载我们的 DLL）
    let args: Vec<String> = std::env::args().collect();
    if let Some(cmd) = args.get(1).map(String::as_str) {
        match cmd {
            "--install" => {
                shell_install()?;
                return Ok(());
            }
            "--uninstall" => {
                shell_uninstall()?;
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {}
        }
    }

    // 主循环：监听命名管道
    run_pipe_server().await
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("glimpse=info")),
        )
        .with_target(false)
        .init();
}

fn print_help() {
    println!(
        "Glimpse — file preview utility\n\n\
         USAGE:\n  glimpse [COMMAND]\n\n\
         COMMANDS:\n  --install    register Shell Extension DLL\n  \
         --uninstall  unregister\n  \
         (no args)     listen on named pipe (default)"
    );
}

/// 安装 / 卸载：写注册表让 Explorer 加载 glimpse_previewhandler.dll
fn shell_install() -> Result<()> {
    const CLSID: &str = "{d7f66eff-9453-4de1-93f1-4290aee16e95}";
    const DLL_NAME: &str = "glimpse_previewhandler.dll";

    let hkcr = winreg::RegKey::predef(winreg::enums::HKEY_CLASSES_ROOT);
    let (clsid_key, _) = hkcr.create_subkey(format!(r"CLSID\{CLSID}"))?;
    clsid_key.set_value("", &"Glimpse Preview Handler")?;
    let (server, _) = hkcr.create_subkey(format!(r"CLSID\{CLSID}\InProcServer32"))?;
    server.set_value("", &DLL_NAME)?;
    server.set_value("ThreadingModel", &"Apartment")?;

    let (handler, _) = hkcr.create_subkey(r"*\shellex\{8895b1c6-b41f-4c1c-a562-0d564250836f}")?;
    handler.set_value("Glimpse", &CLSID)?;

    println!("Shell extension installed. Restart Explorer to pick up changes.");
    Ok(())
}

fn shell_uninstall() -> Result<()> {
    const CLSID: &str = "{d7f66eff-9453-4de1-93f1-4290aee16e95}";

    let hkcr = winreg::RegKey::predef(winreg::enums::HKEY_CLASSES_ROOT);
    let _ = hkcr.delete_subkey_all(r"*\shellex\{8895b1c6-b41f-4c1c-a562-0d564250836f}\Glimpse");
    let _ = hkcr.delete_subkey_all(format!(r"CLSID\{CLSID}"));
    println!("Shell extension unregistered.");
    Ok(())
}

/// 主循环：每个连接 spawn 一个 task
async fn run_pipe_server() -> Result<()> {
    info!(pipe = %PIPE_NAME, "starting pipe server");

    loop {
        match ServerOptions::new().create(PIPE_NAME) {
            Ok(server) => {
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(server).await {
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

async fn handle_connection(server: NamedPipeServer) -> Result<()> {
    server.connect().await.context("pipe connect")?;
    let reader = BufReader::new(server);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        match serde_json::from_str::<TriggerMessage>(&line) {
            Ok(msg) => {
                info!(path = %msg.path, "preview trigger");
                show_preview(msg).await;
            }
            Err(e) => warn!(error = %e, "malformed pipe message"),
        }
    }
    Ok(())
}

/// 处理一条预览请求：在主线程上创建窗口（Win32 要求）
async fn show_preview(msg: TriggerMessage) {
    let path = PathBuf::from(&msg.path);
    if !path.exists() {
        warn!("file not found: {}", path.display());
        return;
    }

    // 兜底用的 path 拷贝（decode 任务会 move 原 path）
    let path_for_fallback = path.clone();

    // 解码文件（CPU 密集，可以放后台）
    let decode_result = tokio::task::spawn_blocking(move || {
        preview::decode_for_preview(&path)
    })
    .await;

    let content = match decode_result {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => {
            error!("decode failed: {e:#}");
            open_with_default_app(&path_for_fallback);
            return;
        }
        Err(e) => {
            error!("decode task panicked: {e}");
            open_with_default_app(&path_for_fallback);
            return;
        }
    };

    // Win32 必须在创建窗口的线程上 —— 用 spawn_blocking 包到独立线程
    let _ = tokio::task::spawn_blocking(move || {
        if let Err(e) = show_window(content, msg) {
            error!("show_window failed: {e:#}");
        }
    })
    .await;
}

/// 在独立线程上跑一个消息循环，创建窗口 + 显示 bitmap + 等用户关闭
fn show_window(content: preview::PreviewContent, msg: TriggerMessage) -> Result<()> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, HMODULE};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, RegisterClassExW, GetMessageW, DispatchMessageW, TranslateMessage,
        MSG, WNDCLASSEXW, CS_HREDRAW, CS_VREDRAW, WS_POPUPWINDOW, WS_VISIBLE, WS_EX_TOPMOST,
        DefWindowProcW, PostQuitMessage, CS_OWNDC,
    };

    // 窗口尺寸：跟 Explorer 给我们 rect 一样
    let (w, h) = if msg.rect_w >= 200 && msg.rect_h >= 150 {
        (msg.rect_w, msg.rect_h)
    } else {
        (640, 480)
    };
    let (x, y) = (msg.rect_x, msg.rect_y);

    // 注册窗口类（用原子名字段做 unique）
    static CLASS_NAME: once_cell::sync::Lazy<Vec<u16>> = once_cell::sync::Lazy::new(|| {
        widestring_to_wide("GlimpsePreviewWindow")
    });

    let hinstance = unsafe { GetModuleHandleW(None) }
        .map_err(|e| anyhow::anyhow!("GetModuleHandleW: {e}"))?;
    let hinstance: HMODULE = HINSTANCE(hinstance.0);

    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
        lpfnWndProc: Some(wnd_proc),
        hInstance: hinstance,
        lpszClassName: windows::core::PCWSTR(CLASS_NAME.as_ptr()),
        ..Default::default()
    };
    let atom = unsafe { RegisterClassExW(&wc) };
    if atom == 0 {
        return Err(anyhow::anyhow!("RegisterClassExW failed"));
    }

    let title = widestring_to_wide("Glimpse Preview");
    let hwnd_res = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST,
            windows::core::PCWSTR(CLASS_NAME.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            WS_POPUPWINDOW | WS_VISIBLE,
            x, y, w, h,
            HWND(std::ptr::null_mut()),
            None,
            hinstance,
            None,
        )
    };
    let hwnd = hwnd_res.map_err(|e| anyhow::anyhow!("CreateWindowExW: {e}"))?;
    if hwnd.0.is_null() {
        return Err(anyhow::anyhow!("CreateWindowExW returned null HWND"));
    }

    // 把 PreviewContent 关联到窗口（用窗口 user data 槽位 — 简化为全局静态）
    let mut guard = ACTIVE_PREVIEW.try_lock().map_err(|e| anyhow::anyhow!("{e}"))?;
    // 关掉旧窗口（如果有）
    if let Some(old) = guard.take() {
        old.close();
    }
    *guard = Some(window::PreviewWindow::new(hwnd, content));

    // 消息循环
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
            if msg.message == 0x0012 {
                // WM_QUIT
                break;
            }
        }
    }

    // 关闭并清理
    if let Some(mut w) = guard.take() {
        w.close();
    }
    Ok(())
}

unsafe extern "system" fn wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::{LRESULT, WPARAM, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, PostQuitMessage, WM_PAINT, WM_KEYDOWN, WM_LBUTTONDOWN, WM_CLOSE,
        WM_DESTROY, WM_ERASEBKGND,
    };

    match msg {
        WM_PAINT => {
            // 委托给 PreviewWindow（它持有 bitmap）
            let guard = ACTIVE_PREVIEW.try_lock();
            if let Ok(g) = guard {
                if let Some(w) = g.as_ref() {
                    w.paint(hwnd);
                }
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1), // 我们自己画
        WM_KEYDOWN => {
            if wparam.0 == 0x1B {
                // VK_ESCAPE
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_CLOSE => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// `widestring::U16String` 替代品（避免加依赖）
fn widestring_to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 调系统默认应用打开文件
fn open_with_default_app(path: &std::path::Path) {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        ShellExecuteW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC,
    };
    let path_str = path.to_string_lossy().to_string();
    unsafe {
        let verb = "open\0".encode_utf16().collect::<Vec<u16>>();
        let file = (path_str + "\0").encode_utf16().collect::<Vec<u16>>();
        let _ = ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            None,
            None,
            SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        );
    }
}