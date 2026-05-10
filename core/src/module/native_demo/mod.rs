use crate::module::{DanneoModule, NativeBlockDefinition};
use crate::state::AppState;
use async_trait::async_trait;
use axum::Router;
use std::sync::Arc;
use sea_orm::DatabaseConnection;

pub struct NativeDemoModule;

impl NativeDemoModule {
    pub fn new(_db: Arc<DatabaseConnection>) -> Self {
        Self
    }
}

#[async_trait]
impl DanneoModule for NativeDemoModule {
    fn name(&self) -> &'static str {
        "native_demo"
    }

    async fn init(&self, state: Arc<AppState>) -> Result<(), String> {
        // Register in Admin Menu via RPC
        state.rpc_registry.call(
            "admin_menu",
            "register_items",
            serde_json::json!({
                "module": "native_demo",
                "items": [
                    {
                        "code": "demo",
                        "category": "tools",
                        "label": "Native Demo",
                        "link": "/admin/m/native_demo/info",
                        "weight": 100
                    }
                ]
            }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.ok();

        Ok(())
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new().route("/info", axum::routing::get(|| async { "Native Module Demo Content" }))
    }

    fn block_definitions(&self) -> Vec<NativeBlockDefinition> {
        vec![NativeBlockDefinition {
            block_code: "native_demo.summary",
            version: "1.0.0",
            settings_schema: None,
        }]
    }

    async fn render_block(
        &self,
        block_code: &str,
        _ctx: Arc<crate::blocks::BlockContext>,
        _settings: Arc<tokio::sync::RwLock<crate::state::GlobalSettings>>,
    ) -> Option<String> {
        if block_code == "native_demo.summary" {
            return Some("<div class='native-block'>Native Title: Summary Block Content from native Rust module</div>".to_string());
        }
        None
    }
}

crate::register_native_module!("native_demo", |db| Arc::new(NativeDemoModule::new(db)));
