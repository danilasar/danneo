use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use axum::{Router, response::Html, routing::get};
use std::sync::Arc;

pub struct NativeDemoModule;

#[async_trait]
impl DanneoModule for NativeDemoModule {
    fn name(&self) -> &'static str {
        "native_demo"
    }

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        tracing::info!("Native Demo Module initialized!");
        Ok(())
    }

    fn register_admin_routes(&self) -> Router<Arc<AppState>> {
        Router::new()
            .route("/", get(|| async { Html("<h1>Native Demo Page</h1><p>This page is rendered by a native Rust module.</p>") }))
    }
}
