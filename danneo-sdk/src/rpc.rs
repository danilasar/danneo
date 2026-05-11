use crate::module::DanneoModule;
use crate::state::AppState;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum RpcError {
    #[error("RPC Method Not Found: {0}")]
    NotFound(String),
    #[error("RPC Forbidden: {0}")]
    Forbidden(String),
    #[error("RPC Bad Request: {0}")]
    BadRequest(String),
    #[error("RPC Invalid Parameters: {0}")]
    InvalidParams(String),
    #[error("RPC Timeout")]
    Timeout,
    #[error("RPC Runtime Error: {0}")]
    Runtime(String),
    #[error("RPC Max Call Depth Reached")]
    MaxDepthReached,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RpcVisibility {
    Private,  // Только для самого модуля
    Internal, // Для других модулей
    Admin,    // Только для админ-контекста
    Public,   // Доступно через внешний API
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMethodDescriptor {
    pub name: String,
    pub handler: String, // Название Lua-функции или Rust-метода
    pub permission: Option<String>,
    pub visibility: RpcVisibility,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RpcContext {
    pub caller: Option<String>, // ID вызывающего модуля
    pub trace_id: String,
    pub call_depth: u32,
    pub auth_admin_id: Option<i32>,
}

impl Default for RpcContext {
    fn default() -> Self {
        Self {
            caller: None,
            trace_id: uuid::Uuid::new_v4().to_string(),
            call_depth: 0,
            auth_admin_id: None,
        }
    }
}

impl RpcContext {
    pub fn next(&self, caller: String) -> Self {
        Self {
            caller: Some(caller),
            trace_id: self.trace_id.clone(),
            call_depth: self.call_depth + 1,
            auth_admin_id: self.auth_admin_id,
        }
    }
}

#[async_trait]
pub trait RpcHandler: Send + Sync {
    async fn call(
        &self,
        method: &str,
        payload: Value,
        ctx: RpcContext,
        state: Arc<AppState>,
    ) -> Result<Value, RpcError>;
}

pub struct NativeRpcHandler {
    module: Arc<dyn DanneoModule>,
}

impl NativeRpcHandler {
    pub fn new(module: Arc<dyn DanneoModule>) -> Self {
        Self { module }
    }
}

#[async_trait]
impl RpcHandler for NativeRpcHandler {
    async fn call(
        &self,
        method: &str,
        payload: Value,
        ctx: RpcContext,
        state: Arc<AppState>,
    ) -> Result<Value, RpcError> {
        self.module.call_rpc(method, payload, ctx, state).await
    }
}

#[async_trait]
pub trait IRpcRegistry: Send + Sync {
    async fn register(
        &self,
        module_code: &str,
        handler: Arc<dyn RpcHandler>,
        descriptors: Vec<RpcMethodDescriptor>,
    );
    async fn unregister(&self, module_code: &str);
    async fn call(
        &self,
        target_module: &str,
        method: &str,
        payload: Value,
        ctx: RpcContext,
        state: Arc<AppState>,
    ) -> Result<Value, RpcError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rpc_message_serialization() {
        let error = RpcError::NotFound("test_method".to_string());
        let serialized = serde_json::to_string(&error).unwrap();
        let deserialized: RpcError = serde_json::from_str(&serialized).unwrap();
        if let RpcError::NotFound(m) = deserialized {
            assert_eq!(m, "test_method");
        } else {
            panic!("Wrong variant");
        }

        let ctx = RpcContext::default();
        let serialized_ctx = serde_json::to_string(&ctx).unwrap();
        let _deserialized_ctx: RpcContext = serde_json::from_str(&serialized_ctx).unwrap();
    }
}
