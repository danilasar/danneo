use async_trait::async_trait;
use casbin::function_map::OperatorFunction;
use casbin::rhai::Dynamic;
use casbin::{CoreApi, DefaultModel, Enforcer, MgmtApi, RbacApi};
use danneo_sdk::module::DanneoModule;
use danneo_sdk::register_native_module;
use danneo_sdk::state::AppState;
use once_cell::sync::OnceCell;
use sea_orm::DatabaseConnection;
use sea_orm_adapter::SeaOrmAdapter;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::RwLock;

static GLOBAL_REGISTRY: OnceCell<Arc<danneo_sdk::functions::FunctionRegistry>> = OnceCell::new();

pub struct CasbinModule {
    db: Arc<DatabaseConnection>,
    enforcer: Arc<RwLock<Option<Enforcer>>>,
}

impl CasbinModule {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            enforcer: Arc::new(RwLock::new(None)),
        }
    }
}

// Special case for matchLevel as it's common and has a fixed Arg2 signature
fn match_level_bridge(a: Dynamic, b: Dynamic) -> Dynamic {
    let registry = GLOBAL_REGISTRY
        .get()
        .expect("Global FunctionRegistry not initialized");

    // Convert Dynamic to Value for registry
    let a_val = if a.is_int() {
        json!(a.as_int().unwrap())
    } else {
        json!(a.to_string())
    };
    let b_val = if b.is_int() {
        json!(b.as_int().unwrap())
    } else {
        json!(b.to_string())
    };

    let args = json!([a_val, b_val]);
    let result =
        futures::executor::block_on(async { registry.call("casbin.matchLevel", args).await });

    match result {
        Ok(v) => Dynamic::from(v.as_bool().unwrap_or(false)),
        Err(_) => Dynamic::from(false),
    }
}

