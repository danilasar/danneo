use crate::module::DanneoModule;
use crate::state::AppState;
use crate::registry::{AdminMenu, AdminMenuCategory, AdminMenuItem, AdminMenuSupercategory, AdminMenuManifest, ItemContribution, CategoryContribution};
use crate::rpc::{RpcContext, RpcError, RpcVisibility, RpcMethodDescriptor};
use async_trait::async_trait;
use axum::Router;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use sea_query::{Alias, Query, ColumnDef, Table};
use std::sync::Arc;
use tracing::info;

pub struct AdminMenuModule {
    db: Arc<DatabaseConnection>,
}

impl AdminMenuModule {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Сборка финального меню с учетом Core, Module contributions и Admin overrides.
    /// Если передан admin_id, выполняется фильтрация по ACL.
    pub async fn build_menu(&self, admin_id: Option<i32>, acl: Option<&Arc<crate::acl::service::AclService>>) -> AdminMenu {
        let db = self.db.as_ref();
        let backend = db.get_database_backend();

        // 1. Загружаем Надкатегории
        let super_rows = db.query_all(Statement::from_string(
            backend,
            "SELECT code, label_key, weight FROM core_admin_menu_supercategories ORDER BY weight ASC"
        )).await.unwrap_or_default();

        let mut supercategories = Vec::new();
        for row in super_rows {
            let code: String = row.try_get("", "code").unwrap();
            let label_key: String = row.try_get("", "label_key").unwrap();
            let weight: i32 = row.try_get("", "weight").unwrap();

            supercategories.push(AdminMenuSupercategory {
                code: code.clone(),
                label: self.localize(&label_key),
                weight,
                categories: Vec::new(),
            });
        }
        tracing::debug!("Found {} supercategories", supercategories.len());

        // 2. Загружаем Категории
        let cat_rows = db.query_all(Statement::from_string(
            backend,
            "SELECT super_code, code, label_key, icon, weight FROM core_admin_menu_categories ORDER BY weight ASC"
        )).await.unwrap_or_default();

        let mut cat_count = 0;
        for row in cat_rows {
            let super_code: String = row.try_get("", "super_code").unwrap();
            let code: String = row.try_get("", "code").unwrap();
            let label_key: String = row.try_get("", "label_key").unwrap();
            let icon: Option<String> = row.try_get("", "icon").ok();
            let weight: i32 = row.try_get("", "weight").unwrap();

            if let Some(super_cat) = supercategories.iter_mut().find(|s| s.code == super_code) {
                super_cat.categories.push(AdminMenuCategory {
                    code,
                    label: self.localize(&label_key),
                    icon,
                    weight,
                    items: Vec::new(),
                });
                cat_count += 1;
            }
        }
        tracing::debug!("Found {} categories", cat_count);

        // 3. Загружаем Пункты меню (только для включенных модулей)
        use sea_query::{Expr, JoinType};
        let (sql, values) = Query::select()
            .columns([
                (Alias::new("i"), Alias::new("code")),
                (Alias::new("i"), Alias::new("category_code")),
                (Alias::new("i"), Alias::new("label_key")),
                (Alias::new("i"), Alias::new("link")),
                (Alias::new("i"), Alias::new("weight")),
                (Alias::new("i"), Alias::new("acl_key")),
            ])
            .from_as(Alias::new("core_admin_menu_items"), Alias::new("i"))
            .join_as(
                JoinType::InnerJoin,
                Alias::new("core_modules"),
                Alias::new("m"),
                Expr::col((Alias::new("i"), Alias::new("module_code")))
                    .eq(Expr::col((Alias::new("m"), Alias::new("code"))))
            )
            .and_where(Expr::col((Alias::new("i"), Alias::new("is_hidden"))).eq(false))
            .and_where(Expr::col((Alias::new("m"), Alias::new("enabled"))).eq(true))
            .order_by((Alias::new("i"), Alias::new("weight")), sea_query::Order::Asc)
            .build_any(match backend {
                sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
            });

        let item_rows = db.query_all(Statement::from_sql_and_values(backend, &sql, values))
            .await.unwrap_or_default();
        
        tracing::debug!("Found {} menu items from enabled modules", item_rows.len());

        for row in item_rows {
            let cat_code: String = row.try_get("", "category_code").unwrap();
            let label_key: String = row.try_get("", "label_key").unwrap();
            let link: String = row.try_get("", "link").unwrap();
            let weight: i32 = row.try_get("", "weight").unwrap();
            let acl_key: Option<String> = row.try_get("", "acl_key").ok();

            // Проверка ACL
            if let (Some(id), Some(acl_svc), Some(key)) = (admin_id, acl, acl_key) {
                if !key.is_empty() {
                    // Ищем логин админа для Casbin
                    let admin_login = self.get_admin_login(id).await.unwrap_or_default();
                    if !acl_svc.enforce(&admin_login, &key, "view", 0).await {
                        continue;
                    }
                }
            }

            for super_cat in &mut supercategories {
                if let Some(cat) = super_cat.categories.iter_mut().find(|c| c.code == cat_code) {
                    cat.items.push(AdminMenuItem {
                        label: self.localize(&label_key),
                        link: link.clone(),
                        weight,
                    });
                }
            }
        }

        // Удаляем пустые категории и надкатегории
        for super_cat in &mut supercategories {
            super_cat.categories.retain(|c| !c.items.is_empty());
        }
        supercategories.retain(|s| !s.categories.is_empty());

        tracing::debug!("Final menu has {} supercategories after pruning", supercategories.len());

        AdminMenu { supercategories }
    }

