use crate::registry::{PackageRegistry, ScriptEngine};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;

pub struct PackageInstaller {
    db: Arc<DatabaseConnection>,
    registry: Arc<tokio::sync::RwLock<PackageRegistry>>,
    modules: Arc<tokio::sync::RwLock<crate::registry::ModuleRegistry>>,
    routes: Arc<tokio::sync::RwLock<crate::registry::RouteRegistry>>,
    script_engine: Arc<ScriptEngine>,
}

impl PackageInstaller {
    pub fn new(
        db: Arc<DatabaseConnection>,
        registry: Arc<tokio::sync::RwLock<PackageRegistry>>,
        modules: Arc<tokio::sync::RwLock<crate::registry::ModuleRegistry>>,
        routes: Arc<tokio::sync::RwLock<crate::registry::RouteRegistry>>,
        script_engine: Arc<ScriptEngine>,
    ) -> Self {
        Self {
            db,
            registry,
            modules,
            routes,
            script_engine,
        }
    }

    pub async fn refresh_registries(&self) {
        // 1. Refresh package registry
        self.registry.write().await.scan();

        // 2. Refresh module and route registries
        let packages_dir = self.registry.read().await.packages_dir.clone();

        {
            let modules_guard = self.modules.read().await;
            modules_guard.admin_menus.write().await.clear();
        }

        {
            let mut routes_guard = self.routes.write().await;
            routes_guard.routes.clear();
        }

        let modules_guard = self.modules.read().await;
        modules_guard
            .init(
                self.script_engine.clone(),
                self.routes.clone(),
                packages_dir,
            )
            .await;
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

        // Check if upgrade
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        let existing = crate::models::core_modules::Entity::find()
            .filter(crate::models::core_modules::Column::Code.eq(&module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        // Transactional install/update
        let db = self.db.as_ref();

        // 1. Move files with backup for rollback
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

        // 2. Database update
        let db_result: Result<(), String> = async {
            if let Some(module) = existing {
                let mut active_model: crate::models::core_modules::ActiveModel = module.into();
                active_model.version = Set(manifest.package.version.clone());
                active_model.manifest =
                    Set(serde_json::to_value(&manifest).map_err(|e| e.to_string())?);
                active_model.updated_at = Set(now);
                active_model.update(db).await.map_err(|e| e.to_string())?;
                tracing::info!(
                    "Module {} updated to version {}",
                    module_code,
                    manifest.package.version
                );
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
                        .unwrap_or_else(|| "declarative".to_string())),
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
                tracing::info!(
                    "Module {} installed version {}",
                    module_code,
                    manifest.package.version
                );
            }

            // 3. Entity handling
            if let Some(entry) = manifest.entrypoints.as_ref() {
                if let Some(entities_path) = &entry.entities {
                    let ent_path = final_path.join(entities_path);
                    let content = std::fs::read_to_string(&ent_path).map_err(|e| e.to_string())?;
                    let schema: crate::crud::EntitySchema =
                        serde_json::from_str(&content).map_err(|e| e.to_string())?;

                    // Create physical table (if not exists)
                    crate::crud::create_entity_table(&self.db, &schema)
                        .await
                        .map_err(|e| e.to_string())?;

                    // Update metadata
                    use crate::models::core_module_entities::Entity as EntEntity;
                    let existing_ent = EntEntity::find()
                        .filter(
                            crate::models::core_module_entities::Column::ModuleCode
                                .eq(&module_code),
                        )
                        .filter(
                            crate::models::core_module_entities::Column::EntityName
                                .eq(&schema.table_name),
                        )
                        .one(db)
                        .await
                        .map_err(|e| e.to_string())?;

                    if let Some(ent) = existing_ent {
                        let mut ent_active: crate::models::core_module_entities::ActiveModel =
                            ent.into();
                        ent_active.schema = Set(serde_json::to_value(&schema).unwrap());
                        ent_active.update(db).await.map_err(|e| e.to_string())?;
                    } else {
                        let entity_model = crate::models::core_module_entities::ActiveModel {
                            module_code: Set(module_code.clone()),
                            entity_name: Set(schema.table_name.clone()),
                            table_name: Set(schema.table_name.clone()),
                            schema: Set(serde_json::to_value(&schema).unwrap()),
                            ..Default::default()
                        };
                        entity_model.insert(db).await.map_err(|e| e.to_string())?;
                    }
                }
            }
            Ok(())
        }
        .await;

        if let Err(e) = db_result {
            // Rollback files
            let _ = std::fs::remove_dir_all(&final_path);
            if has_backup {
                let _ = std::fs::rename(&backup_path, &final_path);
            }
            return Err(format!("Database error during installation: {}", e));
        }

        // 4. Hook loading (we don't rollback if hooks fail for now, but we could)
        if let Some(entry) = manifest.entrypoints.as_ref() {
            if let Some(hooks_path) = &entry.hooks {
                let full_hooks_path = final_path.join(hooks_path);
                let _ = self
                    .script_engine
                    .load_module_scripts(&module_code, &full_hooks_path)
                    .await;
                let _ = self
                    .script_engine
                    .call_hook(&module_code, "on_install", script_rhai::Dynamic::UNIT)
                    .await;
            }
        }

        // Clean up backup
        if has_backup {
            let _ = std::fs::remove_dir_all(&backup_path);
        }

        // 5. Rescan registries to pick up changes
        self.refresh_registries().await;

        Ok(())
    }

    pub async fn install(&self, package_id: &str) -> Result<(), String> {
        let manifest_data = {
            let registry = self.registry.read().await;
            if let Some(m) = registry.packages.get(package_id) {
                Some((m.clone(), true)) // (manifest, is_module)
            } else {
                None
            }
        };

        if let Some((manifest, is_module)) = manifest_data {
            if is_module {
                let module_code = manifest.package.id.clone();
                let now = chrono::Utc::now().into();

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
                        .unwrap_or_else(|| "declarative".to_string())),
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

                if let Err(e) = module_model.insert(self.db.as_ref()).await {
                    tracing::error!("Failed to install module {}: {}", package_id, e);
                    return Err(e.to_string());
                }

                if let Some(entry) = manifest.entrypoints.as_ref() {
                    let module_dir = {
                        let registry = self.registry.read().await;
                        registry.packages_dir.join(&module_code)
                    };
                    if let Some(hooks_path) = &entry.hooks {
                        let full_hooks_path = module_dir.join(hooks_path);
                        if let Err(e) = self
                            .script_engine
                            .load_module_scripts(&module_code, &full_hooks_path)
                            .await
                        {
                            tracing::error!(
                                "Failed to load hooks for module {}: {}",
                                module_code,
                                e
                            );
                        } else {
                            tracing::info!("Calling on_install hook for module {}", &module_code);
                            let result = self
                                .script_engine
                                .call_hook(&module_code, "on_install", script_rhai::Dynamic::UNIT)
                                .await;
                            if let Err(e) = result {
                                tracing::error!(
                                    "Failed to run on_install hook for module {}: {}",
                                    module_code,
                                    e
                                );
                            }
                        }
                    }
                    if let Some(entities_path) = &entry.entities {
                        let ent_path = module_dir.join(entities_path);
                        let content =
                            std::fs::read_to_string(&ent_path).map_err(|e| e.to_string())?;
                        let schema: crate::crud::EntitySchema =
                            serde_json::from_str(&content).map_err(|e| e.to_string())?;
                        crate::crud::create_entity_table(&self.db, &schema)
                            .await
                            .map_err(|e| e.to_string())?;
                        let entity_model = crate::models::core_module_entities::ActiveModel {
                            module_code: Set(module_code.clone()),
                            entity_name: Set(schema.table_name.clone()),
                            table_name: Set(schema.table_name.clone()),
                            schema: Set(serde_json::to_value(&schema).unwrap()),
                            ..Default::default()
                        };
                        entity_model
                            .insert(self.db.as_ref())
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                }
                self.refresh_registries().await;
                return Ok(());
            }
        }

        // Handle blocks
        let block_manifest = {
            let registry = self.registry.read().await;
            registry.blocks.get(package_id).cloned()
        };

        if let Some(manifest) = block_manifest {
            let block_code = manifest.block.id.clone();
            let block_def_model = crate::models::core_block_definitions::ActiveModel {
                block_code: Set(block_code.clone()),
                module_code: Set(manifest.block.module.clone()),
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
                    .unwrap_or_else(|| "declarative".to_string())),
                ..Default::default()
            };

            if let Err(e) = block_def_model.insert(self.db.as_ref()).await {
                tracing::error!("Failed to install block {}: {}", package_id, e);
                return Err(e.to_string());
            }
            self.refresh_registries().await;
            return Ok(());
        }

        Err(format!("Package {} not found in registry", package_id))
    }

