use async_trait::async_trait;
use axum::{Router, routing::get};
use danneo_sdk::module::DanneoModule;
use danneo_sdk::register_native_module;
use danneo_sdk::state::AppState;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

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

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        let state = _state.clone();
        // Register in Admin Menu
        state
            .rpc_registry
            .call(
                "admin_menu",
                "register_items",
                serde_json::json!({
                    "module": "seo",
                    "items": [
                        {
                            "code": "manage",
                            "category": "settings",
                            "label": "admin_seo",
                            "link": "/admin/seo",
                            "weight": 50
                        }
                    ]
                }),
                danneo_sdk::rpc::RpcContext::default(),
                state.clone(),
            )
            .await
            .ok();
        Ok(())
    }

    fn register_admin_routes(&self, state: Arc<AppState>) -> Router<Arc<AppState>> {
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

register_native_module!("seo", |db| Arc::new(SeoModule::new(db)));

#[cfg(test)]
mod tests {
    use super::*;
    use danneo_core::state::AppState;
    use danneo_sdk::danneotest;

    #[danneotest]
    async fn test_seo_init(state: Arc<AppState>) {
        assert!(state.is_module_available("seo").await);
    }
}
