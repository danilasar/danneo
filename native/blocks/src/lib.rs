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
pub mod migrations;

pub struct BlocksModule;

danneo_sdk::inventory::submit! {
    danneo_sdk::module::migration::ModuleMigrationRegistration { migration: &migrations::CreateBlockTables }
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

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        let state = _state.clone();
        // Register in Admin Menu
        state
            .rpc_registry
            .call(
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
                danneo_sdk::rpc::RpcContext::default(),
                state.clone(),
            )
            .await
            .ok();
        Ok(())
    }
    fn rpc_methods(&self) -> Vec<danneo_sdk::rpc::RpcMethodDescriptor> {
        use danneo_sdk::rpc::{RpcMethodDescriptor, RpcVisibility};
        vec![
            RpcMethodDescriptor {
                name: "list_positions".into(),
                handler: "list_positions".into(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "save_position".into(),
                handler: "save_position".into(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "list_blocks".into(),
                handler: "list_blocks".into(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "edit_block".into(),
                handler: "edit_block".into(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "save_block".into(),
                handler: "save_block".into(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "delete_position".into(),
                handler: "delete_position".into(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "delete_block".into(),
                handler: "delete_block".into(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
        ]
    }

    fn register_admin_routes(&self, state: Arc<AppState>) -> Router<Arc<AppState>> {
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

danneo_sdk::register_native_module!("blocks", |db| Arc::new(BlocksModule::new(db)));

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use danneo_sdk::auth::AuthService;
    use danneo_sdk::danneotest;
    use tower::ServiceExt;

    #[danneotest]
    async fn test_blocks_init(state: Arc<AppState>) {
        assert!(state.is_module_available("blocks").await);
    }

    #[danneotest]
    async fn test_list_positions(state: Arc<AppState>) {
        let module = BlocksModule;
        let app = module
            .register_admin_routes(state.clone())
            .with_state(state.clone());

        let auth_service = AuthService::new(state.jwt_secret.clone());
        let token = auth_service
            .create_token(1, 9999999999, 1000000000)
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/positions")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[danneotest]
    async fn test_save_position(state: Arc<AppState>) {
        let module = BlocksModule;
        let app = module
            .register_admin_routes(state.clone())
            .with_state(state.clone());

        let auth_service = AuthService::new(state.jwt_secret.clone());
        let token = auth_service
            .create_token(1, 9999999999, 1000000000)
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/positions/save")
                    .header("Cookie", format!("danneo_token={}", token))
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from("positname=Right&positcode=RIGHT&pposit=2"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/admin/blocks/positions"
        );
    }

    #[danneotest]
    async fn test_list_blocks(state: Arc<AppState>) {
        let module = BlocksModule;
        let app = module
            .register_admin_routes(state.clone())
            .with_state(state.clone());

        let auth_service = AuthService::new(state.jwt_secret.clone());
        let token = auth_service
            .create_token(1, 9999999999, 1000000000)
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
