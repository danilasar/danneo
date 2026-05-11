use crate::state::AppState;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait DanneoModule: Send + Sync {
    fn name(&self) -> &'static str;
    async fn on_install(&self, _state: Arc<AppState>) -> Result<(), String> { Ok(()) }
    async fn on_uninstall(&self, _state: Arc<AppState>) -> Result<(), String> { Ok(()) }
    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> { Ok(()) }

    fn rpc_methods(&self) -> Vec<crate::rpc::RpcMethodDescriptor> { vec![] }
    async fn call_rpc(
        &self,
        _method: &str,
        _payload: Value,
        _ctx: crate::rpc::RpcContext,
        _state: Arc<AppState>,
    ) -> Result<Value, crate::rpc::RpcError> {
        Err(crate::rpc::RpcError::NotFound(_method.to_string()))
    }

    fn register_routes(&self) -> axum::Router<Arc<AppState>> { axum::Router::new() }
    fn register_admin_routes(&self) -> axum::Router<Arc<AppState>> { axum::Router::new() }
    fn register_admin_middleware(&self, router: axum::Router<Arc<AppState>>, _state: Arc<AppState>) -> axum::Router<Arc<AppState>> { router }
    
    fn block_definitions(&self) -> Vec<NativeBlockDefinition> { vec![] }
    async fn render_block(
        &self,
        _block_code: &str,
        _ctx: Arc<crate::blocks::BlockContext>,
        _settings: Arc<tokio::sync::RwLock<crate::state::GlobalSettings>>,
    ) -> Option<String> { None }
}

#[derive(Clone, Debug)]
pub struct ModuleInfo {
    pub code: String,
}

#[derive(Clone, Debug)]
pub struct NativeBlockDefinition {
    pub block_code: &'static str,
    pub version: &'static str,
    pub settings_schema: Option<Value>,
}

pub type NativeModuleFactory = fn(Arc<DatabaseConnection>) -> Arc<dyn DanneoModule>;

pub struct NativeModuleRegistration {
    pub name: &'static str,
    pub factory: NativeModuleFactory,
}

inventory::collect!(NativeModuleRegistration);

#[macro_export]
macro_rules! register_native_module {
    ($name:expr, $factory:expr) => {
        crate::inventory::submit! {
            crate::module::NativeModuleRegistration {
                name: $name,
                factory: $factory,
            }
        }
    };
}

pub mod admin_menu;
pub mod blocks;
pub mod casbin;
pub mod design;
pub mod image;
pub mod lua_adapter;
pub mod native_demo;
pub mod security;
pub mod seo;
pub mod settings;
pub mod storage;

pub fn init_native_modules() {
    let _ = inventory::iter::<NativeModuleRegistration>().count();
}
