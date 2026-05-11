use async_trait::async_trait;
use axum::{
    Router,
    routing::{get, post},
};
use danneo_sdk::module::DanneoModule;
use danneo_sdk::register_native_module;
use danneo_sdk::rpc::{RpcContext, RpcError, RpcMethodDescriptor, RpcVisibility};
use danneo_sdk::state::AppState;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use sea_query::{Alias, Query};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::info;

pub mod handlers;

pub struct SettingsModule {
    db: Arc<DatabaseConnection>,
}

impl SettingsModule {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DanneoModule for SettingsModule {
    fn name(&self) -> &'static str {
        "settings"
    }

    async fn on_install(&self, _state: Arc<AppState>) -> Result<(), String> {
        let db = self.db.as_ref();
        let backend = db.get_database_backend();
        use sea_orm_migration::prelude::SchemaManager;
        let manager = SchemaManager::new(db);
        use sea_query::{ColumnDef, Table};

        // 1. Создание таблицы настроек (если нет)
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("core_settings"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("key"))
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("value")).json_binary().not_null())
                    .to_owned(),
            )
            .await
            .map_err(|e| e.to_string())?;

        // 2. Дефолтные настройки
        let defaults = [
            ("site_name", "\"Danneo\""),
            ("admin_email", "\"admin@example.com\""),
            ("site_url", "\"http://localhost:3000\""),
            ("site_temp", "\"Soft\""),
            // Storage defaults
            ("storage_endpoint", "\"http://localhost:9000\""),
            ("storage_access_key", "\"minioadmin\""),
            ("storage_secret_key", "\"minioadmin\""),
            ("storage_bucket", "\"neodanneo\""),
            ("storage_region", "\"us-east-1\""),
        ];

        for (key, val) in defaults {
            let (sql, values) = Query::select()
                .column(Alias::new("key"))
                .from(Alias::new("core_settings"))
                .and_where(sea_query::Expr::col(Alias::new("key")).eq(key))
                .build_any(match backend {
                    sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                    sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                    sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                });

            let exists = db
                .query_one(Statement::from_sql_and_values(backend, &sql, values))
                .await
                .unwrap_or(None)
                .is_some();

            if !exists {
                let (sql, values) = Query::insert()
                    .into_table(Alias::new("core_settings"))
                    .columns([Alias::new("key"), Alias::new("value")])
                    .values_panic([key.into(), val.into()])
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });
                db.execute(Statement::from_sql_and_values(backend, &sql, values))
                    .await
                    .ok();
            }
        }
        Ok(())
    }

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        let state = _state.clone();
        // 1. Load from DB into state memory
        use danneo_sdk::models::core_settings;
        use sea_orm::EntityTrait;
        let db = self.db.as_ref();

        if let Ok(records) = core_settings::Entity::find().all(db).await {
            let mut guard = state.settings.write().await;
            for record in records {
                let val_str = match record.value {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    v => v.to_string().replace('"', ""),
                };
                match record.key.as_str() {
                    "site_name" => guard.site_name = val_str,
                    "admin_email" => guard.admin_email = val_str,
                    "site_url" => guard.site_url = val_str,
                    "site_temp" => guard.site_temp = val_str,
                    "storage_endpoint" => guard.storage_endpoint = val_str,
                    "storage_access_key" => guard.storage_access_key = val_str,
                    "storage_secret_key" => guard.storage_secret_key = val_str,
                    "storage_bucket" => guard.storage_bucket = val_str,
                    "storage_region" => guard.storage_region = val_str,
                    _ => {}
                }
            }
        }

        // 2. Register in Admin Menu via RPC
        state
            .rpc_registry
            .call(
                "admin_menu",
                "register_items",
                serde_json::json!({
                    "module": "settings",
                    "items": [
                        {
                            "code": "site",
                            "category": "settings",
                            "label": "admin_settings",
                            "link": "/admin/settings/",
                            "weight": 10
                        }
                    ]
                }),
                danneo_sdk::rpc::RpcContext::default(),
                state.clone(),
            )
            .await
            .ok();

        info!("Settings Native Module initialized and state synchronized");
        Ok(())
    }

    fn register_admin_routes(&self, state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(handlers::show_settings))
            .route("/save", post(handlers::save_settings))
    }

    fn rpc_methods(&self) -> Vec<RpcMethodDescriptor> {
        vec![
            RpcMethodDescriptor {
                name: "get".to_string(),
                handler: "get".to_string(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "set".to_string(),
                handler: "set".to_string(),
                permission: Some("admin.manage".to_string()),
                visibility: RpcVisibility::Internal,
            },
        ]
    }

    async fn call_rpc(
        &self,
        method: &str,
        payload: Value,
        _ctx: RpcContext,
        _state: Arc<AppState>,
    ) -> Result<Value, RpcError> {
        let state = _state.clone();
        let backend = self.db.get_database_backend();
        match method {
            "get" => {
                let key = payload
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing key".to_string()))?;
                let settings = state.settings.read().await;
                match key {
                    "site_name" => Ok(json!(settings.site_name)),
                    "site_url" => Ok(json!(settings.site_url)),
                    "site_temp" => Ok(json!(settings.site_temp)),
                    "admin_email" => Ok(json!(settings.admin_email)),
                    "storage_endpoint" => Ok(json!(settings.storage_endpoint)),
                    _ => Err(RpcError::NotFound(key.to_string())),
                }
            }
            "set" => {
                let key = payload
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing key".to_string()))?;
                let val = payload
                    .get("value")
                    .ok_or_else(|| RpcError::BadRequest("Missing value".to_string()))?;
                let val_str = serde_json::to_string(val).unwrap();

                let (sql, values) = Query::insert()
                    .into_table(Alias::new("core_settings"))
                    .columns([Alias::new("key"), Alias::new("value")])
                    .values_panic([key.into(), val_str.into()])
                    .on_conflict(
                        sea_query::OnConflict::column(Alias::new("key"))
                            .update_column(Alias::new("value"))
                            .to_owned(),
                    )
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });

                self.db
                    .execute(Statement::from_sql_and_values(backend, &sql, values))
                    .await
                    .map_err(|e| RpcError::Runtime(e.to_string()))?;

                Ok(json!({ "status": "success" }))
            }
            _ => Err(RpcError::NotFound(method.to_string())),
        }
    }
}

register_native_module!("settings", |db| Arc::new(SettingsModule::new(db)));

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
    async fn test_settings_init(state: Arc<AppState>) {
        assert!(state.is_module_available("settings").await);
        let settings = state.settings.read().await;
        // The default site name from on_install
        assert_eq!(settings.site_name, "Danneo");
    }

    #[danneotest]
    async fn test_show_settings_page(state: Arc<AppState>) {
        let module = SettingsModule::new(state.db.clone());
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

    #[danneotest]
    async fn test_save_settings_redirect(state: Arc<AppState>) {
        let module = SettingsModule::new(state.db.clone());
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
                    .uri("/save")
                    .header("Cookie", format!("danneo_token={}", token))
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from("site_name=NewName&admin_email=new@example.com&site_url=http://new.com&site_temp=Old"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/admin/settings"
        );

        let settings = state.settings.read().await;
        assert_eq!(settings.site_name, "NewName");
    }
}
