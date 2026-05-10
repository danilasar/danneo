use async_trait::async_trait;
use danneo_core::rpc::registry::{RpcHandler, RpcRegistry};
use danneo_core::rpc::{RpcContext, RpcError, RpcMethodDescriptor, RpcVisibility};
use danneo_core::state::AppState;
use serde_json::json;
use std::sync::Arc;

mod common;

struct MockNativeHandler;

#[async_trait]
impl RpcHandler for MockNativeHandler {
    async fn call(
        &self,
        method: &str,
        payload: serde_json::Value,
        _ctx: RpcContext,
        _state: Arc<AppState>,
    ) -> Result<serde_json::Value, RpcError> {
        match method {
            "hello" => {
                let name = payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("World");
                Ok(json!({ "message": format!("Hello, {}!", name) }))
            }
            "recursive" => {
                // This will be handled by the registry test
                Ok(json!({ "status": "ok" }))
            }
            _ => Err(RpcError::NotFound(method.to_string())),
        }
    }
}

#[tokio::test]
async fn test_rpc_native_call() {
    let state = common::create_test_state().await;
    let registry = RpcRegistry::new();
    let handler = Arc::new(MockNativeHandler);

    registry
        .register(
            "test_mod",
            handler,
            vec![RpcMethodDescriptor {
                name: "hello".to_string(),
                handler: "hello".to_string(),
                permission: None,
                visibility: RpcVisibility::Internal,
            }],
        )
        .await;

    let res = registry
        .call(
            "test_mod",
            "hello",
            json!({ "name": "Rust" }),
            RpcContext::default(),
            state.clone(),
        )
        .await;

    assert!(res.is_ok());
    assert_eq!(res.unwrap()["message"], "Hello, Rust!");
}

#[tokio::test]
async fn test_rpc_not_found() {
    let state = common::create_test_state().await;
    let registry = RpcRegistry::new();
    let res = registry
        .call(
            "non_existent",
            "any",
            json!({}),
            RpcContext::default(),
            state.clone(),
        )
        .await;

    match res {
        Err(RpcError::NotFound(_)) => {}
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_rpc_max_depth() {
    let state = common::create_test_state().await;
    let registry = Arc::new(RpcRegistry::new());

    struct RecursiveHandler {
        reg: Arc<RpcRegistry>,
    }

    #[async_trait]
    impl RpcHandler for RecursiveHandler {
        async fn call(
            &self,
            _method: &str,
            _payload: serde_json::Value,
            ctx: RpcContext,
            state: Arc<AppState>,
        ) -> Result<serde_json::Value, RpcError> {
            self.reg
                .call(
                    "rec_mod",
                    "loop",
                    json!({}),
                    ctx.next("rec_mod".to_string()),
                    state,
                )
                .await
        }
    }

    let handler = Arc::new(RecursiveHandler {
        reg: registry.clone(),
    });
    registry.register("rec_mod", handler, vec![]).await;

    let res = registry
        .call(
            "rec_mod",
            "loop",
            json!({}),
            RpcContext::default(),
            state.clone(),
        )
        .await;

    match res {
        Err(RpcError::MaxDepthReached) => {}
        _ => panic!("Expected MaxDepthReached error, got {:?}", res),
    }
}
