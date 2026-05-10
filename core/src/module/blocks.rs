use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, routing::{get, post}};
use std::sync::Arc;

pub struct BlocksModule;

crate::inventory::submit! {
    crate::module::NativeModuleRegistration {
        name: "blocks",
        factory: |_| Arc::new(BlocksModule),
    }
}

#[async_trait]
impl DanneoModule for BlocksModule {
    fn name(&self) -> &'static str {
        "blocks"
    }

    async fn init(&self, state: Arc<AppState>) -> Result<(), String> {
        // Register in Admin Menu
        state.rpc_registry.call(
            "admin_menu",
            "register_items",
            serde_json::json!({
                "module": "blocks",
                "items": [
                    {
                        "code": "manage",
                        "category": "settings",
                        "label": "admin_blocks",
                        "link": "/admin/blocks",
                        "weight": 40
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
            serde_json::json!({ "module": "blocks" }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.ok();
        Ok(())
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
            .route("/positions", get(crate::apanel::blocks::list_positions))
            .route("/positions/save", post(crate::apanel::blocks::save_position))
            .route("/positions/delete", post(crate::apanel::blocks::delete_position))
            .route("/", get(crate::apanel::blocks::list_blocks))
            .route("/edit", get(crate::apanel::blocks::edit_block))
            .route("/save", post(crate::apanel::blocks::save_block))
            .route("/delete", post(crate::apanel::blocks::delete_block))
    }
}
