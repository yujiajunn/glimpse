//! Win32 预览窗口：BitBlt 已有 HBITMAP 到窗口
//!
//! 不依赖 Tauri/egui/iced，纯 windows-rs + GDI。

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, DeleteObject, EndPaint, PatBlt, PAINTSTRUCT, SRCCOPY, WHITENESS,
};
// DrawTextW + DT_* 常量实际在 Win32::Graphics::Gdi
use windows::Win32::Graphics::Gdi::{
    DrawTextW, DT_BOTTOM, DT_CENTER, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_TOP,
    DT_VCENTER, DT_WORDBREAK,
};

use crate::preview::PreviewContent;

/// 预览窗口包装
pub struct PreviewWindow {
    hwnd: HWND,
    #[allow(dead_code)]
    content: PreviewContent,
}

impl PreviewWindow {
    pub fn new(hwnd: HWND, content: PreviewContent) -> Self {
        Self { hwnd, content }
    }

    /// 调系统销毁窗口
    pub fn close(&mut self) {
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.hwnd);
            // HBITMAP / DC 在 PreviewContent drop 时一起释放（这里简单不释放）
        }
    }

    /// WM_PAINT 回调：把 bitmap 画到窗口上
    pub fn paint(&self, hwnd: HWND) {
        unsafe {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            if hdc.0.is_null() {
                return;
            }

            let mut rect = RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
            let win_w = rect.right - rect.left;
            let win_h = rect.bottom - rect.top;
            if win_w <= 0 || win_h <= 0 {
                EndPaint(hwnd, &mut ps);
                return;
            }

            // 黑色背景
            let _ = PatBlt(hdc, 0, 0, win_w, win_h, WHITENESS);

            match &self.content {
                PreviewContent::Bitmap {
                    handle, width, height, ..
                } => {
                    draw_bitmap_centered(hdc, *handle, *width, *height, win_w, win_h);
                }
                PreviewContent::Text { title, lines } => {
                    draw_text(hdc, title, lines, win_w, win_h);
                }
                PreviewContent::Video {
                    title,
                    bitmap,
                    bitmap_size,
                    duration_sec,
                    width,
                    height,
                } => {
                    if let (Some(bmp), Some((bw, bh))) = (bitmap, bitmap_size) {
                        draw_bitmap_centered(hdc, *bmp, *bw, *bh, win_w, win_h);
                        // 时长叠加文本
                        let info = format!("  {}×{}  {:.1}s", width, height, duration_sec);
                        draw_overlay_text(hdc, &info, win_w, win_h);
                    } else {
                        draw_text(hdc, title, &vec![format!("[视频] {}×{} {:.1}s", width, height, duration_sec)], win_w, win_h);
                    }
                }
                PreviewContent::Unsupported { reason } => {
                    draw_text(
                        hdc,
                        "Glimpse",
                        &vec![reason.clone()],
                        win_w,
                        win_h,
                    );
                }
            }

            EndPaint(hwnd, &mut ps);
        }
    }
}

impl Drop for PreviewWindow {
    fn drop(&mut self) {
        if let PreviewContent::Bitmap { handle, .. } = self.content {
            unsafe {
                let _ = DeleteObject(handle);
            }
        }
        if let PreviewContent::Video { bitmap: Some(hbm), .. } = self.content {
            unsafe {
                let _ = DeleteObject(hbm);
            }
        }
    }
}

/// 把 HBITMAP 等比缩放画到窗口中央
unsafe fn draw_bitmap_centered(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    hbm: HWND, // HBITMAP
    bw: i32,
    bh: i32,
    win_w: i32,
    win_h: i32,
) {
    use windows::Win32::Graphics::Gdi::{
        GetObjectW, BITMAP, CreateCompatibleDC, DeleteDC, StretchBlt, SetStretchBltMode,
        HALFTONE,
    };

    if bw <= 0 || bh <= 0 || win_w <= 0 || win_h <= 0 {
        return;
    }

    // 等比缩放到窗口内（保留 8px 边距）
    let avail_w = win_w - 16;
    let avail_h = win_h - 16;
    let scale = (avail_w as f32 / bw as f32).min(avail_h as f32 / bh as f32);
    let scale = scale.min(1.0);
    let (tw, th) = (((bw as f32) * scale) as i32, ((bh as f32) * scale) as i32);
    let (x, y) = ((win_w - tw) / 2, (win_h - th) / 2);

    let mem_dc = CreateCompatibleDC(hdc);
    if mem_dc.0.is_null() {
        return;
    }
    let old = SelectObject(mem_dc, hbm.into());
    let _ = SetStretchBltMode(hdc, HALFTONE);
    let _ = StretchBlt(hdc, x, y, tw, th, mem_dc, 0, 0, bw, bh, SRCCOPY);
    SelectObject(mem_dc, old);
    DeleteDC(mem_dc);
}

unsafe fn draw_text(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    title: &str,
    lines: &[String],
    win_w: i32,
    win_h: i32,
) {
    use windows::Win32::UI::WindowsAndMessaging::{DrawTextW, DT_LEFT, DT_TOP, DT_WORDBREAK};

    // 标题
    let mut s = String::from(title);
    s.push('\0');
    let wide: Vec<u16> = s.encode_utf16().collect();
    let mut rect = RECT {
        left: 16,
        top: 8,
        right: win_w - 16,
        bottom: 40,
    };
    let _ = DrawTextW(
        hdc,
        windows::core::PCWSTR(wide.as_ptr()),
        &mut rect,
        DT_LEFT | DT_TOP | DT_SINGLELINE,
    );

    // 内容（拼接成一段，用 word break）
    let body: String = lines.join("\n");
    let body_truncated = if body.len() > 8192 {
        let mut s: String = body.chars().take(8192).collect();
        s.push_str("\n... [内容被截断] ...");
        s
    } else {
        body
    };
    let mut s = body_truncated;
    s.push('\0');
    let wide: Vec<u16> = s.encode_utf16().collect();
    let mut rect = RECT {
        left: 16,
        top: 48,
        right: win_w - 16,
        bottom: win_h - 16,
    };
    let _ = DrawTextW(
        hdc,
        windows::core::PCWSTR(wide.as_ptr()),
        &mut rect,
        DT_LEFT | DT_TOP | DT_WORDBREAK,
    );
}

unsafe fn draw_overlay_text(hdc: windows::Win32::Graphics::Gdi::HDC, info: &str, win_w: i32, win_h: i32) {
    use windows::Win32::UI::WindowsAndMessaging::{DrawTextW, DT_BOTTOM, DT_RIGHT, DT_SINGLELINE};
    let mut s = String::from(info);
    s.push('\0');
    let wide: Vec<u16> = s.encode_utf16().collect();
    let mut rect = RECT {
        left: 0,
        top: win_h - 24,
        right: win_w - 8,
        bottom: win_h - 4,
    };
    let _ = DrawTextW(
        hdc,
        windows::core::PCWSTR(wide.as_ptr()),
        &mut rect,
        DT_BOTTOM | DT_RIGHT | DT_SINGLELINE,
    );
}