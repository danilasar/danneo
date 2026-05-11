use crate::blocks::{BlockContext, DanneoBlock};
use crate::models::core_block_definitions;
use crate::module::DanneoModule;
use async_trait::async_trait;
use danneo_sdk::registry::{IBlockRegistry, IScriptEngine};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tera::Tera;

pub struct BlockRegistry {
    pub db: Arc<DatabaseConnection>,
    pub script_engine: Arc<dyn IScriptEngine>,
    renderers: Arc<tokio::sync::RwLock<HashMap<&'static str, Box<dyn DanneoBlock>>>>,
}

#[async_trait]
impl danneo_sdk::registry::IBlockRegistry for BlockRegistry {
    async fn render_block(
        &self,
        block_code: &str,
        ctx: Arc<dyn std::any::Any + Send + Sync>,
        settings: Option<Value>,
        tera: &Tera,
    ) -> Option<String> {
        let block_ctx = ctx.downcast::<crate::blocks::BlockContext>().ok()?;
        self.render_block(block_code, block_ctx, settings, tera)
            .await
    }

    async fn get_all_positions_html(
        &self,
        ctx: Arc<dyn std::any::Any + Send + Sync>,
        tera: &Tera,
    ) -> std::collections::HashMap<String, String> {
        if let Ok(block_ctx) = ctx.downcast::<crate::blocks::BlockContext>() {
            self.get_all_positions_html(block_ctx, tera).await
        } else {
            std::collections::HashMap::new()
        }
    }
}

impl BlockRegistry {
    pub fn new(db: Arc<DatabaseConnection>, script_engine: Arc<dyn IScriptEngine>) -> Self {
        let mut renderers: HashMap<&'static str, Box<dyn DanneoBlock>> = HashMap::new();
        // Register native renderers
        let sample = Box::new(crate::blocks::SampleBlock);
        renderers.insert(sample.identifier(), sample);

        Self {
            db,
            script_engine,
            renderers: Arc::new(tokio::sync::RwLock::new(renderers)),
        }
    }

    pub fn register_sync(&self, block: Box<dyn DanneoBlock>) {
        // This is tricky because we can't await in new().
        // But for native ones we can use blocking_write or just handle it differently.
        self.renderers
            .blocking_write()
            .insert(block.identifier(), block);
    }

    pub async fn register(&self, block: Box<dyn DanneoBlock>) {
        self.renderers
            .write()
            .await
            .insert(block.identifier(), block);
    }

