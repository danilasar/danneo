use crate::models::core_modules;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;

pub struct ModuleRegistry {
    pub db: Arc<DatabaseConnection>,
}

impl ModuleRegistry {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn init(&self) {
        tracing::info!("Initializing ModuleRegistry");

        match core_modules::Entity::find()
            .filter(core_modules::Column::Enabled.eq(true))
            .all(self.db.as_ref())
            .await
        {
            Ok(modules) => {
                tracing::info!("Loaded {} active modules", modules.len());
            }
            Err(e) => {
                tracing::error!("Failed to load active modules: {}", e);
            }
        }
    }

    pub async fn enable(&self, module_code: &str) -> Result<(), String> {
        let module = core_modules::Entity::find()
            .filter(core_modules::Column::Code.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Module {} not found", module_code))?;

        let mut active_model: core_modules::ActiveModel = module.into();
        active_model.enabled = Set(true);
        active_model.updated_at = Set(chrono::Utc::now().into());

        active_model
            .update(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn disable(&self, module_code: &str) -> Result<(), String> {
        let module = core_modules::Entity::find()
            .filter(core_modules::Column::Code.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Module {} not found", module_code))?;

        let mut active_model: core_modules::ActiveModel = module.into();
        active_model.enabled = Set(false);
        active_model.updated_at = Set(chrono::Utc::now().into());

        active_model
            .update(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
