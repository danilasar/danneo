use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, routing::{get, post}};
use std::sync::Arc;

pub struct SeoModule;

#[async_trait]
impl DanneoModule for SeoModule {
    fn name(&self) -> &'static str {
        "seo"
    }

    async fn init(&self, state: Arc<AppState>) -> Result<(), String> {
        // Register in Admin Menu
        state.rpc_registry.call(
            "admin_menu",
            "register_items",
            serde_json::json!({
                "module": "seo",
                "items": [
                    {
                        "code": "settings",
                        "category": "settings",
                        "label": "admin_seo",
                        "link": "/admin/seo",
                        "weight": 20
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
            serde_json::json!({ "module": "seo" }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.ok();
        Ok(())
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(crate::apanel::seo::show_settings))
            .route("/save", post(crate::apanel::seo::save_settings))
            .route("/sitemap", get(crate::apanel::seo::show_sitemap))
            .route("/sitemap/save", post(crate::apanel::seo::save_sitemap))
            .route("/social", get(crate::apanel::seo::show_social))
            .route("/social/save", post(crate::apanel::seo::save_social))
            .route("/social/delete", post(crate::apanel::seo::delete_social))
    }
}
