use danneo_core::state::AppState;
use sea_orm::{Database, EntityTrait, ColumnTrait, QueryFilter};
use serde_json::json;
use std::sync::Arc;
use danneo_core::rpc::{RpcContext, RpcError};

#[tokio::test]
async fn test_rpc_access_to_disabled_module() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();
    let state = Arc::new(AppState::new(db).await.unwrap());

    // 1. Register a fake native module in Bus
    struct FakeModule;
    #[async_trait::async_trait]
    impl danneo_core::module::DanneoModule for FakeModule {
        fn name(&self) -> &'static str { "fake" }
        async fn call_rpc(&self, _m: &str, _p: serde_json::Value, _c: RpcContext, _s: Arc<AppState>) -> Result<serde_json::Value, RpcError> {
            Ok(json!({"status": "ok"}))
        }
    }
    
    let fake = Arc::new(FakeModule);
    state.rpc_registry.register("fake", Arc::new(danneo_core::rpc::registry::NativeRpcHandler::new(fake.clone())), vec![]).await;

    // 2. Call when NOT in DB -> should fail (RpcRegistry check)
    let res = state.rpc_registry.call("fake", "any", json!({}), RpcContext::default(), state.clone()).await;
    assert!(matches!(res, Err(RpcError::NotFound(_))));

    // 3. Add to DB but DISABLED
    use sea_orm::{ActiveModelTrait, Set};
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    danneo_core::models::core_modules::ActiveModel {
        code: Set("fake".to_string()),
        name: Set("Fake".to_string()),
        version: Set("1.0.0".to_string()),
        enabled: Set(false),
        installed: Set(true),
        package_id: Set(99),
        package_path: Set("path".to_string()),
        package_hash: Set("hash".to_string()),
        runtime_type: Set("native".to_string()),
        manifest: Set(json!({})),
        installed_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }.insert(state.db.as_ref()).await.unwrap();

    // 4. Call when DISABLED -> should fail
    let res = state.rpc_registry.call("fake", "any", json!({}), RpcContext::default(), state.clone()).await;
    assert!(matches!(res, Err(RpcError::NotFound(_))));
    if let Err(RpcError::NotFound(msg)) = res {
        assert!(msg.contains("disabled"));
    }

    // 5. Enable and call -> should work
    let model = danneo_core::models::core_modules::Entity::find()
        .filter(danneo_core::models::core_modules::Column::Code.eq("fake"))
        .one(state.db.as_ref()).await.unwrap().unwrap();
        
    let mut active: danneo_core::models::core_modules::ActiveModel = model.into();
    active.enabled = Set(true);
    active.update(state.db.as_ref()).await.unwrap();

    let res = state.rpc_registry.call("fake", "any", json!({}), RpcContext::default(), state.clone()).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_db_isolation_lua_prefixes() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();
    let state = Arc::new(AppState::new(db).await.unwrap());

    // Module A creates table "secrets"
    let script_a = r#"
        function on_install(arg)
            db.create_table({
                table_name = "secrets",
                fields = { { name = "val", field_type = "string" } }
            })
            db.insert("secrets", { val = "hidden" })
        end
    "#;
    state.script_engine.load_script_str("mod_a", script_a).await.unwrap();
    state.script_engine.call_hook("mod_a", "on_install", script_rhai::Dynamic::UNIT, state.clone()).await.unwrap();

    // Module B tries to read "mod_a_secrets" directly
    let script_b = r#"
        function steal(arg)
            -- This should fail because Lua DB API adds "mod_b_" prefix automatically
            return db.select("mod_a_secrets", {"val"})
        end
    "#;
    state.script_engine.load_script_str("mod_b", script_b).await.unwrap();
    
    let res = state.script_engine.call_hook("mod_b", "steal", script_rhai::Dynamic::UNIT, state.clone()).await;
    
    // In current implementation, db.select("mod_a_secrets") will result in table "mod_b_mod_a_secrets"
    // which doesn't exist.
    assert!(res.is_err() || res.unwrap().try_cast::<Vec<script_rhai::Dynamic>>().map(|v| v.is_empty()).unwrap_or(true));
}
