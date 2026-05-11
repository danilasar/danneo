use async_trait::async_trait;
use danneo_sdk::rpc::{IRpcRegistry, RpcContext, RpcError, RpcHandler, RpcMethodDescriptor};
use danneo_sdk::state::AppState;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RpcRegistry {
    methods: RwLock<HashMap<String, Arc<dyn RpcHandler>>>,
    descriptors: RwLock<HashMap<String, Vec<RpcMethodDescriptor>>>,
    max_depth: u32,
}

impl RpcRegistry {
    pub fn new() -> Self {
        Self {
            methods: RwLock::new(HashMap::new()),
            descriptors: RwLock::new(HashMap::new()),
            max_depth: 8,
        }
    }
}

#[async_trait]
impl IRpcRegistry for RpcRegistry {
    async fn register(
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

    async fn unregister(&self, module_code: &str) {
        self.methods.write().await.remove(module_code);
        self.descriptors.write().await.remove(module_code);
    }

    async fn call(
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

        // 1. Check if module is available and enabled
        if !matches!(target_module, "admin_menu" | "settings" | "casbin")
            && !state.modules.is_available(target_module).await
        {
            return Err(RpcError::NotFound(format!(
                "Module {} is disabled or not found",
                target_module
            )));
        }

        let handler = {
            let methods = self.methods.read().await;
            methods.get(target_module).cloned()
        };

        if let Some(h) = handler {
            h.call(method, payload, ctx, state).await
        } else {
            Err(RpcError::NotFound(format!(
                "Module {} handler not registered",
                target_module
            )))
        }
    }
}
