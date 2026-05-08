use crate::blocks::{BlockContext, DanneoBlock};
use crate::models::core_block_definitions;
use crate::registry::ScriptEngine;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashMap;
use std::sync::Arc;

pub struct BlockRegistry {
    pub db: Arc<DatabaseConnection>,
    pub script_engine: Arc<ScriptEngine>,
    renderers: HashMap<&'static str, Box<dyn DanneoBlock>>,
}

impl BlockRegistry {
    pub fn new(db: Arc<DatabaseConnection>, script_engine: Arc<ScriptEngine>) -> Self {
        let mut registry = Self {
            db,
            script_engine,
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

    pub async fn render_block(
        &self,
        block_code: &str,
        ctx: Arc<BlockContext>,
        settings: Option<serde_json::Value>,
        tera: &tera::Tera,
    ) -> Option<String> {
        let definition = core_block_definitions::Entity::find()
            .filter(core_block_definitions::Column::BlockCode.eq(block_code))
            .filter(core_block_definitions::Column::Enabled.eq(true))
            .one(self.db.as_ref())
            .await
            .unwrap_or(None)?;

        match definition.renderer_type.as_str() {
            "native" => {
                if let Some(renderer) = self.renderers.get(block_code) {
                    return Some(renderer.render(ctx, settings).await);
                }
            }
            "script" => {
                let module_code = definition.module_code.as_ref()?;
                let arg = serde_json::json!({
                    "block_code": block_code,
                    "settings": settings,
                });
                let dynamic_arg = script_rhai::serde::to_dynamic(arg).unwrap();

                match self
                    .script_engine
                    .call_hook(module_code, "render_block", dynamic_arg)
                    .await
                {
                    Ok(res) => {
                        if let Some(html) = res.clone().try_cast::<String>() {
                            return Some(html);
                        } else if let Some(res_map) = res.try_cast::<script_rhai::Map>() {
                            let template = res_map
                                .get("template")
                                .and_then(|v| v.clone().into_string().ok())?;
                            let context_val =
                                res_map.get("context").cloned().unwrap_or_else(|| {
                                    script_rhai::Dynamic::from(script_rhai::Map::new())
                                });

                            let mut context = tera::Context::new();
                            if let Ok(ctx_json) =
                                script_rhai::serde::from_dynamic::<serde_json::Value>(&context_val)
                            {
                                if let Some(obj) = ctx_json.as_object() {
                                    for (k, v) in obj {
                                        context.insert(k, v);
                                    }
                                }
                            }
                            if let Some(s) = settings {
                                context.insert("settings", &s);
                            }

                            let full_template_path =
                                format!("{}/templates/{}", module_code, template);
                            match tera.render(&full_template_path, &context) {
                                Ok(html) => return Some(html),
                                Err(e) => {
                                    tracing::error!(
                                        "Block {} script template error: {}",
                                        block_code,
                                        e
                                    );
                                    return Some(format!("Template error: {}", e));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Block {} script error: {}", block_code, e);
                        return Some(format!("Script error: {}", e));
                    }
                }
            }
            "declarative" => {
                let mut context = tera::Context::new();
                if let Some(s) = settings {
                    context.insert("settings", &s);
                }

                context.insert(
                    "items",
                    &vec![
                        serde_json::json!({"message": "Привет из декларативного блока!"}),
                        serde_json::json!({"message": "Контент из модуля Hello."}),
                    ],
                );

                if let Some(template_path) = definition.template_path {
                    let full_template_path = format!("{}/{}", block_code, template_path);
                    match tera.render(&full_template_path, &context) {
                        Ok(html) => return Some(html),
                        Err(e) => {
                            tracing::error!("Block {} template error: {}", block_code, e);
                            return Some(format!("Template error: {}", e));
                        }
                    }
                }
            }
            _ => {}
        }

        tracing::warn!("Renderer for block {} not found or failed", block_code);
        None
    }

    pub async fn get_all_positions_html(
        &self,
        ctx: Arc<BlockContext>,
        tera: &tera::Tera,
    ) -> HashMap<String, String> {
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
            if let Some(block_html) = self
                .render_block(block_code, ctx.clone(), config.block_setting, tera)
                .await
            {
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
