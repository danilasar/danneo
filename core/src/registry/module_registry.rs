use std::sync::Arc;
use sea_orm::DatabaseConnection;

pub struct ModuleRegistry {
    pub db: Arc<DatabaseConnection>,
}

impl ModuleRegistry {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
        }
    }

    pub async fn init(&self) {
        // Here we will load installed and enabled modules from `core_modules` table
        tracing::info!("Initializing ModuleRegistry");
    }
}
