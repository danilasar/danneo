use crate::module::DanneoModule;
use crate::rpc::RpcMethodDescriptor;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tera::Tera;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDescriptor {
    pub name: String,
    pub method: String,
    pub path: String,
    pub handler: String,
    pub entity: Option<String>,
    pub template: Option<String>,
    pub middlewares: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub package: PackageInfo,
    pub module: Option<ModuleInfo>,
    pub compatibility: Option<CompatibilityInfo>,
    pub dependencies: Option<HashMap<String, String>>,
    pub optional_dependencies: Option<HashMap<String, String>>,
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
    pub methods: Vec<RpcMethodDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub package_type: String,
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub runtime_type: String,
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
pub struct Capabilities {
    pub database: Option<Vec<String>>,
    pub filesystem: Option<Vec<String>>,
    pub network: Option<Vec<String>>,
    pub mail: Option<Vec<String>>,
    pub templates: Option<Vec<String>>,
    pub users: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockManifest {
    pub block: BlockInfo,
    pub setting: Option<Vec<BlockSettingSchema>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub id: String,
    #[serde(alias = "module", default)]
    pub module_code: String,
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
    pub default: Option<Value>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub required: Option<bool>,
}

#[async_trait]
pub trait IModuleRegistry: Send + Sync {
    async fn is_available(&self, code: &str) -> bool;
    async fn get_native_modules(&self) -> HashMap<String, Arc<dyn DanneoModule>>;
    async fn init(
        &self,
        script_engine: Arc<dyn IScriptEngine>,
        routes: Arc<dyn IRouteRegistry>,
        packages_dir: std::path::PathBuf,
        state: Arc<crate::state::AppState>,
    );
    async fn clear_admin_menus(&self);
    async fn enable(&self, module_code: &str) -> Result<(), String>;
    async fn disable(&self, module_code: &str) -> Result<(), String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub manifest: PackageManifest,
    pub staging_path: std::path::PathBuf,
    pub is_upgrade: bool,
    pub current_version: Option<String>,
    pub issues: Vec<String>,
}

#[async_trait]
pub trait IPackageRegistry: Send + Sync {
    async fn get_packages(&self) -> HashMap<String, PackageManifest>;
    async fn get_blocks(&self) -> HashMap<String, BlockManifest>;
    fn get_packages_dir(&self) -> std::path::PathBuf;
    async fn scan(&self);
    async fn extract_and_verify(
        &self,
        zip_bytes: &[u8],
        installed_versions: &HashMap<String, String>,
    ) -> Result<VerificationResult, String>;
}

#[async_trait]
pub trait IRouteRegistry: Send + Sync {
    async fn register_frontend(&self, module_code: &str, descriptor: RouteDescriptor);
    async fn register_admin(&self, module_code: &str, descriptor: RouteDescriptor);
    async fn clear_routes(&self);
    async fn get_frontend_routes(&self) -> Vec<(String, RouteDescriptor)>;
    async fn get_admin_routes(&self) -> Vec<(String, RouteDescriptor)>;
}

#[async_trait]
pub trait IScriptEngine: Send + Sync {
    async fn load_module_scripts(
        &self,
        module_code: &str,
        scripts_path: &std::path::Path,
    ) -> Result<(), String>;
    async fn load_script_str(&self, module_code: &str, script: &str) -> Result<(), String>;
    async fn call_hook(
        &self,
        module_code: &str,
        hook_name: &str,
        args: Value,
        state: Arc<crate::state::AppState>,
    ) -> Result<Value, String>;
}

#[async_trait]
pub trait IBlockRegistry: Send + Sync {
    async fn render_block(
        &self,
        block_code: &str,
        ctx: Arc<dyn std::any::Any + Send + Sync>,
        settings: Option<Value>,
        tera: &Tera,
    ) -> Option<String>;
    async fn get_all_positions_html(
        &self,
        ctx: Arc<dyn std::any::Any + Send + Sync>,
        tera: &Tera,
    ) -> HashMap<String, String>;
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
pub struct AdminMenuManifest {
    pub categories: Option<Vec<CategoryContribution>>,
    pub items: Option<Vec<ItemContribution>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryContribution {
    pub code: String,
    pub parent: String,
    pub label: String,
    pub icon: Option<String>,
    pub weight: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemContribution {
    pub code: String,
    pub category: String,
    pub label: String,
    pub link: String,
    pub weight: Option<i32>,
    pub acl_key: Option<String>,
}
