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

        let manifest = registry
            .packages
            .get(package_id)
            .ok_or_else(|| format!("Package {} not found in registry", package_id))?;

        if manifest.package.package_type != "module" {
            return Err("Only module packages can be installed via this method".to_string());
        }

        let module_code = manifest.package.id.clone();

        let now = chrono::Utc::now().into();

        let module_model = core_modules::ActiveModel {
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
    }

    pub async fn uninstall(&self, module_code: &str) -> Result<(), String> {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        match core_modules::Entity::delete_many()
            .filter(core_modules::Column::Code.eq(module_code))
            .exec(self.db.as_ref())
            .await
        {
            Ok(res) if res.rows_affected > 0 => {
                tracing::info!("Module {} uninstalled successfully", module_code);
                Ok(())
            }
            Ok(_) => Err(format!("Module {} is not installed", module_code)),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use std::path::PathBuf;

    async fn setup_db() -> Arc<DatabaseConnection> {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        use sea_orm_migration::MigratorTrait;
        migration::Migrator::up(&db, None).await.unwrap();
        Arc::new(db)
    }

    #[tokio::test]
    async fn test_install_package_not_found() {
        let db = setup_db().await;
        let mut registry =
            PackageRegistry::new(PathBuf::from("nonexistent"), PathBuf::from("nonexistent"));
        registry.scan();
        let registry = Arc::new(tokio::sync::RwLock::new(registry));

        let installer = PackageInstaller::new(db, registry);
        let result = installer.install("some_pkg").await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Package some_pkg not found in registry"
        );
    }
}
