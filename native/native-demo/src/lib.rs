use async_trait::async_trait;
use axum::Router;
use danneo_sdk::module::{DanneoModule, NativeBlockDefinition};
use danneo_sdk::state::AppState;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

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

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        let state = _state.clone();
        // Register in Admin Menu via RPC
        state
            .rpc_registry
            .call(
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
                danneo_sdk::rpc::RpcContext::default(),
                state.clone(),
            )
            .await
            .ok();

        Ok(())
    }

    fn register_admin_routes(&self, state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new().route(
            "/info",
            axum::routing::get(|| async { "Native Module Demo Content" }),
        )
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
        _ctx: Arc<dyn std::any::Any + Send + Sync>,
        _settings: Arc<dyn std::any::Any + Send + Sync>,
    ) -> Option<String> {
        if block_code == "native_demo.summary" {
            return Some("<div class='native-block'>Native Title: Summary Block Content from native Rust module</div>".to_string());
        }
        None
    }
}

danneo_sdk::register_native_module!("native_demo", |db| Arc::new(NativeDemoModule::new(db)));

#[cfg(test)]
mod tests {
    use super::*;
    use danneo_core::state::AppState;
    use danneo_sdk::danneotest;

    #[danneotest]
    async fn test_native_demo_init(state: Arc<AppState>) {
        assert!(state.is_module_available("native_demo").await);
    }
}
