use crate::rpc::{RpcContext, RpcError, RpcMethodDescriptor};
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use serde_json::Value;
use std::sync::Arc;

pub mod migration;

#[async_trait]
pub trait DanneoModule: Send + Sync {
    fn name(&self) -> &'static str;

    async fn on_install(&self, _state: Arc<crate::state::AppState>) -> Result<(), String> {
        Ok(())
    }
    async fn on_uninstall(&self, _state: Arc<crate::state::AppState>) -> Result<(), String> {
        Ok(())
    }
    async fn init(&self, _state: Arc<crate::state::AppState>) -> Result<(), String> {
        Ok(())
    }

    fn rpc_methods(&self) -> Vec<RpcMethodDescriptor> {
        vec![]
    }

    async fn call_rpc(
        &self,
        _method: &str,
        _payload: Value,
        _ctx: RpcContext,
        _state: Arc<crate::state::AppState>,
    ) -> Result<Value, RpcError> {
        Err(RpcError::NotFound(_method.to_string()))
    }

    fn register_routes(
        &self,
        _state: Arc<crate::state::AppState>,
    ) -> axum::Router<Arc<crate::state::AppState>> {
        axum::Router::new()
    }
    fn register_admin_routes(
        &self,
        _state: Arc<crate::state::AppState>,
    ) -> axum::Router<Arc<crate::state::AppState>> {
        axum::Router::new()
    }
    fn register_admin_middleware(
        &self,
        router: axum::Router<Arc<crate::state::AppState>>,
        _state: Arc<crate::state::AppState>,
    ) -> axum::Router<Arc<crate::state::AppState>> {
        router
    }

    fn frontend_routes(&self) -> Vec<crate::registry::RouteDescriptor> {
        vec![]
    }
    fn admin_routes(&self) -> Vec<crate::registry::RouteDescriptor> {
        vec![]
    }
    fn admin_menu(&self) -> Option<Value> {
        None
    }

    fn block_definitions(&self) -> Vec<NativeBlockDefinition> {
        vec![]
    }
    async fn render_block(
        &self,
        _block_code: &str,
        _ctx: Arc<dyn std::any::Any + Send + Sync>,
        _settings: Arc<dyn std::any::Any + Send + Sync>,
    ) -> Option<String> {
        None
    }
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
        $crate::inventory::submit! {
            $crate::module::NativeModuleRegistration {
                name: $name,
                factory: $factory,
            }
        }
    };
}
