use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, routing::{get, post}};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

pub mod migrations;
pub mod handlers;

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
                        "link": "/admin/blocks/",
                        "weight": 40
                    }
                ]
            }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.ok();
        Ok(())
    }
fn rpc_methods(&self) -> Vec<crate::rpc::RpcMethodDescriptor> {
    use crate::rpc::{RpcMethodDescriptor, RpcVisibility};
    vec![
        RpcMethodDescriptor { name: "list_positions".into(), handler: "list_positions".into(), permission: None, visibility: RpcVisibility::Internal },
        RpcMethodDescriptor { name: "save_position".into(), handler: "save_position".into(), permission: None, visibility: RpcVisibility::Internal },
        RpcMethodDescriptor { name: "list_blocks".into(), handler: "list_blocks".into(), permission: None, visibility: RpcVisibility::Internal },
        RpcMethodDescriptor { name: "edit_block".into(), handler: "edit_block".into(), permission: None, visibility: RpcVisibility::Internal },
        RpcMethodDescriptor { name: "save_block".into(), handler: "save_block".into(), permission: None, visibility: RpcVisibility::Internal },
        RpcMethodDescriptor { name: "delete_position".into(), handler: "delete_position".into(), permission: None, visibility: RpcVisibility::Internal },
        RpcMethodDescriptor { name: "delete_block".into(), handler: "delete_block".into(), permission: None, visibility: RpcVisibility::Internal },
    ]
}

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(handlers::list_blocks))
            .route("/edit", get(handlers::edit_block))
            .route("/save", post(handlers::save_block))
            .route("/delete", get(handlers::delete_block))
            .route("/positions", get(handlers::list_positions))
            .route("/positions/save", post(handlers::save_position))
            .route("/positions/delete", get(handlers::delete_position))
    }
}

crate::register_native_module!("blocks", |db| Arc::new(BlocksModule::new(db)));
