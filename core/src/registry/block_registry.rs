use std::sync::Arc;
use std::collections::HashMap;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use crate::models::core_block_definitions;
use crate::blocks::{DanneoBlock, BlockContext};

pub struct BlockRegistry {
    pub db: Arc<DatabaseConnection>,
    renderers: HashMap<&'static str, Box<dyn DanneoBlock>>,
}

impl BlockRegistry {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let mut registry = Self {
            db,
            renderers: HashMap::new(),
        };
        // Register native renderers
        registry.register(Box::new(crate::blocks::SampleBlock));
        registry.register(Box::new(crate::blocks::menu::MenuBlock));
        registry
    }

    pub fn register(&mut self, block: Box<dyn DanneoBlock>) {
        self.renderers.insert(block.identifier(), block);
    }

    pub async fn init(&self) {
        use sea_orm::{ActiveModelTrait, Set};

        for &identifier in self.renderers.keys() {
            let exists = core_block_definitions::Entity::find()
                .filter(core_block_definitions::Column::BlockCode.eq(identifier))
                .one(self.db.as_ref())
                .await
                .unwrap_or(None)
                .is_some();

            if !exists {
                let model = core_block_definitions::ActiveModel {
                    block_code: Set(identifier.to_string()),
                    module_code: Set(None),
                    package_id: Set(0),
                    version: Set("1.0.0".to_string()),
                    enabled: Set(true),
                    manifest: Set(serde_json::json!({})),
                    settings_schema: Set(Some(serde_json::json!([]))),
                    template_path: Set(None),
                    renderer_type: Set("native".to_string()),
                    ..Default::default()
                };

                if let Err(e) = model.insert(self.db.as_ref()).await {
                    tracing::error!("Failed to auto-register native block {}: {}", identifier, e);
                } else {
                    tracing::info!("Auto-registered native block: {}", identifier);
                }
            }
        }
    }

    pub async fn render_block(&self, block_code: &str, ctx: Arc<BlockContext>, settings: Option<serde_json::Value>) -> Option<String> {
        let definition = core_block_definitions::Entity::find()
            .filter(core_block_definitions::Column::BlockCode.eq(block_code))
            .filter(core_block_definitions::Column::Enabled.eq(true))
            .one(self.db.as_ref())
            .await
            .unwrap_or(None)?;

        let identifier = match definition.renderer_type.as_str() {
            "native" => block_code,
            _ => block_code,
        };

        if let Some(renderer) = self.renderers.get(identifier) {
            Some(renderer.render(ctx, settings).await)
        } else {
            tracing::warn!("Renderer for block {} not found", block_code);
            None
        }
    }

    pub async fn get_all_positions_html(&self, ctx: Arc<BlockContext>) -> HashMap<String, String> {
        use crate::models::core_blocks;
        use sea_orm::QueryOrder;
        
        let db = &self.db;
        let mut results = HashMap::new();

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
            let block_code = config.block_file.as_str(); // using block_file as block_code
            if let Some(block_html) = self.render_block(block_code, ctx.clone(), config.block_setting).await {
                let entry = results
                    .entry(config.positcode.clone())
                    .or_insert_with(String::new);

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
