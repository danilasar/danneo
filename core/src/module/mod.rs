use crate::state::AppState;
use async_trait::async_trait;
use axum::Router;
use std::sync::Arc;

pub mod native_demo;

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
    use sea_orm::Database;

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
        assert_eq!(module.name(), "test_module");

        let db = Database::connect("sqlite::memory:").await.unwrap();
        use sea_orm_migration::MigratorTrait;
        migration::Migrator::up(&db, None).await.unwrap();

        let state = Arc::new(AppState::new(db).await.unwrap());
        let init_result = module.init(state).await;
        assert!(init_result.is_ok());
    }
}
