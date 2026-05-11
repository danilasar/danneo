use async_trait::async_trait;
use axum::{Router, routing::get};
use base64::{Engine as _, engine::general_purpose};
use danneo_sdk::module::DanneoModule;
use danneo_sdk::register_native_module;
use danneo_sdk::rpc::{RpcContext, RpcError, RpcMethodDescriptor, RpcVisibility};
use danneo_sdk::state::AppState;
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use std::sync::Arc;

pub mod logic;
pub mod migrations;
pub mod routes;

pub struct ImageModule {
    db: Arc<DatabaseConnection>,
}

danneo_sdk::inventory::submit! {
    danneo_sdk::module::migration::ModuleMigrationRegistration { migration: &migrations::CreateImageTable }
}

danneo_sdk::inventory::submit! {
    danneo_sdk::module::migration::ModuleMigrationRegistration { migration: &migrations::UpgradeImageTable }
}

#[derive(serde::Deserialize, Clone)]
pub struct ThumbnailConfig {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub strategy: String,
}

impl ImageModule {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn apply_strategy(
        img: image::DynamicImage,
        config: &ThumbnailConfig,
    ) -> image::DynamicImage {
        match config.strategy.as_str() {
            "crop" => img.resize_to_fill(
                config.width,
                config.height,
                image::imageops::FilterType::Lanczos3,
            ),
            "fill" => img.resize_exact(
                config.width,
                config.height,
                image::imageops::FilterType::Lanczos3,
            ),
            _ => img.resize(
                config.width,
                config.height,
                image::imageops::FilterType::Lanczos3,
            ),
        }
    }
}

#[async_trait]
impl DanneoModule for ImageModule {
    fn name(&self) -> &'static str {
        "image"
    }

    async fn on_install(&self, _state: Arc<AppState>) -> Result<(), String> {
        let state = _state.clone();
        let defaults = [
            ("image_small_width", "150"),
            ("image_small_height", "150"),
            ("image_small_strategy", "\"crop\""),
            ("image_medium_width", "400"),
            ("image_medium_height", "400"),
            ("image_medium_strategy", "\"fit\""),
            ("image_large_width", "1024"),
            ("image_large_height", "768"),
            ("image_large_strategy", "\"fit\""),
        ];

        for (key, val) in defaults {
            state
                .rpc_registry
                .call(
                    "settings",
                    "set",
                    json!({
                        "key": key,
                        "value": serde_json::from_str::<Value>(val).unwrap()
                    }),
                    RpcContext::default(),
                    state.clone(),
                )
                .await
                .ok();
        }

        Ok(())
    }

    fn rpc_methods(&self) -> Vec<RpcMethodDescriptor> {
        vec![
            RpcMethodDescriptor {
                name: "process".to_string(),
                handler: "process".to_string(),
                permission: Some("image.process".to_string()),
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "process_custom".to_string(),
                handler: "process_custom".to_string(),
                permission: Some("image.process".to_string()),
                visibility: RpcVisibility::Internal,
            },
        ]
    }

    async fn call_rpc(
        &self,
        method: &str,
        payload: Value,
        _ctx: RpcContext,
        _state: Arc<AppState>,
    ) -> Result<Value, RpcError> {
        let state = _state.clone();
        match method {
            "process" => {
                let content_b64 = payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing content".to_string()))?;
                let access = payload
                    .get("access")
                    .and_then(|v| v.as_str())
                    .unwrap_or("public");

                let data = general_purpose::STANDARD
                    .decode(content_b64)
                    .map_err(|e| RpcError::BadRequest(e.to_string()))?;

                let presets_fn = || {
                    vec![
                        ThumbnailConfig {
                            name: "small".into(),
                            width: 150,
                            height: 150,
                            strategy: "crop".into(),
                        },
                        ThumbnailConfig {
                            name: "medium".into(),
                            width: 400,
                            height: 400,
                            strategy: "fit".into(),
                        },
                        ThumbnailConfig {
                            name: "large".into(),
                            width: 1024,
                            height: 768,
                            strategy: "fit".into(),
                        },
                    ]
                };

                logic::process_image(self.db.clone(), data, access, None, state, presets_fn)
                    .await
                    .map_err(|e| RpcError::Runtime(e))
            }
            "process_custom" => {
                let content_b64 = payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing content".to_string()))?;
                let access = payload
                    .get("access")
                    .and_then(|v| v.as_str())
                    .unwrap_or("public");
                let presets_val = payload
                    .get("presets")
                    .ok_or_else(|| RpcError::BadRequest("Missing presets".to_string()))?;
                let presets: Vec<ThumbnailConfig> = serde_json::from_value(presets_val.clone())
                    .map_err(|e| RpcError::BadRequest(format!("Invalid presets format: {}", e)))?;

                let data = general_purpose::STANDARD
                    .decode(content_b64)
                    .map_err(|e| RpcError::BadRequest(e.to_string()))?;

                let presets_fn = || -> Vec<ThumbnailConfig> { vec![] };

                logic::process_image(
                    self.db.clone(),
                    data,
                    access,
                    Some(presets),
                    state,
                    presets_fn,
                )
                .await
                .map_err(|e| RpcError::Runtime(e))
            }
            _ => Err(RpcError::NotFound(method.to_string())),
        }
    }

    fn register_routes(&self, state: Arc<AppState>) -> Router<Arc<AppState>> {
        Router::new()
            .route("/v/:id", get(routes::serve_image))
            .route("/t/:id/:size", get(routes::serve_thumb))
    }
}

danneo_sdk::register_native_module!("image", |db| Arc::new(ImageModule::new(db)));

#[cfg(test)]
mod tests {
    use super::*;
    use danneo_core::state::AppState;
    use danneo_sdk::danneotest;

    #[danneotest]
    async fn test_image_init(state: Arc<AppState>) {
        assert!(state.is_module_available("image").await);
    }
}
