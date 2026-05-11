use async_trait::async_trait;
use axum::{
    Router,
    routing::{get, post},
};
use danneo_sdk::module::DanneoModule;
use danneo_sdk::register_native_module;
use danneo_sdk::state::AppState;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub mod handlers;

pub struct DesignModule;

impl DesignModule {
    pub fn new(_db: Arc<DatabaseConnection>) -> Self {
        Self
    }
}

#[async_trait]
impl DanneoModule for DesignModule {
    fn name(&self) -> &'static str {
        "design"
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
                    "module": "design",
                    "items": [
                        {
                            "code": "manage",
                            "category": "design",
                            "label": "admin_design",
                            "link": "/admin/design",
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
        Router::new()
            .route("/", get(handlers::list_themes))
            .route("/edit", get(handlers::edit_theme))
            .route("/save", post(handlers::save_theme))
    }
}

danneo_sdk::register_native_module!("design", |db| Arc::new(DesignModule::new(db)));

#[cfg(test)]
mod tests {
    use super::*;
    use danneo_core::state::AppState;
    use danneo_sdk::danneotest;

    #[danneotest]
    async fn test_design_init(state: Arc<AppState>) {
        assert!(state.is_module_available("design").await);
    }
}
