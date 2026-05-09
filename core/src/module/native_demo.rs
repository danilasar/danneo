use crate::module::{DanneoModule, NativeBlockDefinition};
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, response::Html, routing::get};
use serde_json::Value;
use std::sync::Arc;

pub struct NativeDemoModule;

#[async_trait]
impl DanneoModule for NativeDemoModule {
    fn name(&self) -> &'static str {
        "native_demo"
    }

    async fn on_install(&self, state: Arc<AppState>) -> Result<(), String> {
        state.rpc_registry.call(
            "admin_menu",
            "register_items",
            serde_json::json!({
                "module": "native_demo",
                "items": [
                    {
                        "code": "index",
                        "category": "system",
                        "label": "Native Demo",
                        "link": "/admin/m/native_demo/",
                        "weight": 999
                    }
                ]
            }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.map(|_| ()).map_err(|e| e.to_string())
    }

    async fn on_uninstall(&self, state: Arc<AppState>) -> Result<(), String> {
        state.rpc_registry.call(
            "admin_menu",
            "unregister_module",
            serde_json::json!({
                "module": "native_demo",
                "mode": "remove"
            }),
            crate::rpc::RpcContext::default(),
            state.clone()
        ).await.map(|_| ()).map_err(|e| e.to_string())
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(|| async { Html("<h1>Native Demo Page</h1><p>This page is rendered by a native Rust module.</p>") }))
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
        settings: Option<Value>,
    ) -> Option<String> {
        match block_code {
            "native_demo.summary" => {
                let title = settings
                    .as_ref()
                    .and_then(|v| v.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Native Demo");
                Some(format!(
                    "<div class=\"native-demo-block\"><strong>{}</strong><p>This block is rendered by a native Rust module.</p></div>",
                    title
                ))
            }
            _ => None,
        }
    }
}
