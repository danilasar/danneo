use crate::module::DanneoModule;
use crate::registry::{
    AdminMenu, AdminMenuCategory, AdminMenuItem, AdminMenuManifest, AdminMenuSupercategory,
    ItemContribution, CategoryContribution
};
use crate::rpc::{RpcContext, RpcError, RpcVisibility, RpcMethodDescriptor};
use crate::state::AppState;
use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
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

        // 2. Загружаем Категории
        let cat_rows = db.query_all(Statement::from_string(
            backend,
            "SELECT super_code, code, label_key, icon, weight FROM core_admin_menu_categories ORDER BY weight ASC"
        )).await.unwrap_or_default();

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
            }
        }

        // 3. Загружаем Пункты меню
        let item_rows = db.query_all(Statement::from_string(
            backend,
            "SELECT code, category_code, label_key, link, weight, acl_key FROM core_admin_menu_items WHERE is_hidden = FALSE ORDER BY weight ASC"
        )).await.unwrap_or_default();

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

        // Удаляем пустые категории и надкатегории если это "effective" режим
        if admin_id.is_some() {
            for super_cat in &mut supercategories {
                super_cat.categories.retain(|c| !c.items.is_empty());
            }
            supercategories.retain(|s| !s.categories.is_empty());
        }

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

        // 1. Обрабатываем предложенные категории
        if let Some(categories) = manifest.categories {
            for cat in categories {
                let exists = db
                    .query_one(Statement::from_sql_and_values(
                        db.get_database_backend(),
                        "SELECT id FROM core_admin_menu_categories WHERE code = ?",
                        vec![cat.code.clone().into()],
                    ))
                    .await
                    .unwrap_or(None)
                    .is_some();

                if !exists {
                    info!(
                        "Creating managed category '{}' for module {}",
                        cat.code, module_code
                    );
                    let sql = "INSERT INTO core_admin_menu_categories (super_code, code, label_key, icon, weight, is_managed) VALUES (?, ?, ?, ?, ?, ?)";
                    db.execute(Statement::from_sql_and_values(
                        db.get_database_backend(),
                        sql,
                        vec![
                            cat.parent.into(),
                            cat.code.into(),
                            cat.label.into(),
                            cat.icon.into(),
                            cat.weight.unwrap_or(0).into(),
                            true.into(),
                        ],
                    ))
                    .await
                    .map_err(|e| e.to_string())?;
                }
            }
        }

        // 2. Обрабатываем пункты меню
        if let Some(items) = manifest.items {
            for item in items {
                let full_code = format!("{}.{}", module_code, item.code);

                // Удаляем старую запись с таким же кодом (если есть)
                db.execute(Statement::from_sql_and_values(
                    db.get_database_backend(),
                    "DELETE FROM core_admin_menu_items WHERE code = ?",
                    vec![full_code.clone().into()],
                ))
                .await
                .ok();

                let sql = "INSERT INTO core_admin_menu_items (code, category_code, module_code, label_key, link, weight, acl_key) VALUES (?, ?, ?, ?, ?, ?, ?)";
                db.execute(Statement::from_sql_and_values(
                    db.get_database_backend(),
                    sql,
                    vec![
                        full_code.into(),
                        item.category.into(),
                        module_code.into(),
                        item.label.into(),
                        item.link.into(),
                        item.weight.unwrap_or(0).into(),
                        item.acl_key.into(),
                    ],
                ))
                .await
                .map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    /// Удаление пунктов меню при деинсталляции модуля
    pub async fn remove_module_items(&self, module_code: &str) -> Result<(), String> {
        self.db
            .execute(Statement::from_sql_and_values(
                self.db.get_database_backend(),
                "DELETE FROM core_admin_menu_items WHERE module_code = ?",
                vec![module_code.into()],
            ))
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

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        let db = self.db.as_ref();
        let backend = db.get_database_backend();

        // 1. Создание таблиц
        db.execute_unprepared("
            CREATE TABLE IF NOT EXISTS core_admin_menu_supercategories (
                id SERIAL PRIMARY KEY,
                code VARCHAR(255) NOT NULL UNIQUE,
                label_key VARCHAR(255) NOT NULL,
                weight INTEGER NOT NULL DEFAULT 0
            );").await.map_err(|e| e.to_string())?;

        db.execute_unprepared("
            CREATE TABLE IF NOT EXISTS core_admin_menu_categories (
                id SERIAL PRIMARY KEY,
                super_code VARCHAR(255) NOT NULL,
                code VARCHAR(255) NOT NULL UNIQUE,
                label_key VARCHAR(255) NOT NULL,
                icon VARCHAR(255),
                weight INTEGER NOT NULL DEFAULT 0,
                is_managed BOOLEAN NOT NULL DEFAULT FALSE
            );").await.map_err(|e| e.to_string())?;

        db.execute_unprepared("
            CREATE TABLE IF NOT EXISTS core_admin_menu_items (
                id SERIAL PRIMARY KEY,
                code VARCHAR(255) NOT NULL UNIQUE,
                category_code VARCHAR(255) NOT NULL,
                module_code VARCHAR(255) NOT NULL,
                label_key VARCHAR(255) NOT NULL,
                link VARCHAR(255) NOT NULL,
                weight INTEGER NOT NULL DEFAULT 0,
                acl_key VARCHAR(255),
                is_hidden BOOLEAN NOT NULL DEFAULT FALSE
            );
        ").await.map_err(|e| e.to_string())?;


        // 2. Наполнение базовыми данными (только если пусто)
        let count_scat = db
            .query_one(Statement::from_string(
                backend,
                "SELECT COUNT(*) as count FROM core_admin_menu_supercategories",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i64>("", "count")
            .unwrap_or(0);

        if count_scat == 0 {
            db.execute_unprepared("
                INSERT INTO core_admin_menu_supercategories (code, label_key, weight) VALUES 
                ('content', 'admin_content', 10),
                ('system', 'admin_system', 20),
                ('tools', 'admin_tools', 30);
                
                INSERT INTO core_admin_menu_categories (super_code, code, label_key, icon, weight, is_managed) VALUES 
                ('content', 'infopages', 'admin_infopages', 'infopages.gif', 10, FALSE),
                ('content', 'news', 'admin_news', 'news.gif', 20, FALSE),
                ('system', 'settings', 'admin_settings_title', 'setting.gif', 10, FALSE),
                ('system', 'security', 'admin_security', 'user.gif', 20, FALSE);

                INSERT INTO core_admin_menu_items (code, category_code, module_code, label_key, link, weight, is_hidden) VALUES 
                ('core.modules', 'settings', 'core', 'Модули и пакеты', '/admin/modules', 30, FALSE),
                ('core.blocks', 'settings', 'core', 'admin_blocks', '/admin/blocks', 40, FALSE),
                ('core.menu', 'settings', 'core', 'admin_menu', '/admin/menu', 50, FALSE),
                ('core.amanage', 'security', 'core', 'admin_amanage', '/admin/amanage', 10, FALSE),
                ('core.agroups', 'security', 'core', 'admin_agroups', '/admin/agroups', 20, FALSE);
            ").await.map_err(|e| e.to_string())?;
        }

        info!("Admin Menu Native Module initialized and schema verified");
        Ok(())
    }

    async fn on_install(&self, _state: Arc<AppState>) -> Result<(), String> {
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
                name: "register_items".to_string(),
                handler: "register_items".to_string(),
                permission: None, // Обычно вызывается ядром или при установке
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
        ctx: RpcContext,
        state: Arc<AppState>,
    ) -> Result<serde_json::Value, RpcError> {
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
                let module_code = payload
                    .get("module")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing 'module'".to_string()))?;
                let items_val = payload
                    .get("items")
                    .ok_or_else(|| RpcError::BadRequest("Missing 'items'".to_string()))?;
                let items: Vec<ItemContribution> = serde_json::from_value(items_val.clone())
                    .map_err(|e| RpcError::BadRequest(e.to_string()))?;

                // We need to map ItemContribution to DB fields.
                // Actually process_contribution handles a whole manifest. Let's adapt it.
                let manifest = AdminMenuManifest {
                    categories: None,
                    items: Some(items),
                };
                self.process_contribution(module_code, manifest)
                    .await
                    .map_err(|e| RpcError::Runtime(e))?;
                Ok(serde_json::json!({ "status": "success" }))
            }
            "unregister_module" => {
                let module_code = payload.get("module").and_then(|v| v.as_str()).ok_or_else(|| RpcError::BadRequest("Missing 'module'".to_string()))?;
                let mode = payload.get("mode").and_then(|v| v.as_str()).unwrap_or("remove");
                
                match mode {
                    "disable" => {
                        self.db.execute(Statement::from_sql_and_values(
                            self.db.get_database_backend(),
                            "UPDATE core_admin_menu_items SET is_hidden = TRUE WHERE module_code = ?",
                            vec![module_code.into()]
                        )).await.map_err(|e| RpcError::Runtime(e.to_string()))?;
                    },
                    "remove" | _ => {
                        self.remove_module_items(module_code).await.map_err(|e| RpcError::Runtime(e))?;
                    }
                }
                Ok(serde_json::json!({ "status": "success" }))
            }
            "ensure_category" => {
                let cat_val = payload.clone();
                let cat: CategoryContribution = serde_json::from_value(cat_val)
                    .map_err(|e| RpcError::BadRequest(e.to_string()))?;
                let manifest = AdminMenuManifest {
                    categories: Some(vec![cat]),
                    items: None,
                };
                self.process_contribution("system", manifest)
                    .await
                    .map_err(|e| RpcError::Runtime(e))?;
                Ok(serde_json::json!({ "status": "success" }))
            }
            "move_item" => {
                let item_code = payload
                    .get("item")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing 'item'".to_string()))?;
                let category = payload
                    .get("category")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing 'category'".to_string()))?;
                let weight = payload.get("weight").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

                self.db.execute(Statement::from_sql_and_values(
                    self.db.get_database_backend(),
                    "UPDATE core_admin_menu_items SET category_code = ?, weight = ? WHERE code = ?",
                    vec![category.into(), weight.into(), item_code.into()]
                )).await.map_err(|e| RpcError::Runtime(e.to_string()))?;

                Ok(serde_json::json!({ "status": "success" }))
            }
            "set_item_visibility" => {
                let item_code = payload
                    .get("item")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing 'item'".to_string()))?;
                let visible = payload
                    .get("visible")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| RpcError::BadRequest("Missing 'visible'".to_string()))?;

                self.db
                    .execute(Statement::from_sql_and_values(
                        self.db.get_database_backend(),
                        "UPDATE core_admin_menu_items SET is_hidden = ? WHERE code = ?",
                        vec![(!visible).into(), item_code.into()],
                    ))
                    .await
                    .map_err(|e| RpcError::Runtime(e.to_string()))?;

                Ok(serde_json::json!({ "status": "success" }))
            }
            _ => Err(RpcError::NotFound(method.to_string())),
        }
    }
}
