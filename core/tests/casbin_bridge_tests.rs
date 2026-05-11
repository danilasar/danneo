#[cfg(test)]
mod tests {
    use danneo_core::rpc::RpcContext;
    use danneo_core::state::AppState;
    use sea_orm::Database;
    use serde_json::json;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    use danneo_sdk::danneotest;

    #[danneotest]
    async fn test_casbin_bridge_with_function_registry(state: Arc<AppState>) {
        // 1. Register matchLevel in FunctionRegistry (simulating Security module)
        state
            .function_registry
            .register_dynamic("casbin.matchLevel", |args| {
                Box::pin(async move {
                    let r_level = args[0].as_i64().unwrap_or(0);
                    let p_level = if args[1].is_i64() {
                        args[1].as_i64().unwrap()
                    } else {
                        args[1].as_str().unwrap_or("0").parse::<i64>().unwrap_or(0)
                    };
                    Ok(json!(r_level >= p_level))
                })
            })
            .await;

        // 2. Create a temporary Casbin model that uses matchLevel
        let mut model_file = NamedTempFile::new().unwrap();
        writeln!(
            model_file,
            r#"
[request_definition]
r = sub, obj, act, level
[policy_definition]
p = sub, obj, act, level
[role_definition]
g = _, _
[policy_effect]
e = some(where (p.eft == allow))
[matchers]
m = g(r.sub, p.sub) && r.obj == p.obj && r.act == p.act && matchLevel(r.level, p.level)
        "#
        )
        .unwrap();

        // 3. Load model into Casbin module via RPC
        state
            .rpc_registry
            .call(
                "casbin",
                "load_model",
                json!({
                    "path": model_file.path().to_str().unwrap()
                }),
                RpcContext::default(),
                state.clone(),
            )
            .await
            .unwrap();

        // 4. Add a policy with level requirement
        state
            .rpc_registry
            .call(
                "casbin",
                "add_policy",
                json!({
                    "sub": "alice", "obj": "data", "act": "read", "level": 50
                }),
                RpcContext::default(),
                state.clone(),
            )
            .await
            .unwrap();

        // 5. Test Enforce
        // Should succeed: 100 >= 50
        let allowed = state
            .rpc_registry
            .call(
                "casbin",
                "enforce",
                json!({
                    "sub": "alice", "obj": "data", "act": "read", "level": 100
                }),
                RpcContext::default(),
                state.clone(),
            )
            .await
            .unwrap();
        assert!(allowed.as_bool().unwrap());

        // Should fail: 10 < 50
        let allowed = state
            .rpc_registry
            .call(
                "casbin",
                "enforce",
                json!({
                    "sub": "alice", "obj": "data", "act": "read", "level": 10
                }),
                RpcContext::default(),
                state.clone(),
            )
            .await
            .unwrap();
        assert!(!allowed.as_bool().unwrap());
    }
}
