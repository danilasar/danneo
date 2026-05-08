use crate::models::core_settings;
use crate::blocks::BlockManager;
use crate::acl::service::AclService;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GlobalSettings {
    pub site_name: String,
    pub admin_email: String,
    pub site_url: String,
    pub site_temp: String,
}

/// Глобальное состояние приложения (Ядра).
/// Доступно каждому роуту и каждому модулю.
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub settings: Arc<tokio::sync::RwLock<GlobalSettings>>,
    pub tera: Arc<tera::Tera>,
    pub block_manager: Arc<BlockManager>,
    pub jwt_secret: String,
    pub acl: Arc<AclService>,
}

impl AppState {
    pub async fn new(db: DatabaseConnection) -> Result<Self, String> {
        let db_arc = Arc::new(db);
        // Пытаемся загрузить настройки из БД
        let settings_records = core_settings::Entity::find()
            .all(db_arc.as_ref())
            .await
            .map_err(|e| format!("Failed to load settings: {}", e))?;

        let mut settings = GlobalSettings::default();
        for record in settings_records {
            match record.key.as_str() {
                "site_name" => {
                    if let Some(val) = record.value.as_str() {
                        settings.site_name = val.to_string();
                    }
                }
                "admin_email" => {
                    if let Some(val) = record.value.as_str() {
                        settings.admin_email = val.to_string();
                    }
                }
                "site_url" => {
                    if let Some(val) = record.value.as_str() {
                        settings.site_url = val.to_string();
                    }
                }
                "site_temp" => {
                    if let Some(val) = record.value.as_str() {
                        settings.site_temp = val.to_string();
                    }
                }
                _ => {}
            }
        }
        let settings = Arc::new(tokio::sync::RwLock::new(settings));

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "super_secret_key".to_string());
        
        let block_manager = Arc::new(BlockManager::new());

        // Инициализируем ACL
        let model_path = if std::path::Path::new("core/casbin_models/rbac_with_level.conf").exists() {
            "core/casbin_models/rbac_with_level.conf"
        } else {
            "casbin_models/rbac_with_level.conf"
        };
        
        let acl = AclService::new_db(db_arc.clone(), model_path).await;
        let acl = Arc::new(acl);

        // Инициализируем Tera, загружая шаблоны из файловой системы
        let template_path = if std::path::Path::new("core/templates").exists() {
            "core/templates/**/*"
        } else {
            "templates/**/*"
        };
        
        let tera = tera::Tera::new(template_path)
            .map_err(|e| format!("Failed to initialize Tera: {}", e))?;

        Ok(Self { 
            db: db_arc, 
            settings,
            tera: Arc::new(tera),
            block_manager,
            jwt_secret,
            acl,
        })
    }
}
