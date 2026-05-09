use crate::rpc::{RpcContext, RpcError, RpcMethodDescriptor};
use crate::state::AppState;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

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
    module: Arc<dyn crate::module::DanneoModule>,
}

impl NativeRpcHandler {
    pub fn new(module: Arc<dyn crate::module::DanneoModule>) -> Self {
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

pub struct RpcRegistry {
    methods: tokio::sync::RwLock<HashMap<String, Arc<dyn RpcHandler>>>,
    descriptors: tokio::sync::RwLock<HashMap<String, Vec<RpcMethodDescriptor>>>,
    max_depth: u32,
}

impl RpcRegistry {
    pub fn new() -> Self {
        Self {
            methods: tokio::sync::RwLock::new(HashMap::new()),
            descriptors: tokio::sync::RwLock::new(HashMap::new()),
            max_depth: 8,
        }
    }

    pub async fn register(
        &self,
        module_code: &str,
        handler: Arc<dyn RpcHandler>,
        descriptors: Vec<RpcMethodDescriptor>,
    ) {
        self.methods
            .write()
            .await
            .insert(module_code.to_string(), handler);
        self.descriptors
            .write()
            .await
            .insert(module_code.to_string(), descriptors);
    }

    pub async fn unregister(&self, module_code: &str) {
        self.methods.write().await.remove(module_code);
        self.descriptors.write().await.remove(module_code);
    }

    pub async fn call(
        &self,
        target_module: &str,
        method: &str,
        payload: Value,
        ctx: RpcContext,
        state: Arc<AppState>,
    ) -> Result<Value, RpcError> {
        if ctx.call_depth >= self.max_depth {
            return Err(RpcError::MaxDepthReached);
        }

        let handler = {
            let methods = self.methods.read().await;
            methods.get(target_module).cloned()
        };

        if let Some(h) = handler {
            // TODO: ACL and visibility check
            h.call(method, payload, ctx, state).await
        } else {
            Err(RpcError::NotFound(format!(
                "Module {} not found",
                target_module
            )))
        }
    }
}
