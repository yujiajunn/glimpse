//! Glimpse Shell Extension DLL
//!
//! 极简版：用 raw COM 接口实现 IPreviewHandler + IClassFactory。
//! 不依赖 #[implement] 宏，直接用 unsafe impl QueryInterface / AddRef / Release。
//!
//! CLSID: {d7f66eff-9453-4de1-93f1-4290aee16e95}
//! 注册位置：HKCR\*\shellex\{8895b1c6-b41f-4c1c-a562-0d564250836f}\Glimpse

#![allow(non_snake_case)]

use std::path::PathBuf;

use windows::core::*;
use windows::Win32::System::Com::*;

// IPreviewHandler vtable（按 windows-rs 0.62 顺序）
// 我们手写 vtable，不依赖 #[implement] 宏
pub unsafe extern "system" fn PreviewHandler_SetWindow(
    _this: *mut core::ffi::c_void,
    hwnd: HWND,
    _prc: *const core::ffi::c_void,
) -> HRESULT {
    tracing::info!(hwnd = ?hwnd, "PreviewHandler_SetWindow");
    S_OK
}
pub unsafe extern "system" fn PreviewHandler_SetRect(
    _this: *mut core::ffi::c_void,
    _prc: *const core::ffi::c_void,
) -> HRESULT {
    S_OK
}
pub unsafe extern "system" fn PreviewHandler_DoPreview(
    _this: *mut core::ffi::c_void,
    _pstream: *mut core::ffi::c_void,
) -> HRESULT {
    tracing::info!("DoPreview called");
    S_OK
}
pub unsafe extern "system" fn PreviewHandler_Unload(_this: *mut core::ffi::c_void) -> HRESULT {
    S_OK
}
pub unsafe extern "system" fn PreviewHandler_SetFocus(_this: *mut core::ffi::c_void) -> HRESULT {
    S_OK
}
pub unsafe extern "system" fn PreviewHandler_QueryFocus(
    _this: *mut core::ffi::c_void,
    phwnd: *mut HWND,
) -> HRESULT {
    unsafe {
        if !phwnd.is_null() {
            *phwnd = HWND(std::ptr::null_mut());
        }
    }
    S_OK
}

// CLSID
const CLSID_GLIMPSE_PREVIEW: GUID = GUID {
    data1: 0xd7f6_6eff,
    data2: 0x9453,
    data3: 0x4de1,
    data4: [0x93, 0xf1, 0x42, 0x90, 0xae, 0xe1, 0x6e, 0x95],
};

// ================================================================
// IClassFactory stub（只返回 E_NOTIMPL 占位）
// ================================================================

unsafe extern "system" fn ClassFactory_QueryInterface(
    _this: *mut core::ffi::c_void,
    riid: *const GUID,
    ppv: *mut *mut core::ffi::c_void,
) -> HRESULT {
    unsafe {
        if !ppv.is_null() {
            *ppv = std::ptr::null_mut();
        }
    }
    S_OK
}
unsafe extern "system" fn ClassFactory_AddRef(_this: *mut core::ffi::c_void) -> u32 {
    1
}
unsafe extern "system" fn ClassFactory_Release(_this: *mut core::ffi::c_void) -> u32 {
    1
}
unsafe extern "system" fn ClassFactory_CreateInstance(
    _this: *mut core::ffi::c_void,
    _punkouter: *mut core::ffi::c_void,
    riid: *const GUID,
    ppv: *mut *mut core::ffi::c_void,
) -> HRESULT {
    // 不实现真正的实例化 —— 让 Explorer 显示错误但不会崩
    unsafe {
        if !ppv.is_null() {
            *ppv = std::ptr::null_mut();
        }
    }
    E_NOINTERFACE
}
unsafe extern "system" fn ClassFactory_LockServer(
    _this: *mut core::ffi::c_void,
    _flock: BOOL,
) -> HRESULT {
    S_OK
}

const CLASS_FACTORY_VTBL: [*const core::ffi::c_void; 5] = [
    ClassFactory_QueryInterface as *const _,
    ClassFactory_AddRef as *const _,
    ClassFactory_Release as *const _,
    ClassFactory_CreateInstance as *const _,
    ClassFactory_LockServer as *const _,
];

#[repr(C)]
struct ClassFactoryObject {
    vtbl: *const [*const core::ffi::c_void; 5],
}

static CLASS_FACTORY_OBJECT: ClassFactoryObject = ClassFactoryObject {
    vtbl: &CLASS_FACTORY_VTBL,
};

// ================================================================
// DLL exports
// ================================================================

#[no_mangle]
pub extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    _riid: *const GUID,
    ppv: *mut *mut core::ffi::c_void,
) -> HRESULT {
    unsafe {
        if rclsid.is_null() || ppv.is_null() {
            return E_POINTER;
        }
        if *rclsid != CLSID_GLIMPSE_PREVIEW {
            return CLASS_E_CLASSNOTREG;
        }
        *ppv = &CLASS_FACTORY_OBJECT as *const _ as *mut _;
        S_OK
    }
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_OK
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    S_OK
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    S_OK
}