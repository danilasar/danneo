use crate::registry::RouteDescriptor;
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
    pub frontend_routes: Option<Vec<RouteDescriptor>>,
    pub admin_routes: Option<Vec<RouteDescriptor>>,
    pub rpc: Option<RpcManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcManifest {
    pub namespace: String,
    pub methods: Vec<crate::rpc::RpcMethodDescriptor>,
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
    pub runtime_type: String, // "lua", "native"
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
    pub entities: Option<String>,
    pub admin_menu: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminMenuManifest {
    pub categories: Option<Vec<CategoryContribution>>,
    pub items: Option<Vec<ItemContribution>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryContribution {
    pub code: String,
    pub parent: String, // Код надкатегории
    pub label: String,
    pub icon: Option<String>,
    pub weight: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemContribution {
    pub code: String,     // Уникальный код пункта (например, news.list)
    pub category: String, // Код категории
    pub label: String,
    pub link: String,
    pub weight: Option<i32>,
    pub acl_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminMenu {
    pub supercategories: Vec<AdminMenuSupercategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminMenuSupercategory {
    pub code: String,
    pub label: String,
    pub weight: i32,
    pub categories: Vec<AdminMenuCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminMenuCategory {
    pub code: String,
    pub label: String,
    pub icon: Option<String>,
    pub weight: i32,
    pub items: Vec<AdminMenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminMenuItem {
    pub label: String,
    pub link: String,
    pub weight: i32,
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
    #[serde(alias = "module", default)]
    pub module_code: String, // Код модуля-поставщика
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySchema {
    pub table_name: String,
    pub name: String,
    pub fields: Vec<EntityField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String, // "integer", "string", "text", "boolean", "datetime"
    pub primary_key: Option<bool>,
    pub auto_increment: Option<bool>,
    pub nullable: Option<bool>,
    pub unique: Option<bool>,
    pub default: Option<serde_json::Value>,
    pub label: Option<String>,
}
