use crate::module::DanneoModule;
use crate::registry::ScriptEngine;
use crate::rpc::{RpcContext, RpcError, RpcMethodDescriptor};
use crate::state::AppState;
use async_trait::async_trait;
use axum::Router;
use danneo_sdk::registry::IScriptEngine;
use serde_json::Value;
use std::sync::Arc;

pub struct LuaModuleAdapter {
    pub module_code: String,
    pub script_engine: Arc<ScriptEngine>,
}

#[async_trait]
impl DanneoModule for LuaModuleAdapter {
    fn name(&self) -> &'static str {
        // We return a static string, but since it's dynamic, we need to be careful.
        // For simplicity in this adapter, we leak the string or use a mapping.
        // In a real actor model, this won't be needed as much.
        Box::leak(self.module_code.clone().into_boxed_str())
    }

    async fn init(&self, state: Arc<AppState>) -> Result<(), String> {
        let arg = serde_json::Value::Null;
        self.script_engine
            .call_hook(&self.module_code, "init", arg, state.clone())
            .await
            .map(|_| ())
    }

    fn rpc_methods(&self) -> Vec<RpcMethodDescriptor> {
        // Lua RPC methods are discovered via manifest, so this might be empty here
        // and handled by the RpcRegistry directly for Lua.
        vec![]
    }

    async fn call_rpc(
        &self,
        method: &str,
        payload: Value,
        ctx: RpcContext,
        state: Arc<AppState>,
    ) -> Result<Value, RpcError> {
        let arg = serde_json::json!({
            "method": method,
            "payload": payload,
            "context": ctx
        });
        self.script_engine
            .call_hook(&self.module_code, "rpc_dispatch", arg, state.clone())
            .await
            .map_err(|e| RpcError::Runtime(e.to_string()))
    }

    fn register_routes(&self, state: Arc<AppState>) -> Router<Arc<AppState>> {
        create_axum_router_from_lua(
            &self.module_code,
            self.script_engine.clone(),
            state.clone(),
            false,
        )
    }

    fn register_admin_routes(&self, state: Arc<AppState>) -> Router<Arc<AppState>> {
        create_axum_router_from_lua(
            &self.module_code,
            self.script_engine.clone(),
            state.clone(),
            true,
        )
    }
}

pub async fn build_lua_module_adapter(
    module_code: String,
    script_engine: Arc<ScriptEngine>,
    _state: Arc<AppState>,
) -> Arc<LuaModuleAdapter> {
    Arc::new(LuaModuleAdapter {
        module_code,
        script_engine,
    })
}

fn create_axum_router_from_lua(
    module_code: &str,
    _script_engine: Arc<ScriptEngine>,
    _state: Arc<AppState>,
    _is_admin: bool,
) -> Router<Arc<AppState>> {
    use axum::routing::get;
    let _code = module_code.to_string();

    // This is a simplified proxy. In a real system, we'd iterate over manifest routes.
    Router::new().fallback(get(move |_req: axum::extract::Request| {
        async move {
            // Logic to call Lua dispatch and return Response
            "Lua Response Placeholder".to_string()
        }
    }))
}
