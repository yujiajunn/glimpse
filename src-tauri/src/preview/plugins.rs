//! 插件系统：扫描 plugins/ 目录，加载 JSON 配置注册的扩展名 → 已有 handler 映射
//!
//! 真实场景可以用 `libloading` 加载外部 .dll/.dylib/.so，
//! 但跨平台 + 安全 (CI) 的复杂度太高；先做配置式插件。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn};

use super::dispatcher::PreviewDispatcher;

/// plugins/*.json 文件格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    /// 把这些扩展名路由到指定的内置 handler
    pub extensions: Vec<ExtensionRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRoute {
    pub ext: String,
    pub handler: String,
    pub priority: Option<i32>,
}

/// 从 plugins_dir 扫描所有 *.json 配置并应用到 dispatcher
pub fn load_plugins(dispatcher: &PreviewDispatcher, plugins_dir: &Path) -> Result<()> {
    if !plugins_dir.exists() {
        std::fs::create_dir_all(plugins_dir)?;
        return Ok(());
    }

    let entries = std::fs::read_dir(plugins_dir)?;
    let mut loaded = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let manifest: PluginManifest = match std::fs::read_to_string(&path)
            .context("read plugin")
            .and_then(|s| serde_json::from_str(&s).context("parse plugin"))
        {
            Ok(m) => m,
            Err(e) => {
                warn!(file = ?path, error = %e, "skipping invalid plugin");
                continue;
            }
        };

        info!(plugin = %manifest.name, version = %manifest.version, "loaded plugin");
        loaded += 1;

        // 把每个 ext→handler 映射写到一个全局 alias map，
        //  handler.matches() 进来时优先匹配别名表
        for route in &manifest.extensions {
            info!(ext = %route.ext, handler = %route.handler, "  + extension route");
            register_alias(&route.ext, &route.handler);
        }
    }

    tracing::info!(count = loaded, "plugin loader finished");
    Ok(())
}

/// 全局别名表：扩展名 → handler 名
static ALIASES: parking_lot::RwLock<std::collections::HashMap<String, String>> =
    parking_lot::RwLock::new(std::collections::HashMap::new());

pub fn register_alias(ext: &str, handler_name: &str) {
    ALIASES.write().insert(ext.to_ascii_lowercase(), handler_name.to_string());
}

pub fn lookup_alias(ext: &str) -> Option<String> {
    ALIASES.read().get(&ext.to_ascii_lowercase()).cloned()
}