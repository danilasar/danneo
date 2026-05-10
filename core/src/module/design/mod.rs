use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, routing::get};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

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

    async fn init(&self, state: Arc<AppState>) -> Result<(), String> {
        // Register in Admin Menu
        state.rpc_registry.call(
            "admin_menu",
            "register_items",
            serde_json::json!({
                "module": "design",
                "items": [
                    {
                        "code": "editor",
                        "category": "design",
                        "label": "admin_design",
                        "link": "/admin/design/",
                        "weight": 30
                    }
                ]
            }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.ok();
        Ok(())
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new().route("/", get(crate::apanel::design::show_design))
    }
}

crate::register_native_module!("design", |db| Arc::new(DesignModule::new(db)));
