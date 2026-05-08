use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub package: PackageInfo,
    pub module: Option<ModuleInfo>,
    pub compatibility: Option<CompatibilityInfo>,
    pub dependencies: Option<HashMap<String, String>>,
    pub install: Option<InstallOptions>,
    pub entrypoints: Option<Entrypoints>,
    pub capabilities: Option<Capabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub package_type: String, // "module" or "block"
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub runtime_type: String, // "declarative", "scripted", "native"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityInfo {
    pub core: Option<String>,
    pub database: Option<Vec<String>>,
    pub template_engine: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallOptions {
    pub default_enabled: Option<bool>,
    pub keep_data_on_uninstall: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entrypoints {
    pub frontend_routes: Option<String>,
    pub admin_routes: Option<String>,
    pub settings: Option<String>,
    pub permissions: Option<String>,
    pub hooks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub database: Option<Vec<String>>,
    pub filesystem: Option<Vec<String>>,
    pub network: Option<Vec<String>>,
    pub mail: Option<Vec<String>>,
    pub templates: Option<Vec<String>>,
    pub users: Option<Vec<String>>,
}

// Block-specific manifest structs (from block.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockManifest {
    pub block: BlockInfo,
    pub setting: Option<Vec<BlockSettingSchema>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub id: String,
    pub module: Option<String>,
    pub name: String,
    pub version: String,
    pub template: Option<String>,
    pub renderer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSettingSchema {
    pub key: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub label: Option<String>,
    pub default: Option<serde_json::Value>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub required: Option<bool>,
}
