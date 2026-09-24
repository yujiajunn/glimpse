//! Windows 集成：注册表安装 + 命名管道服务端

pub mod pipe_server;

use anyhow::{Context, Result};
use tracing::info;
use winreg::enums::*;
use winreg::RegKey;

const CLSID_X64: &str = "{B5E8DAAC-1001-4F1E-AAAA-AAAAAAAAAAAA}";
const CLSID_X86: &str = "{B5E8DAAC-1002-4F1E-AAAA-AAAAAAAAAAAA}";
const FRIENDLY_NAME: &str = "Glimpse Preview Handler";

/// 把 COM 服务器 + PreviewHandler 关联写到注册表
pub fn install() -> Result<()> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    register_clsid(&hkcr, CLSID_X64, "glimpse_previewhandler_x64.dll")?;
    register_clsid(&hkcr, CLSID_X86, "glimpse_previewhandler_x86.dll")?;

    // 关联到所有文件类型（GUID 8895b1c6-b41f-4c1c-a562-0d564250836f = IPreviewHandler）
    let (handler, _) = hkcr
        .create_subkey(r"*\shellex\{8895b1c6-b41f-4c1c-a562-0d564250836f}")
        .context("create PreviewHandlers key")?;
    handler
        .set_value("Glimpse", &CLSID_X64)
        .context("set Glimpse CLSID")?;

    info!("shell extension installed");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    let _ = hkcr.delete_subkey_all(r"*\shellex\{8895b1c6-b41f-4c1c-a562-0d564250836f}\Glimpse");
    let _ = hkcr.delete_subkey_all(format!(r"CLSID\{CLSID_X64}"));
    let _ = hkcr.delete_subkey_all(format!(r"CLSID\{CLSID_X86}"));

    info!("shell extension uninstalled");
    Ok(())
}

fn register_clsid(hkcr: &RegKey, clsid: &str, dll_name: &str) -> Result<()> {
    let (clsid_key, _) = hkcr
        .create_subkey(format!(r"CLSID\{clsid}"))
        .with_context(|| format!("create CLSID {clsid}"))?;
    clsid_key.set_value("", &FRIENDLY_NAME)?;

    let (server, _) = hkcr.create_subkey(format!(r"CLSID\{clsid}\InProcServer32"))?;
    server.set_value("", &dll_name)?;
    server.set_value("ThreadingModel", &"Apartment")?;

    Ok(())
}