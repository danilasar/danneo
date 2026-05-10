use crate::module::DanneoModule;
use crate::state::AppState;
use crate::rpc::{RpcContext, RpcError, RpcMethodDescriptor, RpcVisibility};
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use serde_json::{json, Value};
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use tracing::info;
use base64::{Engine as _, engine::general_purpose};

pub struct StorageModule;

impl StorageModule {
    pub fn new(_db: Arc<DatabaseConnection>) -> Self {
        Self
    }

    async fn get_bucket(&self, state: Arc<AppState>) -> Result<Bucket, String> {
        let settings = state.settings.read().await;
        
        let endpoint = settings.storage_endpoint.clone();
        let access_key = settings.storage_access_key.clone();
        let secret_key = settings.storage_secret_key.clone();
        let bucket_name = settings.storage_bucket.clone();
        let region = settings.storage_region.clone();

        if endpoint.is_empty() || access_key.is_empty() {
            return Err("Storage is not configured".to_string());
        }

        let credentials = Credentials::new(Some(&access_key), Some(&secret_key), None, None, None)
            .map_err(|e| e.to_string())?;
        
        let region_obj = Region::Custom {
            region,
            endpoint,
        };

        let bucket = Bucket::new(&bucket_name, region_obj, credentials)
            .map_err(|e| e.to_string())?
            .with_path_style();

        Ok(*bucket)
    }
}

#[async_trait]
impl DanneoModule for StorageModule {
    fn name(&self) -> &'static str {
        "storage"
    }

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        info!("Storage Native Module initialized");
        Ok(())
    }

    fn rpc_methods(&self) -> Vec<RpcMethodDescriptor> {
        vec![
            RpcMethodDescriptor {
                name: "upload".to_string(),
                handler: "upload".to_string(),
                permission: Some("storage.upload".to_string()),
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "delete".to_string(),
                handler: "delete".to_string(),
                permission: Some("storage.delete".to_string()),
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "get_url".to_string(),
                handler: "get_url".to_string(),
                permission: None,
                visibility: RpcVisibility::Public,
            },
        ]
    }

    async fn call_rpc(
        &self,
        method: &str,
        payload: Value,
        _ctx: RpcContext,
        state: Arc<AppState>,
    ) -> Result<Value, RpcError> {
        let bucket = self.get_bucket(state).await
            .map_err(|e| RpcError::Runtime(format!("Storage config error: {}", e)))?;

        match method {
            "upload" => {
                let path = payload.get("path").and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing 'path'".to_string()))?;
                
                let data = if let Some(file_path) = payload.get("file_path").and_then(|v| v.as_str()) {
                    tokio::fs::read(file_path).await
                        .map_err(|e| RpcError::Runtime(format!("Failed to read temp file: {}", e)))?
                } else if let Some(content_base64) = payload.get("content").and_then(|v| v.as_str()) {
                    general_purpose::STANDARD.decode(content_base64)
                        .map_err(|e| RpcError::BadRequest(format!("Invalid base64: {}", e)))?
                } else {
                    return Err(RpcError::BadRequest("Missing either 'file_path' or 'content'".to_string()));
                };

                bucket.put_object(path, &data).await
                    .map_err(|e| RpcError::Runtime(format!("Upload failed: {}", e)))?;

                Ok(json!({ "status": "success", "path": path }))
            }
            "delete" => {
                let path = payload.get("path").and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing 'path'".to_string()))?;

                bucket.delete_object(path).await
                    .map_err(|e| RpcError::Runtime(format!("Delete failed: {}", e)))?;

                Ok(json!({ "status": "success" }))
            }
            "get_url" => {
                let path = payload.get("path").and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::BadRequest("Missing 'path'".to_string()))?;

                let url = bucket.presign_get(path, 3600, None).await
                    .map_err(|e| RpcError::Runtime(format!("URL generation failed: {}", e)))?;

                Ok(json!({ "url": url }))
            }
            _ => Err(RpcError::NotFound(method.to_string())),
        }
    }
}

crate::register_native_module!("storage", |db| Arc::new(StorageModule::new(db)));
