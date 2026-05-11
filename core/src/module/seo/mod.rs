use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, routing::get};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

pub mod handlers;

pub struct SeoModule;

impl SeoModule {
    pub fn new(_db: Arc<DatabaseConnection>) -> Self {
        Self
    }
}

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
                        "code": "manage",
                        "category": "settings",
                        "label": "admin_seo",
                        "link": "/admin/seo/",
                        "weight": 50
                    }
                ]
            }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.ok();
        Ok(())
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        use axum::routing::post;
        Router::new()
            .route("/", get(handlers::show_settings))
            .route("/save", post(handlers::save_settings))
            .route("/sitemap", get(handlers::show_sitemap))
            .route("/sitemap/save", post(handlers::save_sitemap))
            .route("/social", get(handlers::show_social))
            .route("/social/save", post(handlers::save_social))
            .route("/social/delete", get(handlers::delete_social))
    }
}

crate::register_native_module!("seo", |db| Arc::new(SeoModule::new(db)));
