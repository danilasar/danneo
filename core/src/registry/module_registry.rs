use crate::models::core_modules;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, IntoActiveModel};
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
        let model_opt = core_modules::Entity::find()
            .filter(core_modules::Column::Code.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB Error: {}", e))?;

        if let Some(model) = model_opt {
            let mut active_model = model.into_active_model();
            active_model.enabled = Set(true);
            active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| format!("DB Error: {}", e))?;
            return Ok(());
        }

        use crate::models::core_block_definitions;
        let block_opt = core_block_definitions::Entity::find()
            .filter(core_block_definitions::Column::BlockCode.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB Error: {}", e))?;

        if let Some(block) = block_opt {
            let mut active_model = block.into_active_model();
            active_model.enabled = Set(true);
            active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| format!("DB Error: {}", e))?;
            return Ok(());
        }

        Err(format!("Package {} not found", module_code))
    }

    pub async fn disable(&self, module_code: &str) -> Result<(), String> {
        let model_opt = core_modules::Entity::find()
            .filter(core_modules::Column::Code.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB Error: {}", e))?;

        if let Some(model) = model_opt {
            let mut active_model = model.into_active_model();
            active_model.enabled = Set(false);
            active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| format!("DB Error: {}", e))?;
            return Ok(());
        }

        use crate::models::core_block_definitions;
        let block_opt = core_block_definitions::Entity::find()
            .filter(core_block_definitions::Column::BlockCode.eq(module_code))
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB Error: {}", e))?;

        if let Some(block) = block_opt {
            let mut active_model = block.into_active_model();
            active_model.enabled = Set(false);
            active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| format!("DB Error: {}", e))?;
            return Ok(());
        }

        Err(format!("Package {} not found", module_code))
    }
}
