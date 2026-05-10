use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, routing::{get, post}};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

pub mod migrations;

pub struct BlocksModule;

crate::inventory::submit! {
    migration::ModuleMigrationRegistration { migration: &migrations::CreateBlockTables }
}

impl BlocksModule {
    pub fn new(_db: Arc<DatabaseConnection>) -> Self {
        Self
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

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(crate::apanel::blocks::list_blocks))
            .route("/positions", get(crate::apanel::blocks::list_positions))
            .route("/save_position", post(crate::apanel::blocks::save_position))
    }
}

crate::register_native_module!("blocks", |db| Arc::new(BlocksModule::new(db)));