#[async_trait]
impl DanneoModule for CasbinModule {
    fn name(&self) -> &'static str {
        "casbin"
    }

    async fn init(&self, _state: Arc<AppState>) -> Result<(), String> {
        let state = _state.clone();
        let _ = GLOBAL_REGISTRY.set(state.function_registry.clone());
        tracing::info!("Casbin Native Module initialized (naked with bridge)");
        Ok(())
    }

    fn rpc_methods(&self) -> Vec<danneo_sdk::rpc::RpcMethodDescriptor> {
        use danneo_sdk::rpc::{RpcMethodDescriptor, RpcVisibility};
        vec![
            RpcMethodDescriptor {
                name: "load_model".to_string(),
                handler: "load_model".to_string(),
                permission: Some("admin.manage".to_string()),
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "enforce".to_string(),
                handler: "enforce".to_string(),
                permission: None,
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "add_policy".to_string(),
                handler: "add_policy".to_string(),
                permission: Some("admin.manage".to_string()),
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "add_role_for_user".to_string(),
                handler: "add_role_for_user".to_string(),
                permission: Some("admin.manage".to_string()),
                visibility: RpcVisibility::Internal,
            },
            RpcMethodDescriptor {
                name: "remove_filtered_policy".to_string(),
                handler: "remove_filtered_policy".to_string(),
                permission: Some("admin.manage".to_string()),
                visibility: RpcVisibility::Internal,
            },
        ]
    }

    async fn call_rpc(
        &self,
        method: &str,
        payload: Value,
        _ctx: danneo_sdk::rpc::RpcContext,
        _state: Arc<AppState>,
    ) -> Result<Value, danneo_sdk::rpc::RpcError> {
        match method {
            "load_model" => {
                let path =
                    payload["path"]
                        .as_str()
                        .ok_or(danneo_sdk::rpc::RpcError::BadRequest(
                            "path missing".to_string(),
                        ))?;
                let model = DefaultModel::from_file(path)
                    .await
                    .map_err(|e| danneo_sdk::rpc::RpcError::Runtime(e.to_string()))?;
                let adapter = SeaOrmAdapter::new((*self.db).clone())
                    .await
                    .map_err(|e| danneo_sdk::rpc::RpcError::Runtime(e.to_string()))?;
                let mut e = Enforcer::new(model, adapter)
                    .await
                    .map_err(|e| danneo_sdk::rpc::RpcError::Runtime(e.to_string()))?;

                // Register the bridge function
                e.add_function("matchLevel", OperatorFunction::Arg2(match_level_bridge));

                let mut guard = self.enforcer.write().await;
                *guard = Some(e);
                Ok(json!(true))
            }
            "enforce" => {
                let guard = self.enforcer.read().await;
                let e = guard.as_ref().ok_or(danneo_sdk::rpc::RpcError::Runtime(
                    "Enforcer not initialized".to_string(),
                ))?;

                let sub = payload["sub"]
                    .as_str()
                    .ok_or(danneo_sdk::rpc::RpcError::BadRequest(
                        "sub missing".to_string(),
                    ))?;
                let obj = payload["obj"]
                    .as_str()
                    .ok_or(danneo_sdk::rpc::RpcError::BadRequest(
                        "obj missing".to_string(),
                    ))?;
                let act = payload["act"]
                    .as_str()
                    .ok_or(danneo_sdk::rpc::RpcError::BadRequest(
                        "act missing".to_string(),
                    ))?;

                if let Some(level) = payload.get("level") {
                    let level_i64 = level.as_i64().unwrap_or(0);
                    let allowed = e
                        .enforce((sub, obj, act, level_i64))
                        .map_err(|e| danneo_sdk::rpc::RpcError::Runtime(e.to_string()))?;
                    Ok(json!(allowed))
                } else {
                    let allowed = e
                        .enforce((sub, obj, act))
                        .map_err(|e| danneo_sdk::rpc::RpcError::Runtime(e.to_string()))?;
                    Ok(json!(allowed))
                }
            }
            "add_policy" => {
                let mut guard = self.enforcer.write().await;
                let e = guard.as_mut().ok_or(danneo_sdk::rpc::RpcError::Runtime(
                    "Enforcer not initialized".to_string(),
                ))?;

                let sub = payload["sub"]
                    .as_str()
                    .ok_or(danneo_sdk::rpc::RpcError::BadRequest(
                        "sub missing".to_string(),
                    ))?;
                let obj = payload["obj"]
                    .as_str()
                    .ok_or(danneo_sdk::rpc::RpcError::BadRequest(
                        "obj missing".to_string(),
                    ))?;
                let act = payload["act"]
                    .as_str()
                    .ok_or(danneo_sdk::rpc::RpcError::BadRequest(
                        "act missing".to_string(),
                    ))?;

                let mut policy = vec![sub.to_string(), obj.to_string(), act.to_string()];
                if let Some(level) = payload.get("level") {
                    policy.push(level.to_string().replace('"', ""));
                }

                let ok = e
                    .add_policy(policy)
                    .await
                    .map_err(|e| danneo_sdk::rpc::RpcError::Runtime(e.to_string()))?;
                Ok(json!(ok))
            }
            "add_role_for_user" => {
                let mut guard = self.enforcer.write().await;
                let e = guard.as_mut().ok_or(danneo_sdk::rpc::RpcError::Runtime(
                    "Enforcer not initialized".to_string(),
                ))?;

                let user =
                    payload["user"]
                        .as_str()
                        .ok_or(danneo_sdk::rpc::RpcError::BadRequest(
                            "user missing".to_string(),
                        ))?;
                let role =
                    payload["role"]
                        .as_str()
                        .ok_or(danneo_sdk::rpc::RpcError::BadRequest(
                            "role missing".to_string(),
                        ))?;

                let ok = e
                    .add_role_for_user(user, role, None)
                    .await
                    .map_err(|e| danneo_sdk::rpc::RpcError::Runtime(e.to_string()))?;
                Ok(json!(ok))
            }
            "remove_filtered_policy" => {
                let mut guard = self.enforcer.write().await;
                let e = guard.as_mut().ok_or(danneo_sdk::rpc::RpcError::Runtime(
                    "Enforcer not initialized".to_string(),
                ))?;

                let index = payload["index"].as_u64().unwrap_or(0) as usize;
                let value =
                    payload["value"]
                        .as_str()
                        .ok_or(danneo_sdk::rpc::RpcError::BadRequest(
                            "value missing".to_string(),
                        ))?;

                let ok = e
                    .remove_filtered_policy(index, vec![value.to_string()])
                    .await
                    .map_err(|e| danneo_sdk::rpc::RpcError::Runtime(e.to_string()))?;
                Ok(json!(ok))
            }
            _ => Err(danneo_sdk::rpc::RpcError::NotFound(method.to_string())),
        }
    }
}

danneo_sdk::register_native_module!("casbin", |db| Arc::new(CasbinModule::new(db)));

#[cfg(test)]
mod tests {
    use super::*;
    use danneo_core::state::AppState;
    use danneo_sdk::danneotest;

    #[danneotest]
    async fn test_casbin_init(state: Arc<AppState>) {
        assert!(state.is_module_available("casbin").await);
    }
}
