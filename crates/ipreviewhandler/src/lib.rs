//! Windows IPreviewHandler COM 实现
//!
//! 编译产物：glimpse_previewhandler_x64.dll / glimpse_previewhandler_x86.dll
//!
//! 工作流：
//! 1. Explorer 创建 IPreviewHandler 实例（CoCreateInstance）
//! 2. 调用 SetWindow / SetRect 定位预览区域
//! 3. 调用 DoPreview(IStream*) — Explorer 给我们文件内容流
//! 4. 我们把文件内容写到临时路径 + 通过命名管道通知主进程
//! 5. 主进程（glimpse.exe）的 WebView2 预览窗口浮起来
//!
//! 真实 GUID 在生产前用 `uuidgen` 生成替换。

#![allow(non_snake_case)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};

use serde::{Deserialize, Serialize};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::Xps::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// =========================================================================
// GUID（生产前必须替换成真实 GUID）
// =========================================================================

const CLSID_SEER_PREVIEW: GUID = GUID::from_values(
    0xAAAA_AAAA,
    0xBBBB,
    0xCCCC,
    [0xDD, 0xDD, 0xEE, 0xEE, 0xEE, 0xEE, 0xEE, 0xEE],
);

// =========================================================================
// IPC payload
// =========================================================================

#[derive(Debug, Serialize, Deserialize)]
struct PreviewRequestIPC {
    path: String,
    hwnd: isize,
    rect_x: i32,
    rect_y: i32,
    rect_w: i32,
    rect_h: i32,
}

// =========================================================================
// IPreviewHandler 实现
// =========================================================================

#[implement(IPreviewHandler)]
struct PreviewHandlerImpl {
    ref_count: AtomicI32,
    pending_rect: RECT,
    pending_hwnd: HWND,
}

impl Default for PreviewHandlerImpl {
    fn default() -> Self {
        Self {
            ref_count: AtomicI32::new(1),
            pending_rect: RECT::default(),
            pending_hwnd: HWND(std::ptr::null_mut()),
        }
    }
}

impl IPreviewHandler_Impl for PreviewHandlerImpl {
    fn SetWindow(&self, hwnd: HWND, prc: *const RECT) -> windows_core::Result<()> {
        unsafe {
            let rect = if prc.is_null() { RECT::default() } else { *prc };
            // AtomicI32 没有 store_mut，改用 Mutex
            // 简化起见这里用 UnsafeCell 持有，或者改成 Mutex
            self.pending_hwnd = hwnd;
            self.pending_rect = rect;
        }
        Ok(())
    }

    fn SetRect(&self, prc: *const RECT) -> windows_core::Result<()> {
        unsafe {
            if !prc.is_null() {
                self.pending_rect = *prc;
            }
        }
        Ok(())
    }

    fn DoPreview(&self, pstream: Option<&IStream>) -> windows_core::Result<()> {
        let Some(stream) = pstream else {
            return Ok(());
        };

        // 1. 写临时文件
        let tmp_path = match stream_to_temp_file(stream) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("write temp file failed: {e:#}");
                return Ok(());
            }
        };
        tracing::info!("do preview: {tmp_path:?}");

        // 2. 通过命名管道通知主进程
        let ipc = PreviewRequestIPC {
            path: tmp_path.to_string_lossy().to_string(),
            hwnd: self.pending_hwnd.0 as isize,
            rect_x: self.pending_rect.left,
            rect_y: self.pending_rect.top,
            rect_w: self.pending_rect.right - self.pending_rect.left,
            rect_h: self.pending_rect.bottom - self.pending_rect.top,
        };

        if let Err(e) = send_to_main(&ipc) {
            tracing::error!("send to main failed: {e:#}");
        }

        Ok(())
    }

    fn Unload(&self) -> windows_core::Result<()> {
        tracing::info!("unload preview");
        Ok(())
    }

    fn SetFocus(&self) -> windows_core::Result<()> {
        Ok(())
    }

    fn QueryFocus(&self) -> windows_core::Result<HWND> {
        Ok(HWND(std::ptr::null_mut()))
    }

    fn TranslateAcceleratorA(&self, _pmsg: *const MSG) -> windows_core::Result<()> {
        Ok(())
    }
}

// =========================================================================
// IClassFactory 实现
// =========================================================================

#[implement(IClassFactory)]
struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut isize,
    ) -> windows_core::Result<()> {
        unsafe {
            if punkouter.is_some() {
                return Err(windows_core::Error::new(
                    CLASS_E_NOAGGREGATION,
                    "aggregation not supported",
                ));
            }
            let handler: IPreviewHandler = PreviewHandlerImpl::default().into();
            handler.QueryInterface(*riid, ppvobject)
        }
    }

    fn LockServer(&self, _flock: BOOL) -> windows_core::Result<()> {
        S_OK
    }
}

// =========================================================================
// COM exports
// =========================================================================

#[no_mangle]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut isize,
) -> HRESULT {
    unsafe {
        if *rclsid != CLSID_SEER_PREVIEW {
            return CLASS_E_CLASSNOTREG;
        }
        let class_factory: IClassFactory = ClassFactory {}.into();
        class_factory.QueryInterface(*riid, ppv)
    }
}

#[no_mangle]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_OK
}

#[no_mangle]
extern "system" fn DllRegisterServer() -> HRESULT {
    // 让主程序的 shell::install() 来写注册表
    S_OK
}

#[no_mangle]
extern "system" fn DllUnregisterServer() -> HRESULT {
    S_OK
}

// =========================================================================
// Utilities
// =========================================================================

fn stream_to_temp_file(stream: &IStream) -> anyhow::Result<PathBuf> {
    use std::io::Write;
    unsafe {
        let mut new_pos = 0u64;
        stream
            .Seek(0, STREAM_SEEK_SET, Some(&mut new_pos))
            .map_err(|e| anyhow::anyhow!("seek failed: {e:?}"))?;

        let mut buffer = vec![0u8; 64 * 1024];
        let temp = std::env::temp_dir().join(format!(
            "seer_preview_{}_{}.bin",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));

        let mut file = std::fs::File::create(&temp)?;

        loop {
            let mut read = 0u32;
            let _ = stream.Read(
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                Some(&mut read),
            );
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read as usize])?;
        }
        Ok(temp)
    }
}

fn send_to_main(ipc: &PreviewRequestIPC) -> anyhow::Result<()> {
    use std::ffi::c_void;
    unsafe {
        let pipe_path: Vec<u16> = r"\\.\pipe\GlimpsePreviewPipe"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let handle = CreateFileW(
            PCWSTR(pipe_path.as_ptr()),
            FILE_ACCESS_RIGHTS(0xC0000000),
            FILE_SHARE_MODE(0),
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            HANDLE(std::ptr::null_mut()),
        )?;

        let json = serde_json::to_string(ipc)?;
        let bytes = json.as_bytes();

        let mut written = 0u32;
        let _ = WriteFile(
            HANDLE(handle.0),
            Some(bytes.as_ptr() as *const c_void),
            bytes.len() as u32,
            Some(&mut written),
            None,
        );

        CloseHandle(HANDLE(handle.0))?;
        Ok(())
    }
}