    pub async fn uninstall(&self, package_id: &str) -> Result<(), String> {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        // Delete module record and clean up dynamic entities
        let module_res = crate::models::core_modules::Entity::delete_many()
            .filter(crate::models::core_modules::Column::Code.eq(package_id))
            .exec(self.db.as_ref())
            .await;

        if let Ok(res) = module_res {
            if res.rows_affected > 0 {
                // Remove dynamic entity tables and metadata
                use crate::models::core_module_entities::Entity as EntEntity;
                let ents = EntEntity::find()
                    .filter(crate::models::core_module_entities::Column::ModuleCode.eq(package_id))
                    .all(self.db.as_ref())
                    .await
                    .unwrap_or_default();
                for ent in ents {
                    let _ = crate::crud::drop_entity_table(&self.db, &ent.table_name).await;
                }
                // Delete metadata rows
                EntEntity::delete_many()
                    .filter(crate::models::core_module_entities::Column::ModuleCode.eq(package_id))
                    .exec(self.db.as_ref())
                    .await
                    .ok();

                // Rescan registries
                self.refresh_registries().await;
                tracing::info!(
                    "Module {} uninstalled and dynamic tables dropped",
                    package_id
                );
                return Ok(());
            }
        }

        // Fallback to block uninstallation
        let block_res = crate::models::core_block_definitions::Entity::delete_many()
            .filter(crate::models::core_block_definitions::Column::BlockCode.eq(package_id))
            .exec(self.db.as_ref())
            .await;

        if let Ok(res) = block_res {
            if res.rows_affected > 0 {
                // Rescan registries
                self.refresh_registries().await;
                tracing::info!("Block {} uninstalled successfully", package_id);
                return Ok(());
            }
        }

        Err(format!("Package {} is not installed", package_id))
    }
}
