use crate::models::core_settings;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GlobalSettings {
    pub site_name: String,
    pub admin_email: String,
}

/// Глобальное состояние приложения (Ядра).
/// Доступно каждому роуту и каждому модулю.
pub struct AppState {
    pub db: DatabaseConnection,
    pub settings: GlobalSettings,
    pub tera: std::sync::Arc<tera::Tera>,
    pub jwt_secret: String,
}

impl AppState {
    pub async fn new(db: DatabaseConnection) -> Result<Self, String> {
        // Пытаемся загрузить настройки из БД
        let settings_records = core_settings::Entity::find()
            .all(&db)
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
                _ => {}
            }
        }

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "super_secret_key".to_string());

        // Инициализируем Tera, загружая шаблоны из файловой системы
        let tera = tera::Tera::new("core/templates/**/*")
            .map_err(|e| format!("Failed to initialize Tera: {}", e))?;

        Ok(Self { 
            db, 
            settings,
            tera: std::sync::Arc::new(tera),
            jwt_secret,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};

    #[tokio::test]
    async fn test_app_state_initialization_with_settings() {
        // Создаем мок базы данных с предопределенными результатами
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![
                    core_settings::Model {
                        key: "site_name".to_string(),
                        value: serde_json::json!("Test Site"),
                    },
                    core_settings::Model {
                        key: "admin_email".to_string(),
                        value: serde_json::json!("admin@test.com"),
                    },
                ]
            ])
            .into_connection();
            
        // Проверяем, что AppState инициализируется
        let state_result = AppState::new(db).await;
        assert!(state_result.is_ok(), "AppState should initialize successfully");
        
        let state = state_result.unwrap();
        assert_eq!(state.settings.site_name, "Test Site");
        assert_eq!(state.settings.admin_email, "admin@test.com");
    }
}
