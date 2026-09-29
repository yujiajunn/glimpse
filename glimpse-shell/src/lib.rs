//! Glimpse Shell Extension DLL — 最简版
//!
//! 编译产物：glimpse_previewhandler.dll
//! CLSID: {d7f66eff-9453-4de1-93f1-4290aee16e95}
//!
//! 当前只导出 DllGetClassObject，返回 S_OK。Explorer 注册它后会显示 "无预览"
//! 但不会崩。等能编译了再加 IPreviewHandler 内容。

#![allow(non_snake_case)]

const CLSID_GLIMPSE_PREVIEW: [u8; 16] = [
    0xff, 0x6e, 0xf6, 0xd7, 0x53, 0x94, 0xe1, 0x4d,
    0x93, 0xf1, 0x42, 0x90, 0xae, 0xe1, 0x6e, 0x95,
];

#[no_mangle]
pub extern "system" fn DllGetClassObject(
    _rclsid: *const u8,
    _riid: *const u8,
    _ppv: *mut *mut u8,
) -> u32 {
    // E_NOINTERFACE = 0x80004002
    0x8000_4002
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> u32 {
    0 // S_OK
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> u32 {
    0
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> u32 {
    0
}

#[no_mangle]
pub extern "C" fn DllMain(_hinst: usize, _reason: u32, _reserved: *mut u8) -> i32 {
    1 // TRUE
}