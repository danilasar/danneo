use async_trait::async_trait;
use axum::{
    Router,
    routing::{get, post},
};
use danneo_sdk::module::DanneoModule;
use danneo_sdk::register_native_module;
use danneo_sdk::rpc::{RpcContext, RpcError, RpcMethodDescriptor, RpcVisibility};
use danneo_sdk::state::AppState;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tracing::info;

pub mod migrations;

pub struct AdminMenuModule {
    db: Arc<DatabaseConnection>,
}

danneo_sdk::inventory::submit! {
    danneo_sdk::module::migration::ModuleMigrationRegistration { migration: &migrations::CreateMenuTables }
}

impl AdminMenuModule {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DanneoModule for AdminMenuModule {
    fn name(&self) -> &'static str {
        "admin_menu"
    }

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        info!("Admin Menu Native Module initialized");
        Ok(())
    }

    fn rpc_methods(&self) -> Vec<RpcMethodDescriptor> {
        vec![
            RpcMethodDescriptor {
                name: "register_items".to_string(),
                handler: "register_items".to_string(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "unregister_module".to_string(),
                handler: "unregister_module".to_string(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "move_item".to_string(),
                handler: "move_item".to_string(),
                permission: Some("admin.manage".to_string()),
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "unregister_item".to_string(),
                handler: "unregister_item".to_string(),
                permission: Some("admin.manage".to_string()),
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "get_menu".to_string(),
                handler: "get_menu".to_string(),
                permission: None,
                visibility: RpcVisibility::Public,
            },
            RpcMethodDescriptor {
                name: "ensure_category".to_string(),
                handler: "ensure_category".to_string(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "ensure_supercategory".to_string(),
                handler: "ensure_category".to_string(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "unregister_category".to_string(),
                handler: "unregister_category".to_string(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "get_tree".to_string(),
                handler: "get_tree".to_string(),
                permission: None,
                visibility: RpcVisibility::Public,
            },
            RpcMethodDescriptor {
                name: "get_effective_tree".to_string(),
                handler: "get_tree".to_string(),
                permission: None,
                visibility: RpcVisibility::Public,
            },
        ]
    }

    async fn call_rpc(
        &self,
        method: &str,
        payload: serde_json::Value,
        _ctx: RpcContext,
        _state: Arc<AppState>,
    ) -> Result<serde_json::Value, RpcError> {
        let state = _state.clone();
        use sea_orm::{ConnectionTrait, Statement};
        use sea_query::{Alias, Query};
        let db = self.db.as_ref();
        let backend = db.get_database_backend();

        match method {
            "get_tree" | "get_effective_tree" => {
                let (sql, values) = Query::select()
                    .columns([
                        Alias::new("code"),
                        Alias::new("parent_code"),
                        Alias::new("label"),
                        Alias::new("icon"),
                        Alias::new("weight"),
                    ])
                    .from(Alias::new("core_menu_groups"))
                    .order_by(Alias::new("weight"), sea_query::Order::Asc)
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });

                let group_rows = db
                    .query_all(Statement::from_sql_and_values(backend, &sql, values))
                    .await
                    .map_err(|e| RpcError::Runtime(e.to_string()))?;
                let menu_items = self
                    .call_rpc("get_menu", serde_json::json!({}), _ctx, _state.clone())
                    .await?;

                // Track enabled modules
                use danneo_sdk::models::core_modules;
                use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
                let enabled_modules: std::collections::HashSet<String> =
                    core_modules::Entity::find()
                        .filter(danneo_sdk::models::core_modules::Column::Enabled.eq(true))
                        .all(state.db.as_ref())
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .map(|m| m.code)
                        .collect();

                let mut supercats = std::collections::HashMap::new();

                for row in &group_rows {
                    let code: String = row.try_get("", "code").unwrap_or_default();
                    let parent: Option<String> = row.try_get("", "parent_code").ok();

                    if parent.is_none()
                        || parent.as_deref() == Some("")
                        || parent.as_deref() == Some("root")
                    {
                        supercats.insert(
                            code.clone(),
                            serde_json::json!({
                                "code": code,
                                "label": row.try_get::<String>("", "label").unwrap_or_default(),
                                "weight": row.try_get::<i64>("", "weight").unwrap_or(0),
                                "categories": []
                            }),
                        );
                    }
                }

                if !supercats.contains_key("content") {
                    supercats.insert("content".to_string(), serde_json::json!({"code": "content", "label": "Контент", "weight": 10, "categories": []}));
                }
                if !supercats.contains_key("settings") {
                    supercats.insert("settings".to_string(), serde_json::json!({"code": "settings", "label": "Настройки", "weight": 20, "categories": []}));
                }

                for row in &group_rows {
                    let parent: String = row.try_get("", "parent_code").unwrap_or_default();
                    if parent != "" && parent != "root" {
                        if let Some(supercat) = supercats.get_mut(&parent) {
                            let cats = supercat
                                .get_mut("categories")
                                .and_then(|v| v.as_array_mut())
                                .unwrap();
                            cats.push(serde_json::json!({
                                "code": row.try_get::<String>("", "code").unwrap_or_default(),
                                "label": row.try_get::<String>("", "label").unwrap_or_default(),
                                "icon": row.try_get::<String>("", "icon").ok(),
                                "weight": row.try_get::<i64>("", "weight").unwrap_or(0),
                                "items": []
                            }));
                        }
                    }
                }

                if let Some(items) = menu_items.as_array() {
                    for item in items {
                        let module_code = item
                            .get("module")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        // PRUNING: Skip items from disabled modules
                        if !enabled_modules.contains(module_code)
                            && module_code != "admin_menu"
                            && module_code != "settings"
                        {
                            continue;
                        }

                        let cat_code = item
                            .get("category")
                            .and_then(|v| v.as_str())
                            .unwrap_or("general");
                        let mut found = false;

                        for supercat in supercats.values_mut() {
                            let cats = supercat
                                .get_mut("categories")
                                .and_then(|v| v.as_array_mut())
                                .unwrap();
                            for cat in cats {
                                if cat.get("code").and_then(|v| v.as_str()) == Some(cat_code) {
                                    let items_list = cat
                                        .get_mut("items")
                                        .and_then(|v| v.as_array_mut())
                                        .unwrap();
                                    items_list.push(serde_json::json!({
                                        "label": item.get("label").cloned().unwrap_or(serde_json::json!("Unknown")),
                                        "link": item.get("link").cloned().unwrap_or(serde_json::json!("#")),
                                        "weight": item.get("weight").cloned().unwrap_or(serde_json::json!(0)),
                                    }));
                                    found = true;
                                    break;
                                }
                            }
                            if found {
                                break;
                            }
                        }
                    }
                }

                // PRUNING: Remove empty categories and empty supercategories
                for supercat in supercats.values_mut() {
                    let sc_code = supercat["code"].as_str().unwrap_or_default().to_string();
                    let cats = supercat
                        .get_mut("categories")
                        .and_then(|v| v.as_array_mut())
                        .unwrap();
                    let before = cats.len();
                    cats.retain(|cat| {
                        let items = cat.get("items").and_then(|v| v.as_array()).unwrap();
                        !items.is_empty()
                    });
                    if before != cats.len() {
                        eprintln!(
                            "Pruned {} empty categories from {}",
                            before - cats.len(),
                            sc_code
                        );
                    }
                }
                supercats.retain(|_, v| {
                    let cats = v.get("categories").and_then(|c| c.as_array()).unwrap();
                    !cats.is_empty() || v["code"] == "settings" || v["code"] == "content"
                });

                let mut result: Vec<_> = supercats.into_values().collect();
                result.sort_by_key(|v| v.get("weight").and_then(|w| w.as_i64()).unwrap_or(0));

                Ok(serde_json::json!({ "supercategories": result }))
            }
            "ensure_category" | "ensure_supercategory" => {
                let code = payload
                    .get("code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing code".to_string()))?;
                let parent = payload
                    .get("parent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("root");
                let label = payload
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or(code);
                let icon = payload.get("icon").and_then(|v| v.as_str());
                let weight = payload.get("weight").and_then(|v| v.as_i64()).unwrap_or(0);

                let (sql, values) = Query::insert()
                    .into_table(Alias::new("core_menu_groups"))
                    .columns([
                        Alias::new("code"),
                        Alias::new("parent_code"),
                        Alias::new("label"),
                        Alias::new("icon"),
                        Alias::new("weight"),
                    ])
                    .values_panic([
                        code.into(),
                        parent.into(),
                        label.into(),
                        icon.into(),
                        weight.into(),
                    ])
                    .on_conflict(
                        sea_query::OnConflict::column(Alias::new("code"))
                            .update_columns([
                                Alias::new("label"),
                                Alias::new("icon"),
                                Alias::new("weight"),
                            ])
                            .to_owned(),
                    )
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });

                db.execute(Statement::from_sql_and_values(backend, &sql, values))
                    .await
                    .map_err(|e| RpcError::Runtime(e.to_string()))?;
                Ok(serde_json::json!({ "status": "success" }))
            }
            "unregister_category" => {
                let code = payload
                    .get("code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing code".to_string()))?;
                let (sql, values) = Query::delete()
                    .from_table(Alias::new("core_menu_groups"))
                    .and_where(sea_query::Expr::col(Alias::new("code")).eq(code))
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });
                db.execute(Statement::from_sql_and_values(backend, &sql, values))
                    .await
                    .map_err(|e| RpcError::Runtime(e.to_string()))?;
                Ok(serde_json::json!({ "status": "success" }))
            }
            "register_items" => {
                let module = payload
                    .get("module")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing module".to_string()))?;
                let items = payload
                    .get("items")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| RpcError::BadRequest("Missing items".to_string()))?;

                for item in items {
                    let code = item
                        .get("code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let category = item
                        .get("category")
                        .and_then(|v| v.as_str())
                        .unwrap_or("general");
                    let label = item.get("label").and_then(|v| v.as_str()).unwrap_or(code);
                    let link = item.get("link").and_then(|v| v.as_str()).unwrap_or("#");
                    let weight = item.get("weight").and_then(|v| v.as_i64()).unwrap_or(0);

                    let (sql, values) = Query::insert()
                        .into_table(Alias::new("core_menu_items"))
                        .columns([
                            Alias::new("module_code"),
                            Alias::new("item_code"),
                            Alias::new("category"),
                            Alias::new("label"),
                            Alias::new("link"),
                            Alias::new("weight"),
                        ])
                        .values_panic([
                            module.into(),
                            code.into(),
                            category.into(),
                            label.into(),
                            link.into(),
                            weight.into(),
                        ])
                        .on_conflict(
                            sea_query::OnConflict::columns([
                                Alias::new("module_code"),
                                Alias::new("item_code"),
                            ])
                            .update_columns([
                                Alias::new("label"),
                                Alias::new("link"),
                                Alias::new("weight"),
                            ])
                            .to_owned(),
                        )
                        .build_any(match backend {
                            sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                            sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                            sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                        });

                    db.execute(Statement::from_sql_and_values(backend, &sql, values))
                        .await
                        .map_err(|e| RpcError::Runtime(e.to_string()))?;
                }
                Ok(serde_json::json!({ "status": "success" }))
            }
            "move_item" => {
                let item_id = payload
                    .get("item")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing item".to_string()))?;
                let category = payload
                    .get("category")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing category".to_string()))?;
                let weight = payload.get("weight").and_then(|v| v.as_i64());

                let parts: Vec<&str> = item_id.split('.').collect();
                if parts.len() != 2 {
                    return Err(RpcError::BadRequest("Invalid item ID format".into()));
                }
                let module = parts[0];
                let code = parts[1];

                let mut query = Query::update();
                query
                    .table(Alias::new("core_menu_items"))
                    .values([(Alias::new("category"), category.into())]);

                if let Some(w) = weight {
                    query.values([(Alias::new("weight"), w.into())]);
                }

                let (sql, values) = query
                    .and_where(sea_query::Expr::col(Alias::new("module_code")).eq(module))
                    .and_where(sea_query::Expr::col(Alias::new("item_code")).eq(code))
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });

                db.execute(Statement::from_sql_and_values(backend, &sql, values))
                    .await
                    .map_err(|e| RpcError::Runtime(e.to_string()))?;
                Ok(serde_json::json!({ "status": "success" }))
            }
            "unregister_item" => {
                let item_id = payload
                    .get("item")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing item".to_string()))?;
                let parts: Vec<&str> = item_id.split('.').collect();
                if parts.len() != 2 {
                    return Err(RpcError::BadRequest("Invalid item ID format".into()));
                }
                let module = parts[0];
                let code = parts[1];

                let (sql, values) = Query::delete()
                    .from_table(Alias::new("core_menu_items"))
                    .and_where(sea_query::Expr::col(Alias::new("module_code")).eq(module))
                    .and_where(sea_query::Expr::col(Alias::new("item_code")).eq(code))
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });

                db.execute(Statement::from_sql_and_values(backend, &sql, values))
                    .await
                    .map_err(|e| RpcError::Runtime(e.to_string()))?;
                Ok(serde_json::json!({ "status": "success" }))
            }
            "unregister_module" => {
                let module = payload
                    .get("module")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing module".to_string()))?;
                let (sql, values) = Query::delete()
                    .from_table(Alias::new("core_menu_items"))
                    .and_where(sea_query::Expr::col(Alias::new("module_code")).eq(module))
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });
                db.execute(Statement::from_sql_and_values(backend, &sql, values))
                    .await
                    .map_err(|e| RpcError::Runtime(e.to_string()))?;
                Ok(serde_json::json!({ "status": "success" }))
            }
            "get_menu" => {
                let (sql, values) = Query::select()
                    .columns([
                        Alias::new("module_code"),
                        Alias::new("item_code"),
                        Alias::new("category"),
                        Alias::new("label"),
                        Alias::new("link"),
                        Alias::new("weight"),
                    ])
                    .from(Alias::new("core_menu_items"))
                    .order_by(Alias::new("weight"), sea_query::Order::Asc)
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });

                let rows = db
                    .query_all(Statement::from_sql_and_values(backend, &sql, values))
                    .await
                    .map_err(|e| RpcError::Runtime(e.to_string()))?;
                let mut menu = Vec::new();
                for row in rows {
                    menu.push(serde_json::json!({
                        "module": row.try_get::<String>("", "module_code").unwrap_or_default(),
                        "code": row.try_get::<String>("", "item_code").unwrap_or_default(),
                        "category": row.try_get::<String>("", "category").unwrap_or_default(),
                        "label": row.try_get::<String>("", "label").unwrap_or_default(),
                        "link": row.try_get::<String>("", "link").unwrap_or_default(),
                        "weight": row.try_get::<i64>("", "weight").unwrap_or_default(),
                    }));
                }
                Ok(serde_json::json!(menu))
            }
            _ => Err(RpcError::NotFound(method.to_string())),
        }
    }

    fn register_admin_routes(&self, state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
        /*
        .route("/modules", get(crate::apanel::modules::list_modules))
        .route("/modules/install", post(crate::apanel::modules::install_module))
        .route("/modules/uninstall", post(crate::apanel::modules::uninstall_module))
        .route("/modules/enable", post(crate::apanel::modules::enable_module))
        .route("/modules/disable", post(crate::apanel::modules::disable_module))
        .route("/", get(crate::apanel::dashboard::render_dashboard))
        */
    }
}

danneo_sdk::register_native_module!("admin_menu", |db| Arc::new(AdminMenuModule::new(db)));

#[cfg(test)]
mod tests {
    use super::*;
    use danneo_core::state::AppState;
    use danneo_sdk::danneotest;

    #[danneotest]
    async fn test_admin_menu_init(state: Arc<AppState>) {
        assert!(state.is_module_available("admin_menu").await);
    }
}
