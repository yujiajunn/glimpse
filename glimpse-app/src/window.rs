//! Win32 预览窗口绘图：在指定 HDC 上画预览内容

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, DeleteDC, DeleteObject, EndPaint, GetObjectW,
    PatBlt, SelectObject, SetStretchBltMode, StretchBlt, BITMAP, BITMAPINFOHEADER, HALFTONE,
    PAINTSTRUCT, SRCCOPY, WHITENESS,
};
// DrawTextW + DT_* 在 windows-rs 0.58 的 Graphics::Gdi 模块
use windows::Win32::Graphics::Gdi::{
    DrawTextW, DT_BOTTOM, DT_CENTER, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_TOP,
    DT_VCENTER, DT_WORDBREAK,
};

use crate::preview::PreviewContent;

/// 在给定的 HDC 上绘制预览内容
pub fn draw_content(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    content: &PreviewContent,
    win_w: i32,
    win_h: i32,
) {
    if hdc.0.is_null() {
        return;
    }

    let mut rect = RECT::default();
    let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::GetClientRect(
        windows::Win32::Foundation::HWND(std::ptr::null_mut()),
        &mut rect,
    ) };
    // 使用传入的 win_w/win_h 优先
    let (win_w, win_h) = if win_w > 0 && win_h > 0 {
        (win_w, win_h)
    } else {
        (rect.right - rect.left, rect.bottom - rect.top)
    };
    if win_w <= 0 || win_h <= 0 {
        return;
    }

    // 黑色背景
    let _ = unsafe { PatBlt(hdc, 0, 0, win_w, win_h, WHITENESS) };

    match content {
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
                let info = format!("  {}×{}  {:.1}s", width, height, duration_sec);
                draw_overlay_text(hdc, &info, win_w, win_h);
            } else {
                draw_text(
                    hdc,
                    title,
                    &vec![format!("[视频] {}×{} {:.1}s", width, height, duration_sec)],
                    win_w,
                    win_h,
                );
            }
        }
        PreviewContent::Unsupported { reason } => {
            draw_text(hdc, "Glimpse", &vec![reason.clone()], win_w, win_h);
        }
    }
}

/// 把 HBITMAP 等比缩放画到 (win_w, win_h) 中央
unsafe fn draw_bitmap_centered(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    hbm: windows::Win32::Foundation::HWND,
    bw: i32,
    bh: i32,
    win_w: i32,
    win_h: i32,
) {
    if bw <= 0 || bh <= 0 || win_w <= 0 || win_h <= 0 {
        return;
    }

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
    let _ = SetStretchBltMode(hdc, HALFTONE);
    // HBITMAP 选进 mem_dc
    let old = SelectObject(mem_dc, hbm.into());
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

    let body: String = lines.join("\n");
    let body_truncated = if body.len() > 8192 {
        let mut s: String = body.chars().take(8192).collect();
        s.push_str("\n... [截断] ...");
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