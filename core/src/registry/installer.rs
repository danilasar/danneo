use crate::registry::{PackageRegistry, ScriptEngine};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set, PaginatorTrait, Statement, ConnectionTrait};
use std::sync::Arc;
use sea_query::{Alias, Query, Expr};

use crate::state::AppState;

pub struct PackageInstaller {
    db: Arc<DatabaseConnection>,
    registry: Arc<tokio::sync::RwLock<PackageRegistry>>,
    modules: Arc<tokio::sync::RwLock<crate::registry::ModuleRegistry>>,
    routes: Arc<tokio::sync::RwLock<crate::registry::RouteRegistry>>,
    script_engine: Arc<ScriptEngine>,
    state: Arc<AppState>,
}

impl PackageInstaller {
    pub fn new(
        db: Arc<DatabaseConnection>,
        registry: Arc<tokio::sync::RwLock<PackageRegistry>>,
        modules: Arc<tokio::sync::RwLock<crate::registry::ModuleRegistry>>,
        routes: Arc<tokio::sync::RwLock<crate::registry::RouteRegistry>>,
        script_engine: Arc<ScriptEngine>,
        state: Arc<AppState>,
    ) -> Self {
        Self {
            db,
            registry,
            modules,
            routes,
            script_engine,
            state,
        }
    }

    pub async fn refresh_registries(&self) {
        self.registry.write().await.scan();
        let packages_dir = self.registry.read().await.packages_dir.clone();

        {
            let modules_guard = self.modules.read().await;
            modules_guard.admin_menus.write().await.clear();
        }

        {
            let mut routes_guard = self.routes.write().await;
            routes_guard.frontend_routes.clear();
            routes_guard.admin_routes.clear();
        }

        let modules_guard = self.modules.read().await;
        modules_guard
            .init(
                self.script_engine.clone(),
                self.routes.clone(),
                packages_dir,
                self.state.clone(),
            )
            .await;
    }

