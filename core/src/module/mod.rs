use crate::state::AppState;
use async_trait::async_trait;
use axum::Router;
use serde_json::Value;
use std::sync::Arc;

pub mod admin_menu;
pub mod native_demo;
pub mod settings;
pub mod seo;
pub mod design;
pub mod blocks;
pub mod security;

#[derive(Clone, Debug)]
pub struct NativeBlockDefinition {
    pub block_code: &'static str,
    pub version: &'static str,
    pub settings_schema: Option<Value>,
}

/// Базовый трейт, который должен реализовать каждый модуль Danneo.
/// Он определяет жизненный цикл и интеграцию модуля в систему.
#[async_trait]
pub trait DanneoModule: Send + Sync {
    /// Уникальное системное имя модуля (например, "news", "article")
    fn name(&self) -> &'static str;

    /// Инициализация модуля (вызывается при старте приложения).
    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        Ok(())
    }

    /// Вызывается один раз при установке модуля.
    async fn on_install(&self, _state: Arc<AppState>) -> Result<(), String> {
        Ok(())
    }

    /// Вызывается один раз при удалении модуля.
    async fn on_uninstall(&self, _state: Arc<AppState>) -> Result<(), String> {
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

    /// Описание маршрутов для реестра (Frontend)
    fn frontend_routes(&self) -> Vec<crate::registry::RouteDescriptor> {
        Vec::new()
    }

    /// Описание маршрутов для реестра (Admin)
    fn admin_routes(&self) -> Vec<crate::registry::RouteDescriptor> {
        Vec::new()
    }

    /// Блоки, предоставляемые native-модулем.
    fn block_definitions(&self) -> Vec<NativeBlockDefinition> {
        Vec::new()
    }

    /// Рендеринг native-блока, принадлежащего модулю.
    async fn render_block(
        &self,
        _block_code: &str,
        _ctx: Arc<crate::blocks::BlockContext>,
        _settings: Option<Value>,
    ) -> Option<String> {
        None
    }

    /// Регистрация RPC методов модуля.
    fn rpc_methods(&self) -> Vec<crate::rpc::RpcMethodDescriptor> {
        Vec::new()
    }

    /// Вызов RPC метода (для native модулей).
    async fn call_rpc(
        &self,
        _method: &str,
        _payload: serde_json::Value,
        _ctx: crate::rpc::RpcContext,
        _state: Arc<AppState>,
    ) -> Result<serde_json::Value, crate::rpc::RpcError> {
        Err(crate::rpc::RpcError::NotFound(
            "Native RPC not implemented".to_string(),
        ))
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
