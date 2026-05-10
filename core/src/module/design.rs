use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, routing::{get, post}};
use std::sync::Arc;

pub struct DesignModule;

#[async_trait]
impl DanneoModule for DesignModule {
    fn name(&self) -> &'static str {
        "design"
    }

    async fn init(&self, state: Arc<AppState>) -> Result<(), String> {
        // Register in Admin Menu
        state.rpc_registry.call(
            "admin_menu",
            "register_items",
            serde_json::json!({
                "module": "design",
                "items": [
                    {
                        "code": "manage",
                        "category": "settings",
                        "label": "admin_design",
                        "link": "/admin/design",
                        "weight": 35
                    }
                ]
            }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.ok();
        Ok(())
    }

    async fn on_uninstall(&self, state: Arc<AppState>) -> Result<(), String> {
        state.rpc_registry.call(
            "admin_menu",
            "unregister_module",
            serde_json::json!({ "module": "design" }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.ok();
        Ok(())
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(crate::apanel::design::show_design))
            .route("/save", post(crate::apanel::design::save_file))
    }
}