    async fn install_module_blocks(&self, module_code: &str) -> Result<(), String> {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let block_manifests: Vec<_> = {
            let registry = self.registry.read().await;
            registry
                .blocks
                .values()
                .filter(|manifest| manifest.block.module_code == module_code)
                .cloned()
                .collect()
        };

        for manifest in block_manifests {
            let block_code = manifest.block.id.clone();
            let existing = crate::models::core_block_definitions::Entity::find()
                .filter(crate::models::core_block_definitions::Column::BlockCode.eq(&block_code))
                .one(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;

            if existing.is_some() {
                continue;
            }

            let block_def_model = crate::models::core_block_definitions::ActiveModel {
                block_code: Set(block_code),
                module_code: Set(Some(module_code.to_string())),
                package_id: Set(0),
                version: Set(manifest.block.version.clone()),
                enabled: Set(true),
                manifest: Set(serde_json::to_value(&manifest).map_err(|e| e.to_string())?),
                settings_schema: Set(manifest
                    .setting
                    .clone()
                    .map(|s| serde_json::to_value(s).unwrap_or(serde_json::json!([])))),
                template_path: Set(manifest.block.template.clone()),
                renderer_type: Set(manifest
                    .block
                    .renderer
                    .clone()
                    .unwrap_or_else(|| "lua".to_string())),
                ..Default::default()
            };

            block_def_model
                .insert(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub async fn install_from_staging(
        &self,
        package_id: &str,
        staging_path: &std::path::Path,
    ) -> Result<(), String> {
        let manifest_path = staging_path.join("module.toml");
        let manifest_content =
            std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        let manifest: crate::registry::PackageManifest =
            toml::from_str(&manifest_content).map_err(|e| e.to_string())?;

        let module_code = manifest.package.id.clone();
        if module_code != package_id {
            return Err("Package ID mismatch".to_string());
        }

        let now = chrono::Utc::now().into();
        let final_path = {
            let registry = self.registry.read().await;
            registry.packages_dir.join(&module_code)
        };

        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        let existing = crate::models::core_modules::Entity::find()
            .filter(crate::models::core_modules::Column::Code.eq(&module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let db = self.db.as_ref();
        let backup_path = final_path.with_extension("bak");
        let has_backup = if final_path.exists() {
            std::fs::rename(&final_path, &backup_path).map_err(|e| e.to_string())?;
            true
        } else {
            false
        };

        if let Err(e) = std::fs::rename(staging_path, &final_path) {
            if has_backup {
                let _ = std::fs::rename(&backup_path, &final_path);
            }
            return Err(format!("Failed to move package: {}", e));
        }
        self.registry.write().await.scan();

        let db_result: Result<(), String> = async {
            if let Some(module) = existing {
                let mut active_model: crate::models::core_modules::ActiveModel = module.into();
                active_model.version = Set(manifest.package.version.clone());
                active_model.manifest =
                    Set(serde_json::to_value(&manifest).map_err(|e| e.to_string())?);
                active_model.updated_at = Set(now);
                active_model.update(db).await.map_err(|e| e.to_string())?;
            } else {
                let module_model = crate::models::core_modules::ActiveModel {
                    code: Set(module_code.clone()),
                    name: Set(manifest.package.name.clone()),
                    version: Set(manifest.package.version.clone()),
                    package_id: Set(0),
                    package_path: Set(format!("modules/{}", module_code)),
                    package_hash: Set("temp_hash".to_string()),
                    runtime_type: Set(manifest
                        .module
                        .as_ref()
                        .map(|m| m.runtime_type.clone())
                        .unwrap_or_else(|| "lua".to_string())),
                    enabled: Set(manifest
                        .install
                        .as_ref()
                        .and_then(|i| i.default_enabled)
                        .unwrap_or(false)),
                    installed: Set(true),
                    position: Set(0),
                    admin_enabled: Set(manifest
                        .entrypoints
                        .as_ref()
                        .map(|e| e.admin_routes.is_some())
                        .unwrap_or(false)),
                    sitemap_enabled: Set(false),
                    manifest: Set(serde_json::to_value(&manifest).map_err(|e| e.to_string())?),
                    installed_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                };
                module_model.insert(db).await.map_err(|e| e.to_string())?;
            }

            self.apply_lua_migrations(&module_code, &final_path).await?;
            self.install_module_blocks(&module_code).await?;
            Ok(())
        }
        .await;

        if let Err(e) = db_result {
            let _ = std::fs::remove_dir_all(&final_path);
            if has_backup {
                let _ = std::fs::rename(&backup_path, &final_path);
            }
            return Err(format!("Database error: {}", e));
        }

        if let Some(entry) = manifest.entrypoints.as_ref() {
            if let Some(hooks_path) = &entry.hooks {
                let full_hooks_path = final_path.join(hooks_path);
                let _ = self
                    .script_engine
                    .load_module_scripts(&module_code, &full_hooks_path)
                    .await;
                let _ = self
                    .script_engine
                    .call_hook(
                        &module_code,
                        "on_install",
                        script_rhai::Dynamic::UNIT,
                        self.state.clone(),
                    )
                    .await;
            }
        }

        if has_backup {
            let _ = std::fs::remove_dir_all(&backup_path);
        }

        self.refresh_registries().await;
        Ok(())
    }

    async fn apply_lua_migrations(&self, module_code: &str, module_dir: &std::path::Path) -> Result<(), String> {
        let migrations_dir = module_dir.join("migrations");
        if !migrations_dir.exists() { return Ok(()); }

        let mut files: Vec<_> = std::fs::read_dir(migrations_dir).map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "lua"))
            .collect();
        
        files.sort_by_key(|e| e.file_name());

        let db = self.db.as_ref();
        let backend = db.get_database_backend();

        for entry in files {
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            let (sql, values) = sea_query::Query::select()
                .column(sea_query::Alias::new("id"))
                .from(sea_query::Alias::new("core_lua_migrations"))
                .and_where(sea_query::Expr::col(sea_query::Alias::new("module_code")).eq(module_code))
                .and_where(sea_query::Expr::col(sea_query::Alias::new("migration_name")).eq(&file_name))
                .build_any(match backend {
                    sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                    sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                    sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                });

            let exists = db.query_one(Statement::from_sql_and_values(backend, &sql, values))
                .await.unwrap_or(None).is_some();

            if exists { continue; }

            let script = std::fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
            let wrapped_script = format!("
                local mig = (function() 
                    {}
                end)()
                if mig and type(mig.up) == 'function' then
                    return mig.up()
                end
                error('Migration must return a table with an up function')
            ", script);

            self.script_engine.load_script_str(&format!("{}_mig", module_code), &wrapped_script).await.map_err(|e| e.to_string())?;
            let _ = self.script_engine.call_hook(&format!("{}_mig", module_code), "up", script_rhai::Dynamic::UNIT, self.state.clone()).await;

            let (sql, values) = sea_query::Query::insert()
                .into_table(sea_query::Alias::new("core_lua_migrations"))
                .columns([sea_query::Alias::new("module_code"), sea_query::Alias::new("migration_name")])
                .values_panic([module_code.into(), file_name.into()])
                .build_any(match backend {
                    sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                    sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                    sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                });
            db.execute(Statement::from_sql_and_values(backend, &sql, values)).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    async fn rollback_lua_migrations(&self, module_code: &str, module_dir: &std::path::Path) -> Result<(), String> {
        let migrations_dir = module_dir.join("migrations");
        if !migrations_dir.exists() { return Ok(()); }

        let db = self.db.as_ref();
        let backend = db.get_database_backend();

        let (sql, values) = sea_query::Query::select()
            .columns([sea_query::Alias::new("migration_name")])
            .from(sea_query::Alias::new("core_lua_migrations"))
            .and_where(sea_query::Expr::col(sea_query::Alias::new("module_code")).eq(module_code))
            .order_by(sea_query::Alias::new("id"), sea_query::Order::Desc)
            .build_any(match backend {
                sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
            });

        let rows = db.query_all(Statement::from_sql_and_values(backend, &sql, values)).await.map_err(|e| e.to_string())?;

        for row in rows {
            let file_name: String = row.try_get("", "migration_name").unwrap_or_default();
            let file_path = migrations_dir.join(&file_name);
            
            if file_path.exists() {
                let script = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
                let wrapped_script = format!("
                    local mig = (function() 
                        {}
                    end)()
                    if mig and type(mig.down) == 'function' then
                        return mig.down()
                    end
                ", script);

                self.script_engine.load_script_str(&format!("{}_rollback", module_code), &wrapped_script).await.map_err(|e| e.to_string())?;
                let _ = self.script_engine.call_hook(&format!("{}_rollback", module_code), "down", script_rhai::Dynamic::UNIT, self.state.clone()).await;
            }

            let (sql, values) = sea_query::Query::delete()
                .from_table(sea_query::Alias::new("core_lua_migrations"))
                .and_where(sea_query::Expr::col(sea_query::Alias::new("module_code")).eq(module_code))
                .and_where(sea_query::Expr::col(sea_query::Alias::new("migration_name")).eq(&file_name))
                .build_any(match backend {
                    sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                    sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                    sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
                });
            db.execute(Statement::from_sql_and_values(backend, &sql, values)).await.ok();
        }
        Ok(())
    }

    async fn register_native_in_db(&self, module_code: &str, version: &str) -> Result<(), String> {
        use crate::models::core_modules;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let existing = core_modules::Entity::find()
            .filter(core_modules::Column::Code.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        if let Some(model) = existing {
            let mut active = model.into_active_model();
            active.version = Set(version.to_string());
            active.enabled = Set(true);
            active.installed = Set(true);
            active.updated_at = Set(now);
            active.update(self.db.as_ref()).await.map_err(|e| e.to_string())?;
        } else {
            core_modules::ActiveModel {
                code: Set(module_code.to_string()),
                name: Set(module_code.to_string()),
                version: Set(version.to_string()),
                package_id: Set(0),
                package_path: Set(format!("kernel/{}", module_code)),
                package_hash: Set("native".to_string()),
                runtime_type: Set("native".to_string()),
                enabled: Set(true),
                installed: Set(true),
                manifest: Set(serde_json::json!({})),
                installed_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn install(&self, package_id: &str) -> Result<(), String> {
        let registration = inventory::iter::<crate::module::NativeModuleRegistration>()
            .find(|r| r.name == package_id);

        if let Some(reg) = registration {
            let module = (reg.factory)(self.db.clone());
            module.on_install(self.state.clone()).await?;
            module.init(self.state.clone()).await?;
            self.register_native_in_db(package_id, "2.0.0").await?;
            self.refresh_registries().await;
            return Ok(());
        }

        let manifest = {
            let registry = self.registry.read().await;
            registry.packages.get(package_id).cloned()
        }.ok_or_else(|| format!("Package {} not found", package_id))?;

        let mut active_modules = std::collections::HashMap::new();
        use crate::models::core_modules;
        use sea_orm::EntityTrait;
        let installed = core_modules::Entity::find().all(self.db.as_ref()).await.unwrap_or_default();
        for m in installed {
            if m.enabled {
                if let Ok(v) = semver::Version::parse(&m.version) {
                    active_modules.insert(m.code, v);
                }
            }
        }

        if let Some(deps) = &manifest.dependencies {
            for (dep_id, req_str) in deps {
                let req = semver::VersionReq::parse(req_str).map_err(|e| e.to_string())?;
                if let Some(found_ver) = active_modules.get(dep_id) {
                    if !req.matches(found_ver) {
                        return Err(format!("Dependency mismatch: {} requires {}, found {}", package_id, req_str, found_ver));
                    }
                } else {
                    return Err(format!("Missing mandatory dependency: {} for {}", dep_id, package_id));
                }
            }
        }

        let module_code = manifest.package.id.clone();
        let now = chrono::Utc::now().into();

        let module_model = crate::models::core_modules::ActiveModel {
            code: Set(module_code.clone()),
            name: Set(manifest.package.name.clone()),
            version: Set(manifest.package.version.clone()),
            package_id: Set(0),
            package_path: Set(format!("modules/{}", module_code)),
            package_hash: Set("temp_hash".to_string()),
            runtime_type: Set(manifest.module.as_ref().map(|m| m.runtime_type.clone()).unwrap_or_else(|| "lua".to_string())),
            enabled: Set(manifest.install.as_ref().and_then(|i| i.default_enabled).unwrap_or(false)),
            installed: Set(true),
            manifest: Set(serde_json::to_value(&manifest).map_err(|e| e.to_string())?),
            installed_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        module_model.insert(self.db.as_ref()).await.map_err(|e| e.to_string())?;
        let module_dir = {
            let registry = self.registry.read().await;
            registry.packages_dir.join(&module_code)
        };

        self.apply_lua_migrations(&module_code, &module_dir).await?;

        if let Some(entry) = manifest.entrypoints.as_ref() {
            if let Some(hooks_path) = &entry.hooks {
                let full_hooks_path = module_dir.join(hooks_path);
                let _ = self.script_engine.load_module_scripts(&module_code, &full_hooks_path).await;
                let _ = self.script_engine.call_hook(&module_code, "on_install", script_rhai::Dynamic::UNIT, self.state.clone()).await;
            }
        }
        self.install_module_blocks(&module_code).await?;
        self.refresh_registries().await;
        Ok(())
    }

    pub async fn uninstall(&self, package_id: &str) -> Result<(), String> {
        let registration = inventory::iter::<crate::module::NativeModuleRegistration>()
            .find(|r| r.name == package_id);
        
        if let Some(reg) = registration {
             let module = (reg.factory)(self.db.clone());
             module.on_uninstall(self.state.clone()).await?;
        }

        let module_dir = {
            let registry = self.registry.read().await;
            registry.packages_dir.join(package_id)
        };

        self.rollback_lua_migrations(package_id, &module_dir).await.ok();
        let _ = self.script_engine.call_hook(package_id, "on_uninstall", script_rhai::Dynamic::UNIT, self.state.clone()).await;

        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        crate::models::core_modules::Entity::delete_many()
            .filter(crate::models::core_modules::Column::Code.eq(package_id))
            .exec(self.db.as_ref())
            .await.ok();

        self.refresh_registries().await;
        Ok(())
    }

    pub async fn bootstrap(&self) -> Result<(), String> {
        use crate::models::core_modules;
        use sea_orm::{ConnectionTrait, Statement, EntityTrait, ColumnTrait, QueryFilter};
        let db = self.db.as_ref();
        let backend = db.get_database_backend();
        use sea_query::{Alias, Query, Expr};

        let (sql, values) = Query::select()
            .column(Alias::new("value"))
            .from(Alias::new("core_system_state"))
            .and_where(Expr::col(Alias::new("key")).eq("is_bootstrapped"))
            .build_any(match backend {
                sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
            });

        let bootstrapped_row = db.query_one(Statement::from_sql_and_values(backend, &sql, values)).await;
        
        match bootstrapped_row {
            Ok(Some(row)) => {
                 let val: String = row.try_get("", "value").unwrap_or_default();
                 if val == "true" { return Ok(()); }
            },
            _ => {}
        }
        
        let core_modules = vec![
            "admin_menu".to_string(),
            "settings".to_string(),
            "design".to_string(),
            "blocks".to_string(),
            "seo".to_string(),
            "security".to_string(),
            "storage".to_string(),
            "mod_media".to_string(),
            "mod_menu".to_string(),
        ];

        for module_id in core_modules {
            let existing = core_modules::Entity::find()
                .filter(core_modules::Column::Code.eq(&module_id))
                .one(self.db.as_ref())
                .await
                .unwrap_or(None);

            if existing.is_none() {
                let _ = self.install(&module_id).await;
            }
        }

        let (sql, values) = Query::insert()
            .into_table(Alias::new("core_system_state"))
            .columns([Alias::new("key"), Alias::new("value")])
            .values_panic(["is_bootstrapped".into(), "true".into()])
            .on_conflict(sea_query::OnConflict::column(Alias::new("key")).update_column(Alias::new("value")).to_owned())
            .build_any(match backend {
                sea_orm::DatabaseBackend::Postgres => &sea_query::PostgresQueryBuilder,
                sea_orm::DatabaseBackend::MySql => &sea_query::MysqlQueryBuilder,
                sea_orm::DatabaseBackend::Sqlite => &sea_query::SqliteQueryBuilder,
            });

        let _ = db.execute(Statement::from_sql_and_values(backend, &sql, values)).await;
        Ok(())
    }
}
