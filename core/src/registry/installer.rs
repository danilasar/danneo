use crate::models::core_modules;
use crate::registry::PackageRegistry;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;

pub struct PackageInstaller {
    db: Arc<DatabaseConnection>,
    registry: Arc<tokio::sync::RwLock<PackageRegistry>>,
}

impl PackageInstaller {
    pub fn new(
        db: Arc<DatabaseConnection>,
        registry: Arc<tokio::sync::RwLock<PackageRegistry>>,
    ) -> Self {
        Self { db, registry }
    }

    pub async fn install(&self, package_id: &str) -> Result<(), String> {
        let registry = self.registry.read().await;

        if let Some(manifest) = registry.packages.get(package_id) {
            if manifest.package.package_type != "module" {
                return Err("Only module packages can be installed via this method".to_string());
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
                manifest: Set(serde_json::to_value(manifest).map_err(|e| e.to_string())?),
                installed_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };

            if let Err(e) = module_model.insert(self.db.as_ref()).await {
                tracing::error!("Failed to install module {}: {}", package_id, e);
                return Err(e.to_string());
            }

            tracing::info!("Module {} installed successfully", package_id);
            Ok(())
        } else if let Some(manifest) = registry.blocks.get(package_id) {
            let block_code = manifest.block.id.clone();
            
            let block_def_model = crate::models::core_block_definitions::ActiveModel {
                block_code: Set(block_code.clone()),
                module_code: Set(None),
                package_id: Set(0),
                version: Set(manifest.block.version.clone()),
                enabled: Set(true),
                manifest: Set(serde_json::to_value(manifest).map_err(|e| e.to_string())?),
                settings_schema: Set(manifest.setting.clone().map(|s| serde_json::to_value(s).unwrap_or(serde_json::json!([])))),
                template_path: Set(manifest.block.template.clone()),
                renderer_type: Set(manifest.block.renderer.clone().unwrap_or_else(|| "declarative".to_string())),
                ..Default::default()
            };

            if let Err(e) = block_def_model.insert(self.db.as_ref()).await {
                tracing::error!("Failed to install block {}: {}", package_id, e);
                return Err(e.to_string());
            }

            tracing::info!("Block {} installed successfully", package_id);
            Ok(())
        } else {
            Err(format!("Package {} not found in registry", package_id))
        }
    }

    pub async fn uninstall(&self, package_id: &str) -> Result<(), String> {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let module_res = crate::models::core_modules::Entity::delete_many()
            .filter(crate::models::core_modules::Column::Code.eq(package_id))
            .exec(self.db.as_ref())
            .await;

        if let Ok(res) = module_res {
            if res.rows_affected > 0 {
                tracing::info!("Module {} uninstalled successfully", package_id);
                return Ok(());
            }
        }

        let block_res = crate::models::core_block_definitions::Entity::delete_many()
            .filter(crate::models::core_block_definitions::Column::BlockCode.eq(package_id))
            .exec(self.db.as_ref())
            .await;

        if let Ok(res) = block_res {
            if res.rows_affected > 0 {
                tracing::info!("Block {} uninstalled successfully", package_id);
                return Ok(());
            }
        }

        Err(format!("Package {} is not installed", package_id))
    }
}

