use danneo_core::module::DanneoModule;
use danneo_core::module::admin_menu::AdminMenuModule;
use danneo_core::rpc::{RpcContext, RpcError, RpcMethodDescriptor, RpcVisibility};
use sea_orm::Database;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_admin_menu_rpc_scenarios() {
    // 1. Setup isolated DB and Module
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let db_arc = Arc::new(db);
    
    // Run migrations
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(db_arc.as_ref(), None).await.unwrap();

    // We need a real AppState because of Arc fields
    let state = Arc::new(danneo_core::state::AppState::new(db_arc.as_ref().clone()).await.unwrap());
    let menu_mod = state.admin_menu.clone();
    
    // Module is already initialized by AppState::new, but let's re-init for clean slate if needed
    // (Actually AppState::new calls init, which creates tables)

    let ctx = RpcContext::default();

    // 2. Test ensure_category
    let res = menu_mod
        .call_rpc(
            "ensure_category",
            json!({
                "code": "publishing",
                "parent": "content",
                "label": "Публикации",
                "icon": "book",
                "weight": 50
            }),
            ctx.clone(),
            state.clone(),
        )
        .await;
    assert!(res.is_ok(), "ensure_category failed: {:?}", res);

    // 3. Test register_items
    let res = menu_mod
        .call_rpc(
            "register_items",
            json!({
                "module": "mod_news",
                "items": [
                    {
                        "code": "list",
                        "category": "publishing",
                        "label": "Все новости",
                        "link": "/admin/news",
                        "weight": 10
                    }
                ]
            }),
            ctx.clone(),
            state.clone(),
        )
        .await;
    assert!(res.is_ok(), "register_items failed: {:?}", res);

    // 4. Test get_tree
    let res = menu_mod.call_rpc("get_tree", json!({}), ctx.clone(), state.clone()).await;
    assert!(res.is_ok());
    let tree = res.unwrap();

    // Verify our custom category and item exist
    let sections = tree["supercategories"].as_array().unwrap();
    let content_sec = sections.iter().find(|s| s["code"] == "content").unwrap();
    let categories = content_sec["categories"].as_array().unwrap();
    let pub_cat = categories
        .iter()
        .find(|c| c["code"] == "publishing")
        .unwrap();
    assert_eq!(pub_cat["label"], "Публикации");

    let items = pub_cat["items"].as_array().unwrap();
    assert_eq!(items[0]["label"], "Все новости");

    // 5. Test move_item
    let res = menu_mod
        .call_rpc(
            "move_item",
            json!({
                "item": "mod_news.list",
                "category": "news",
                "weight": 99
            }),
            ctx.clone(),
            state.clone(),
        )
        .await;
    assert!(res.is_ok());

    // Verify it moved
    let res = menu_mod.call_rpc("get_tree", json!({}), ctx.clone(), state.clone()).await;
    let tree = res.unwrap();
    let sections = tree["supercategories"].as_array().unwrap();
    let content_sec = sections.iter().find(|s| s["code"] == "content").unwrap();
    let news_cat = content_sec["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == "news")
        .unwrap();
    assert!(
        news_cat["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["link"] == "/admin/news")
    );

    // 7. Test get_effective_tree
    let res = menu_mod.call_rpc("get_effective_tree", json!({
        "admin_id": 1,
        "locale": "ru"
    }), ctx.clone(), state.clone()).await;
    assert!(res.is_ok(), "get_effective_tree failed: {:?}", res);
    let tree = res.unwrap();
    assert!(tree["supercategories"].as_array().unwrap().len() > 0);

    // 8. Test unregister_module (disable mode)
    // Re-register item first
    menu_mod.call_rpc("register_items", json!({
        "module": "mod_news",
        "items": [{ "code": "list", "category": "news", "label": "List", "link": "/news", "weight": 10 }]
    }), ctx.clone(), state.clone()).await.unwrap();

    let res = menu_mod.call_rpc("unregister_module", json!({ "module": "mod_news", "mode": "disable" }), ctx.clone(), state.clone()).await;
    assert!(res.is_ok());

    // Verify it's hidden from effective tree
    let res = menu_mod.call_rpc("get_effective_tree", json!({}), ctx.clone(), state.clone()).await;
    let tree = res.unwrap();
    let sections = tree["supercategories"].as_array().unwrap();
    let content_sec = sections.iter().find(|s| s["code"] == "content").unwrap();
    let news_cat_opt = content_sec["categories"].as_array().unwrap().iter().find(|c| c["code"] == "news");
    
    // Since we filtered out empty categories in get_effective_tree, it might be gone or empty
    assert!(news_cat_opt.is_none() || news_cat_opt.unwrap()["items"].as_array().unwrap().is_empty());
}
