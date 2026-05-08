use crate::state::GlobalSettings;
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use serde_json::Value;
use std::sync::Arc;

pub mod menu;

/// Контекст для рендеринга блоков (без Tera, чтобы избежать циклов)
pub struct BlockContext {
    pub db: Arc<DatabaseConnection>,
    pub settings: Arc<tokio::sync::RwLock<GlobalSettings>>,
}

#[async_trait]
pub trait DanneoBlock: Send + Sync {
    fn identifier(&self) -> &'static str;
    async fn render(&self, ctx: Arc<BlockContext>, settings: Option<Value>) -> String;
}

// Тестовый блок (заглушка)
pub struct SampleBlock;

#[async_trait]
impl DanneoBlock for SampleBlock {
    fn identifier(&self) -> &'static str {
        "sample_block"
    }
    async fn render(&self, _ctx: Arc<BlockContext>, _settings: Option<Value>) -> String {
        "Это тестовый блок Danneo".to_string()
    }
}
