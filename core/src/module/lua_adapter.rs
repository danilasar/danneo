use crate::module::DanneoModule;
use crate::state::AppState;
use axum::{Router, routing::{get, post, put, delete}, extract::{State, Path}, response::IntoResponse};
use std::sync::Arc;
use async_trait::async_trait;
use std::collections::HashMap;

pub async fn lua_module_dispatch_handler(
    state: State<Arc<AppState>>,
    method: axum::http::Method,
    uri: axum::http::Uri,
    params: Path<HashMap<String, String>>,
    module_code: String,
    handler_name: String,
) -> impl IntoResponse {
    let arg = serde_json::json!({
        "method": method.as_str(),
        "uri": uri.to_string(),
        "params": params.0,
        "handler": handler_name,
    });

    let dynamic_arg = script_rhai::serde::to_dynamic(arg).unwrap();
    match state.script_engine.call_hook(&module_code, "frontend_dispatch", dynamic_arg, state.0.clone()).await {
        Ok(res) => crate::frontend::handle_script_response(res).into_response(),
        Err(e) => IntoResponse::into_response((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }
}

pub struct LuaModuleAdapter {
    pub module_code: String,
    pub static_name: &'static str,
    pub script_engine: Arc<crate::registry::ScriptEngine>,
    pub site_router: Router<Arc<AppState>>,
    pub admin_router: Router<Arc<AppState>>,
}

#[async_trait]
impl DanneoModule for LuaModuleAdapter {
    fn name(&self) -> &'static str {
        self.static_name
    }

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        Ok(())
    }

    fn register_routes(&self) -> Router<Arc<AppState>> {
        self.site_router.clone()
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        self.admin_router.clone()
    }
}

pub async fn build_lua_module_adapter(
    module_code: String,
    script_engine: Arc<crate::registry::ScriptEngine>,
    state: Arc<AppState>,
) -> Arc<LuaModuleAdapter> {
    let site_lua_router = script_engine.call_router_hook(&module_code, "register_routes", state.clone()).await.unwrap_or_default();
    let admin_lua_router = script_engine.call_router_hook(&module_code, "register_admin_routes", state.clone()).await.unwrap_or_default();

    let site_router = create_axum_router_from_lua(module_code.clone(), site_lua_router);
    let admin_router = create_axum_router_from_lua(module_code.clone(), admin_lua_router);

    let static_name: &'static str = Box::leak(module_code.clone().into_boxed_str());

    Arc::new(LuaModuleAdapter {
        module_code,
        static_name,
        script_engine,
        site_router,
        admin_router,
    })
}

pub fn create_axum_router_from_lua(
    module_code: String,
    lua_router: crate::registry::LuaRouter,
) -> Router<Arc<AppState>> {
    let mut router = Router::new();
    
    for route in lua_router.routes {
        let mc = module_code.clone();
        let hn = route.handler.clone();
        
        let handler = move |state: State<Arc<AppState>>, method: axum::http::Method, uri: axum::http::Uri, params: Path<HashMap<String, String>>| {
            let mc = mc.clone();
            let hn = hn.clone();
            async move {
                crate::module::lua_adapter::lua_module_dispatch_handler(state, method, uri, params, mc, hn).await
            }
        };

        match route.method.as_str() {
            "GET" => router = router.route(&route.path, get(handler)),
            "POST" => router = router.route(&route.path, post(handler)),
            "PUT" => router = router.route(&route.path, put(handler)),
            "DELETE" => router = router.route(&route.path, delete(handler)),
            _ => {}
        }
    }
    
    router
}