    fn localize(&self, key: &str) -> String {
        let localized = rust_i18n::t!(key);
        if localized == key {
            key.to_string()
        } else {
            localized.to_string()
        }
    }

    async fn get_admin_login(&self, id: i32) -> Option<String> {
        use crate::models::core_admins;
        use sea_orm::EntityTrait;
        core_admins::Entity::find_by_id(id).one(self.db.as_ref()).await.ok().flatten().map(|a| a.login)
    }

    /// Обработка вклада модуля в меню
    pub async fn process_contribution(
        &self,
        module_code: &str,
        manifest: AdminMenuManifest,
    ) -> Result<(), String> {
        let db = self.db.as_ref();
        let backend = db.get_database_backend();

        // 1. Обрабатываем предложенные категории
        if let Some(categories) = manifest.categories {
            for cat in categories {
                let (sql, values) = Query::select()
                    .column(Alias::new("id"))
                    .from(Alias::new("core_admin_menu_categories"))
                    .and_where(sea_query::Expr::col(Alias::new("code")).eq(cat.code.clone()))
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });

                let exists = db.query_one(Statement::from_sql_and_values(backend, &sql, values))
                    .await.unwrap_or(None).is_some();

                if !exists {
                    info!("Creating managed category '{}' for module {}", cat.code, module_code);
                    let (sql, values) = Query::insert()
                        .into_table(Alias::new("core_admin_menu_categories"))
                        .columns([
                            Alias::new("super_code"),
                            Alias::new("code"),
                            Alias::new("label_key"),
                            Alias::new("icon"),
                            Alias::new("weight"),
                            Alias::new("is_managed"),
                        ])
                        .values_panic([
                            cat.parent.into(),
                            cat.code.into(),
                            cat.label.into(),
                            cat.icon.into(),
                            cat.weight.unwrap_or(0).into(),
                            true.into(),
                        ])
                        .build_any(match backend {
                            sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                            sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                            sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                        });
                    
                    db.execute(Statement::from_sql_and_values(backend, &sql, values))
                        .await.map_err(|e| e.to_string())?;
                }
            }
        }

        // 2. Обрабатываем пункты меню
        if let Some(items) = manifest.items {
            for item in items {
                let full_code = format!("{}.{}", module_code, item.code);
                
                // Удаляем старую запись с таким же кодом
                let (sql, values) = Query::delete()
                    .from_table(Alias::new("core_admin_menu_items"))
                    .and_where(sea_query::Expr::col(Alias::new("code")).eq(full_code.clone()))
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });
                
                db.execute(Statement::from_sql_and_values(backend, &sql, values)).await.ok();

                let (sql, values) = Query::insert()
                    .into_table(Alias::new("core_admin_menu_items"))
                    .columns([
                        Alias::new("code"),
                        Alias::new("category_code"),
                        Alias::new("module_code"),
                        Alias::new("label_key"),
                        Alias::new("link"),
                        Alias::new("weight"),
                        Alias::new("acl_key"),
                    ])
                    .values_panic([
                        full_code.into(),
                        item.category.into(),
                        module_code.into(),
                        item.label.into(),
                        item.link.into(),
                        item.weight.unwrap_or(0).into(),
                        item.acl_key.into(),
                    ])
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });
                
                db.execute(Statement::from_sql_and_values(backend, &sql, values))
                    .await.map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    /// Удаление пунктов меню при деинсталляции модуля
    pub async fn remove_module_items(&self, module_code: &str) -> Result<(), String> {
        let db = self.db.as_ref();
        let backend = db.get_database_backend();

        let (sql, values) = Query::delete()
            .from_table(Alias::new("core_admin_menu_items"))
            .and_where(sea_query::Expr::col(Alias::new("module_code")).eq(module_code))
            .build_any(match backend {
                sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
            });

        db.execute(Statement::from_sql_and_values(backend, &sql, values))
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[async_trait]
impl DanneoModule for AdminMenuModule {
    fn name(&self) -> &'static str {
        "admin_menu"
    }

    async fn on_install(&self, _state: Arc<AppState>) -> Result<(), String> {
        let db = self.db.as_ref();
        let backend = db.get_database_backend();
        use sea_orm_migration::prelude::SchemaManager;
        let manager = SchemaManager::new(db);

        // 1. Создание таблиц
        manager.create_table(Table::create().table(Alias::new("core_admin_menu_supercategories")).if_not_exists()
            .col(ColumnDef::new(Alias::new("id")).integer().not_null().auto_increment().primary_key())
            .col(ColumnDef::new(Alias::new("code")).string().not_null().unique_key())
            .col(ColumnDef::new(Alias::new("label_key")).string().not_null())
            .col(ColumnDef::new(Alias::new("weight")).integer().not_null().default(0))
            .to_owned()).await.map_err(|e| e.to_string())?;

        manager.create_table(Table::create().table(Alias::new("core_admin_menu_categories")).if_not_exists()
            .col(ColumnDef::new(Alias::new("id")).integer().not_null().auto_increment().primary_key())
            .col(ColumnDef::new(Alias::new("super_code")).string().not_null())
            .col(ColumnDef::new(Alias::new("code")).string().not_null().unique_key())
            .col(ColumnDef::new(Alias::new("label_key")).string().not_null())
            .col(ColumnDef::new(Alias::new("icon")).string())
            .col(ColumnDef::new(Alias::new("weight")).integer().not_null().default(0))
            .col(ColumnDef::new(Alias::new("is_managed")).boolean().not_null().default(false))
            .to_owned()).await.map_err(|e| e.to_string())?;

        manager.create_table(Table::create().table(Alias::new("core_admin_menu_items")).if_not_exists()
            .col(ColumnDef::new(Alias::new("id")).integer().not_null().auto_increment().primary_key())
            .col(ColumnDef::new(Alias::new("code")).string().not_null().unique_key())
            .col(ColumnDef::new(Alias::new("category_code")).string().not_null())
            .col(ColumnDef::new(Alias::new("module_code")).string().not_null())
            .col(ColumnDef::new(Alias::new("label_key")).string().not_null())
            .col(ColumnDef::new(Alias::new("link")).string().not_null())
            .col(ColumnDef::new(Alias::new("weight")).integer().not_null().default(0))
            .col(ColumnDef::new(Alias::new("acl_key")).string())
            .col(ColumnDef::new(Alias::new("is_hidden")).boolean().not_null().default(false))
            .to_owned()).await.map_err(|e| e.to_string())?;


        // 2. Наполнение базовыми данными
        let count_scat = db.query_one(Statement::from_string(backend, "SELECT COUNT(*) as count FROM core_admin_menu_supercategories"))
            .await.unwrap().unwrap().try_get::<i64>("", "count").unwrap_or(0);
        
        if count_scat == 0 {
            let insert_scat = Query::insert().into_table(Alias::new("core_admin_menu_supercategories"))
                .columns([Alias::new("code"), Alias::new("label_key"), Alias::new("weight")])
                .values_panic(["content".into(), "admin_content".into(), 10.into()])
                .values_panic(["system".into(), "admin_system".into(), 20.into()])
                .values_panic(["tools".into(), "admin_tools".into(), 30.into()])
                .to_owned();
            db.execute(backend.build(&insert_scat)).await.map_err(|e| e.to_string())?;

            let insert_cat = Query::insert().into_table(Alias::new("core_admin_menu_categories"))
                .columns([Alias::new("super_code"), Alias::new("code"), Alias::new("label_key"), Alias::new("icon"), Alias::new("weight"), Alias::new("is_managed")])
                .values_panic(["system".into(), "settings".into(), "admin_settings_title".into(), "setting.gif".into(), 10.into(), false.into()])
                .values_panic(["system".into(), "security".into(), "admin_security".into(), "user.gif".into(), 20.into(), false.into()])
                .to_owned();
            db.execute(backend.build(&insert_cat)).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        // Register OWN items via RPC
        self.call_rpc(
            "register_items",
            serde_json::json!({
                "module": "admin_menu",
                "items": [
                    {
                        "code": "modules",
                        "category": "settings",
                        "label": "Модули и пакеты",
                        "link": "/admin/modules",
                        "weight": 30
                    },
                    {
                        "code": "menu",
                        "category": "settings",
                        "label": "admin_menu",
                        "link": "/admin/menu_system",
                        "weight": 55
                    }
                ]
            }),
            crate::rpc::RpcContext::default(),
            _state.clone()
        ).await.ok();

        info!("Admin Menu Native Module initialized");
        Ok(())
    }

    fn rpc_methods(&self) -> Vec<RpcMethodDescriptor> {
        vec![
            RpcMethodDescriptor {
                name: "get_tree".to_string(),
                handler: "get_tree".to_string(),
                permission: Some("admin.view".to_string()),
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "get_effective_tree".to_string(),
                handler: "get_effective_tree".to_string(),
                permission: Some("admin.view".to_string()),
                visibility: RpcVisibility::Internal,
            },
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
                name: "ensure_category".to_string(),
                handler: "ensure_category".to_string(),
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
                name: "set_item_visibility".to_string(),
                handler: "set_item_visibility".to_string(),
                permission: Some("admin.manage".to_string()),
                visibility: RpcVisibility::Internal,
            },
        ]
    }

    async fn call_rpc(
        &self,
        method: &str,
        payload: serde_json::Value,
        _ctx: RpcContext,
        state: Arc<AppState>,
    ) -> Result<serde_json::Value, RpcError> {
        let backend = self.db.get_database_backend();
        match method {
            "get_tree" => {
                let tree = self.build_menu(None, None).await;
                Ok(serde_json::to_value(tree).unwrap())
            }
            "get_effective_tree" => {
                let admin_id = payload.get("admin_id").and_then(|v| v.as_i64()).map(|v| v as i32);
                let tree = self.build_menu(admin_id, Some(&state.acl)).await;
                Ok(serde_json::to_value(tree).unwrap())
            }
            "register_items" => {
                let module_code = payload.get("module").and_then(|v| v.as_str()).ok_or_else(|| RpcError::BadRequest("Missing 'module'".to_string()))?;
                let items_val = payload.get("items").ok_or_else(|| RpcError::BadRequest("Missing 'items'".to_string()))?;
                let items: Vec<ItemContribution> = serde_json::from_value(items_val.clone()).map_err(|e| RpcError::BadRequest(e.to_string()))?;
                
                let manifest = AdminMenuManifest {
                    categories: None,
                    items: Some(items),
                };
                self.process_contribution(module_code, manifest).await.map_err(|e| RpcError::Runtime(e))?;
                Ok(serde_json::json!({ "status": "success" }))
            }
            "unregister_module" => {
                let module_code = payload.get("module").and_then(|v| v.as_str()).ok_or_else(|| RpcError::BadRequest("Missing 'module'".to_string()))?;
                let mode = payload.get("mode").and_then(|v| v.as_str()).unwrap_or("remove");
                
                match mode {
                    "disable" => {
                        let (sql, values) = Query::update()
                            .table(Alias::new("core_admin_menu_items"))
                            .value(Alias::new("is_hidden"), true)
                            .and_where(sea_query::Expr::col(Alias::new("module_code")).eq(module_code))
                            .build_any(match backend {
                                sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                                sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                                sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                            });
                        self.db.execute(Statement::from_sql_and_values(backend, &sql, values)).await.map_err(|e| RpcError::Runtime(e.to_string()))?;
                    },
                    "remove" | _ => {
                        self.remove_module_items(module_code).await.map_err(|e| RpcError::Runtime(e))?;
                    }
                }
                Ok(serde_json::json!({ "status": "success" }))
            }
            "ensure_category" => {
                let cat_val = payload.clone();
                let cat: CategoryContribution = serde_json::from_value(cat_val).map_err(|e| RpcError::BadRequest(e.to_string()))?;
                let manifest = AdminMenuManifest {
                    categories: Some(vec![cat]),
                    items: None,
                };
                self.process_contribution("system", manifest).await.map_err(|e| RpcError::Runtime(e))?;
                Ok(serde_json::json!({ "status": "success" }))
            }
            "move_item" => {
                let item_code = payload.get("item").and_then(|v| v.as_str()).ok_or_else(|| RpcError::BadRequest("Missing 'item'".to_string()))?;
                let category = payload.get("category").and_then(|v| v.as_str()).ok_or_else(|| RpcError::BadRequest("Missing 'category'".to_string()))?;
                let weight = payload.get("weight").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

                let (sql, values) = Query::update()
                    .table(Alias::new("core_admin_menu_items"))
                    .values([
                        (Alias::new("category_code"), category.into()),
                        (Alias::new("weight"), weight.into()),
                    ])
                    .and_where(sea_query::Expr::col(Alias::new("code")).eq(item_code))
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });

                self.db.execute(Statement::from_sql_and_values(backend, &sql, values)).await.map_err(|e| RpcError::Runtime(e.to_string()))?;
                
                Ok(serde_json::json!({ "status": "success" }))
            }
            "set_item_visibility" => {
                let item_code = payload.get("item").and_then(|v| v.as_str()).ok_or_else(|| RpcError::BadRequest("Missing 'item'".to_string()))?;
                let visible = payload.get("visible").and_then(|v| v.as_bool()).ok_or_else(|| RpcError::BadRequest("Missing 'visible'".to_string()))?;

                let (sql, values) = Query::update()
                    .table(Alias::new("core_admin_menu_items"))
                    .value(Alias::new("is_hidden"), !visible)
                    .and_where(sea_query::Expr::col(Alias::new("code")).eq(item_code))
                    .build_any(match backend {
                        sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                        sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                        sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                    });

                self.db.execute(Statement::from_sql_and_values(backend, &sql, values)).await.map_err(|e| RpcError::Runtime(e.to_string()))?;
                
                Ok(serde_json::json!({ "status": "success" }))
            }
            _ => Err(RpcError::NotFound(method.to_string())),
        }
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        use axum::routing::get;
        Router::new()
            .route("/", get(|| async { axum::response::Html("<h1>Admin Menu Management</h1><p>Work in progress.</p>") }))
    }

    fn admin_routes(&self) -> Vec<crate::registry::RouteDescriptor> {
        use crate::registry::RouteDescriptor;
        vec![
            RouteDescriptor {
                name: "admin_menu.manage".to_string(),
                method: "GET".to_string(),
                path: "/menu_system".to_string(), // Was /menu, now unique
                handler: "manage".to_string(),
                entity: None,
                template: None,
            }
        ]
    }
}
