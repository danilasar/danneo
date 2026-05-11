#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use serde_json::json;
    use danneo_core::state::AppState;
    use danneo_core::rpc::RpcContext;
    use sea_orm::Database;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_casbin_bridge_with_function_registry() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let state = Arc::new(AppState::new(db).await.unwrap());

        // Enable casbin module in DB
        use sea_orm::{Set, ActiveModelTrait};
        use danneo_core::models::core_modules;
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        core_modules::ActiveModel {
            code: Set("casbin".to_string()),
            name: Set("Casbin".to_string()),
            version: Set("1.0.0".to_string()),
            enabled: Set(true),
            manifest: Set(serde_json::json!({})),
            package_id: Set(0),
            package_path: Set("".to_string()),
            package_hash: Set("".to_string()),
            runtime_type: Set("native".to_string()),
            installed_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }.insert(state.db.as_ref()).await.unwrap();

        // 1. Register matchLevel in FunctionRegistry (simulating Security module)
        state.function_registry.register_dynamic("casbin.matchLevel", |args| {
            Box::pin(async move {
                let r_level = args[0].as_i64().unwrap_or(0);
                let p_level = if args[1].is_i64() {
                    args[1].as_i64().unwrap()
                } else {
                    args[1].as_str().unwrap_or("0").parse::<i64>().unwrap_or(0)
                };
                Ok(json!(r_level >= p_level))
            })
        }).await;

        // 2. Create a temporary Casbin model that uses matchLevel
        let mut model_file = NamedTempFile::new().unwrap();
        writeln!(model_file, r#"
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
        "#).unwrap();

        // 3. Load model into Casbin module via RPC
        state.rpc_registry.call("casbin", "load_model", json!({
            "path": model_file.path().to_str().unwrap()
        }), RpcContext::default(), state.clone()).await.unwrap();

        // 4. Add a policy with level requirement
        state.rpc_registry.call("casbin", "add_policy", json!({
            "sub": "alice", "obj": "data", "act": "read", "level": 50
        }), RpcContext::default(), state.clone()).await.unwrap();

        // 5. Test Enforce
        // Should succeed: 100 >= 50
        let allowed = state.rpc_registry.call("casbin", "enforce", json!({
            "sub": "alice", "obj": "data", "act": "read", "level": 100
        }), RpcContext::default(), state.clone()).await.unwrap();
        assert!(allowed.as_bool().unwrap());

        // Should fail: 10 < 50
        let allowed = state.rpc_registry.call("casbin", "enforce", json!({
            "sub": "alice", "obj": "data", "act": "read", "level": 10
        }), RpcContext::default(), state.clone()).await.unwrap();
        assert!(!allowed.as_bool().unwrap());
    }
}
