use crate::models::core_blocks;
use crate::state::GlobalSettings;
use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;
use std::collections::HashMap;
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

pub struct BlockManager {
    registry: std::collections::HashMap<&'static str, Box<dyn DanneoBlock>>,
}

impl BlockManager {
    pub fn new() -> Self {
        let mut manager = Self {
            registry: std::collections::HashMap::new(),
        };
        manager.register(Box::new(SampleBlock));
        manager.register(Box::new(menu::MenuBlock));
        manager
    }

    pub fn register(&mut self, block: Box<dyn DanneoBlock>) {
        self.registry.insert(block.identifier(), block);
    }

    pub async fn get_all_positions_html(&self, ctx: Arc<BlockContext>) -> HashMap<String, String> {
        let db = &ctx.db;
        let mut results = HashMap::new();

        // Получаем все активные блоки
        let blocks_configs = match core_blocks::Entity::find()
            .filter(core_blocks::Column::BlockActive.eq(true))
            .order_by_asc(core_blocks::Column::Positcode)
            .order_by_asc(core_blocks::Column::BlockWeight)
            .all(db.as_ref())
            .await
        {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to fetch all blocks: {}", e);
                return results;
            }
        };

        for config in blocks_configs {
            if let Some(block_logic) = self.registry.get(config.block_file.as_str()) {
                let block_html = block_logic.render(ctx.clone(), config.block_setting).await;

                let entry = results
                    .entry(config.positcode.clone())
                    .or_insert_with(String::new);

                // Оборачиваем в шаблон блока
                entry.push_str(&format!(
                    "<div class=\"block-container\" id=\"block-{}\">\n",
                    config.id
                ));
                if !config.block_name.is_empty() {
                    entry.push_str(&format!(
                        "<div class=\"block-title\">{}</div>\n",
                        config.block_name
                    ));
                }
                entry.push_str("<div class=\"block-content\">\n");
                entry.push_str(&block_html);
                entry.push_str("\n</div>\n</div>\n");
            }
        }

        results
    }
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