    pub async fn init(&self, native_modules: HashMap<String, Arc<dyn DanneoModule>>) {
        use sea_orm::{ActiveModelTrait, Set};

        let renderers_guard = self.renderers.read().await;
        for &identifier in renderers_guard.keys() {
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

        for module in native_modules.values() {
            for definition in module.block_definitions() {
                let exists = core_block_definitions::Entity::find()
                    .filter(core_block_definitions::Column::BlockCode.eq(definition.block_code))
                    .one(self.db.as_ref())
                    .await
                    .unwrap_or(None)
                    .is_some();

                if !exists {
                    let model = core_block_definitions::ActiveModel {
                        block_code: Set(definition.block_code.to_string()),
                        module_code: Set(Some(module.name().to_string())),
                        package_id: Set(0),
                        version: Set(definition.version.to_string()),
                        enabled: Set(true),
                        manifest: Set(serde_json::json!({})),
                        settings_schema: Set(definition.settings_schema),
                        template_path: Set(None),
                        renderer_type: Set("native".to_string()),
                        ..Default::default()
                    };

                    if let Err(e) = model.insert(self.db.as_ref()).await {
                        tracing::error!(
                            "Failed to auto-register native module block {}: {}",
                            definition.block_code,
                            e
                        );
                    } else {
                        tracing::info!(
                            "Auto-registered native module block: {}",
                            definition.block_code
                        );
                    }
                }
            }
        }
    }

    async fn resolve_module_template(
        &self,
        ctx: &BlockContext,
        tera: &tera::Tera,
        module_code: &str,
        block_code: &str,
        template_name: &str,
    ) -> String {
        let settings = ctx.settings.read().await;
        let candidates = [
            template_name.to_string(),
            format!("{}/{}/{}", module_code, settings.site_temp, template_name),
            format!("{}/default/{}", module_code, template_name),
            format!(
                "{}/{}/blocks/{}/{}",
                module_code, settings.site_temp, block_code, template_name
            ),
            format!(
                "{}/default/blocks/{}/{}",
                module_code, block_code, template_name
            ),
            format!("{}/blocks/{}/{}", module_code, block_code, template_name),
        ];

        for candidate in &candidates {
            if tera.get_template_names().any(|n| n == candidate) {
                return candidate.clone();
            }
        }

        format!("{}/default/{}", module_code, template_name)
    }

    fn insert_json_object(context: &mut tera::Context, value: &serde_json::Value) {
        if let Some(obj) = value.as_object() {
            for (key, value) in obj {
                context.insert(key, value);
            }
        }
    }

    async fn render_lua_block_response(
        &self,
        module_code: &str,
        block_code: &str,
        response: serde_json::Value,
        ctx: Arc<BlockContext>,
        settings: Option<serde_json::Value>,
        tera: &tera::Tera,
    ) -> Option<String> {
        if let Some(html) = response.as_str() {
            return Some(html.to_string());
        }

        let res_map = response.as_object()?;
        let template = res_map.get("template").and_then(|v| v.as_str())?;
        let context_val = res_map
            .get("context")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

        let mut context = tera::Context::new();
        {
            let global = ctx.settings.read().await;
            context.insert("site_name", &global.site_name);
            context.insert("site_url", &global.site_url);
            context.insert("site_temp", &global.site_temp);
        }
        context.insert("block_code", block_code);
        if let Some(settings) = settings {
            context.insert("settings", &settings);
        }

        Self::insert_json_object(&mut context, &context_val);

        let full_template_path = self
            .resolve_module_template(&ctx, tera, module_code, block_code, &template)
            .await;
        match tera.render(&full_template_path, &context) {
            Ok(html) => Some(html),
            Err(e) => {
                tracing::error!("Block {} Lua template error: {}", block_code, e);
                Some(format!("Template error: {}", e))
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
                if let Some(module_code) = definition.module_code.as_deref() {
                    let native_modules = ctx.state.modules.get_native_modules().await;
                    if let Some(module) = native_modules.get(module_code) {
                        let ctx_any: Arc<dyn std::any::Any + Send + Sync> = Arc::new(ctx.clone());
                        let settings_any: Arc<dyn std::any::Any + Send + Sync> =
                            Arc::new(ctx.state.settings.clone());
                        if let Some(html) =
                            module.render_block(block_code, ctx_any, settings_any).await
                        {
                            return Some(html);
                        }
                    }
                } else if let Some(renderer) = self.renderers.read().await.get(block_code) {
                    return Some(renderer.render(ctx, settings).await);
                }
            }
            "lua" | "script" => {
                let module_code = definition.module_code.as_ref()?;
                let arg = serde_json::json!({
                    "block_code": block_code,
                    "settings": settings,
                });

                match self
                    .script_engine
                    .call_hook(module_code, "render_block", arg, ctx.state.clone())
                    .await
                {
                    Ok(res) => {
                        return self
                            .render_lua_block_response(
                                module_code,
                                block_code,
                                res,
                                ctx,
                                settings,
                                tera,
                            )
                            .await;
                    }
                    Err(e) => {
                        tracing::error!("Block {} Lua error: {}", block_code, e);
                        return Some(format!("Lua error: {}", e));
                    }
                }
            }
            _ => {
                tracing::warn!(
                    "Unknown renderer type {} for block {}",
                    definition.renderer_type,
                    block_code
                );
            }
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
