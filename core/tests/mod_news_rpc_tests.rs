use danneo_core::rpc::RpcContext;
use danneo_core::state::AppState;
use sea_orm::{Database, ConnectionTrait, Statement, EntityTrait, ColumnTrait, QueryFilter};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_mod_news_menu_registration_via_rpc() {
    // 1. Setup isolated SQLite in-memory
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();

    let state = Arc::new(AppState::new(db).await.unwrap());

    // 1.1 Ensure modules are registered and enabled for JOIN to work
    {
        use sea_orm::{ActiveModelTrait, Set};
        let backend = state.db.get_database_backend();
        
        // Ensure supercategories exist
        let _ = state.db.execute(Statement::from_string(backend, 
            "INSERT OR IGNORE INTO core_admin_menu_supercategories (code, label_key, weight) VALUES ('content', 'admin_content', 10)")).await;
        let _ = state.db.execute(Statement::from_string(backend, 
            "INSERT OR IGNORE INTO core_admin_menu_supercategories (code, label_key, weight) VALUES ('system', 'admin_system', 20)")).await;

        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        
        // Safely register admin_menu if not present
        let exists_adm = danneo_core::models::core_modules::Entity::find()
            .filter(danneo_core::models::core_modules::Column::Code.eq("admin_menu"))
            .one(state.db.as_ref()).await.unwrap();

        if exists_adm.is_none() {
            let adm = danneo_core::models::core_modules::ActiveModel {
                code: Set("admin_menu".to_string()),
                name: Set("Admin Menu".to_string()),
                version: Set("kernel".to_string()),
                package_id: Set(0),
                package_path: Set("kernel".to_string()),
                package_hash: Set("kernel".to_string()),
                runtime_type: Set("native".to_string()),
                enabled: Set(true),
                installed: Set(true),
                manifest: Set(json!({})),
                installed_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };
            let _ = adm.insert(state.db.as_ref()).await.unwrap();
        }

        // Safely register mod_news if not present
        let exists_news = danneo_core::models::core_modules::Entity::find()
            .filter(danneo_core::models::core_modules::Column::Code.eq("mod_news"))
            .one(state.db.as_ref()).await.unwrap();

        if exists_news.is_none() {
            let news = danneo_core::models::core_modules::ActiveModel {
                code: Set("mod_news".to_string()),
                name: Set("News".to_string()),
                version: Set("1.0.0".to_string()),
                package_id: Set(1),
                package_path: Set("modules/mod_news".to_string()),
                package_hash: Set("hash".to_string()),
                runtime_type: Set("lua".to_string()),
                enabled: Set(true),
                installed: Set(true),
                manifest: Set(json!({})),
                installed_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };
            let _ = news.insert(state.db.as_ref()).await.unwrap();
        }
    }

    // 2. Load mod_news hooks script
    let mut hooks_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    hooks_path.pop(); // workspace root
    hooks_path.push("modules/mod_news/scripts/hooks.lua");

    let script = std::fs::read_to_string(&hooks_path)
        .expect(&format!("Could not read hooks.lua at {:?}", hooks_path));
    state
        .script_engine
        .load_script_str("mod_news", &script)
        .await
        .unwrap();

    // 3. Trigger on_install which registers category and items via RPC
    let res = state
        .script_engine
        .call_hook(
            "mod_news",
            "on_install",
            script_rhai::Dynamic::UNIT,
            state.clone(),
        )
        .await;
    assert!(res.is_ok(), "on_install failed: {:?}", res);

    // 4. Verify menu tree
    let tree_res = state
        .rpc_registry
        .call("admin_menu", "get_tree", json!({}), RpcContext::default(), state.clone())
        .await;
    
    assert!(tree_res.is_ok(), "get_tree RPC failed");
    let tree = tree_res.unwrap();
    
    let sections = tree["supercategories"].as_array().expect("supercategories should be array");
    let content_sec = sections.iter().find(|s| s["code"] == "content")
        .expect("content section not found (check pruning logic and JOIN)");
    
    let news_cat = content_sec["categories"].as_array().unwrap()
        .iter().find(|c| c["code"] == "news")
        .expect("news category not found");

    let items = news_cat["items"].as_array().unwrap();
    assert!(!items.is_empty(), "Menu items for news should not be empty");

    // 5. Test uninstallation
    let _ = state.script_engine.call_hook("mod_news", "on_uninstall", script_rhai::Dynamic::UNIT, state.clone()).await;

    let tree_res = state.rpc_registry.call("admin_menu", "get_tree", json!({}), RpcContext::default(), state.clone()).await;
    let tree = tree_res.unwrap();
    let sections = tree["supercategories"].as_array().unwrap();
    let content_sec_opt = sections.iter().find(|s| s["code"] == "content");

    if let Some(sec) = content_sec_opt {
        let news_cat_opt = sec["categories"].as_array().unwrap().iter().find(|c| c["code"] == "news");
        if let Some(news) = news_cat_opt {
             assert_eq!(news["items"].as_array().unwrap().len(), 0, "Items should be removed");
        }
    }
}
