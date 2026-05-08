use axum::Router;
use async_trait::async_trait;
use std::sync::Arc;
use crate::state::AppState;

/// Базовый трейт, который должен реализовать каждый модуль Danneo.
/// Он определяет жизненный цикл и интеграцию модуля в систему.
#[async_trait]
pub trait DanneoModule: Send + Sync {
    /// Уникальное системное имя модуля (например, "news", "article")
    fn name(&self) -> &'static str;

    /// Инициализация модуля (вызывается при старте приложения).
    /// Здесь модуль должен проверять и накатывать свои миграции БД.
    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        // По умолчанию ничего не делаем
        Ok(())
    }

    /// Регистрация маршрутов для публичной части (Frontend).
    /// Возвращает Router, который ядро примонтирует к пути /<module_name>
    fn register_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
    }

    /// Регистрация маршрутов для панели управления (APanel).
    /// Возвращает Router, который ядро примонтирует к пути /admin/<module_name>
    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, MockDatabase};

    // Создаем тестовую реализацию модуля
    struct TestModule;
    
    #[async_trait]
    impl DanneoModule for TestModule {
        fn name(&self) -> &'static str {
            "test_module"
        }
    }

    #[tokio::test]
    async fn test_module_defaults() {
        let module = TestModule;
        
        // Проверяем имя
        assert_eq!(module.name(), "test_module");
        
        // Проверяем, что дефолтный init не возвращает ошибку
        // Нужно подсунуть MockDatabase результаты для загрузки настроек
        use crate::models::core_settings;
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([vec![
                core_settings::Model {
                    key: "site_name".to_string(),
                    value: serde_json::json!("Test"),
                }
            ]])
            .into_connection();
        let state = Arc::new(AppState::new(db).await.unwrap());
        
        let init_result = module.init(state).await;
        assert!(init_result.is_ok());
    }
}
