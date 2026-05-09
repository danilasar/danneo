use danneo_core::rpc::{RpcContext};
use danneo_core::state::AppState;
use sea_orm::Database;
use std::sync::Arc;
use serde_json::json;

#[tokio::test]
async fn test_mod_news_menu_registration_via_rpc() {
    // 1. Setup
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();
    
    let state = Arc::new(AppState::new(db).await.unwrap());
    
    // 2. Simulate installation of mod_news
    // PackageInstaller will call on_install
    let installer = danneo_core::registry::PackageInstaller::new(
        state.db.clone(),
        state.packages.clone(),
        state.modules.clone(),
        state.routes.clone(),
        state.script_engine.clone(),
        state.clone()
    );

    // We need to load the script first
    let mut hooks_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    hooks_path.pop(); // workspace root
    hooks_path.push("modules/mod_news/scripts/hooks.lua");
    
    let script = std::fs::read_to_string(&hooks_path).expect(&format!("Could not read hooks.lua at {:?}", hooks_path));
    state.script_engine.load_script_str("mod_news", &script).await.unwrap();

    // Call on_install manually for the test
    let res = state.script_engine.call_hook("mod_news", "on_install", script_rhai::Dynamic::UNIT, state.clone()).await;
    assert!(res.is_ok(), "on_install failed: {:?}", res);

    // 3. Verify menu tree via RPC
    let ctx = RpcContext::default();
    let tree_res = state.rpc_registry.call("admin_menu", "get_tree", json!({}), ctx.clone(), state.clone()).await;
    assert!(tree_res.is_ok());
    
    let tree = tree_res.unwrap();
    let sections = tree["supercategories"].as_array().expect("supercategories should be array");
    let content_sec = sections.iter().find(|s| s["code"] == "content").expect("content section not found");
    let news_cat = content_sec["categories"].as_array().unwrap().iter().find(|c| c["code"] == "news").expect("news category not found");
    
    let items = news_cat["items"].as_array().unwrap();
    // Debug print
    println!("Menu Items: {:?}", items);
    
    assert!(items.iter().any(|i| i["label"] == "admin_list" || i["label"] == "Список"));
    assert!(items.iter().any(|i| i["label"] == "admin_add" || i["label"] == "Добавить"));

    // 4. Test uninstallation
    let res = state.script_engine.call_hook("mod_news", "on_uninstall", script_rhai::Dynamic::UNIT, state.clone()).await;
    assert!(res.is_ok(), "on_uninstall failed: {:?}", res);

    let tree_res = state.rpc_registry.call("admin_menu", "get_tree", json!({}), ctx, state.clone()).await;
    let tree = tree_res.unwrap();
    let sections = tree["supercategories"].as_array().unwrap();
    let content_sec = sections.iter().find(|s| s["code"] == "content").unwrap();
    let news_cat = content_sec["categories"].as_array().unwrap().iter().find(|c| c["code"] == "news").expect("Category should remain");
    
    assert_eq!(news_cat["items"].as_array().unwrap().len(), 0, "Items should be removed after uninstall");
}
