use danneo_core::rpc::RpcContext;
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
    migration::Migrator::up(db_arc.as_ref(), None)
        .await
        .unwrap();

    // We need a real AppState because of Arc fields
    let state = danneo_core::state::init_state(db_arc.as_ref().clone())
        .await
        .unwrap();
    let menu_mod = {
        let native_modules = state.modules.get_native_modules().await;
        native_modules.get("admin_menu").unwrap().clone()
    };

    // 2.1 MUST Register modules in core_modules for JOIN to work
    {
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();

        let existing = danneo_core::models::core_modules::Entity::find()
            .filter(danneo_core::models::core_modules::Column::Code.eq("mod_news"))
            .one(db_arc.as_ref())
            .await
            .unwrap();

        if existing.is_none() {
            danneo_core::models::core_modules::ActiveModel {
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
            }
            .insert(db_arc.as_ref())
            .await
            .unwrap();
        }

        let existing_admin = danneo_core::models::core_modules::Entity::find()
            .filter(danneo_core::models::core_modules::Column::Code.eq("admin_menu"))
            .one(db_arc.as_ref())
            .await
            .unwrap();

        if existing_admin.is_none() {
            danneo_core::models::core_modules::ActiveModel {
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
            }
            .insert(db_arc.as_ref())
            .await
            .unwrap();
        }
    }

    let ctx = RpcContext::default();

    // 2. Test ensure_category
    menu_mod.call_rpc("ensure_category", json!({
        "code": "publishing", "parent": "content", "label": "Публикации", "icon": "book", "weight": 50
    }), ctx.clone(), state.clone()).await.unwrap();

    // MUST also ensure 'news' category for move_item to work later
    menu_mod.call_rpc("ensure_category", json!({
        "code": "news", "parent": "content", "label": "Новости", "icon": "news", "weight": 10
    }), ctx.clone(), state.clone()).await.unwrap();

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
    let res = menu_mod
        .call_rpc("get_tree", json!({}), ctx.clone(), state.clone())
        .await;
    assert!(res.is_ok());
    let tree = res.unwrap();

    // Verify our custom category and item exist
    let sections = tree["supercategories"].as_array().unwrap();
    let content_sec = sections
        .iter()
        .find(|s| s["code"] == "content")
        .expect("Content supercategory not found");
    let categories = content_sec["categories"].as_array().unwrap();
    let pub_cat = categories
        .iter()
        .find(|c| c["code"] == "publishing")
        .expect("Publishing category not found");
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
    assert!(res.is_ok(), "move_item failed: {:?}", res);

    // Verify it moved
    let res = menu_mod
        .call_rpc("get_tree", json!({}), ctx.clone(), state.clone())
        .await;
    let tree = res.unwrap();
    let sections = tree["supercategories"].as_array().unwrap();
    let content_sec = sections
        .iter()
        .find(|s| s["code"] == "content")
        .expect("Content supercategory not found after move");
    let news_cat = content_sec["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["code"] == "news")
        .expect("News category not found after move");
    assert!(
        news_cat["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["link"] == "/admin/news")
    );

    // 7. Test get_effective_tree
    let res = menu_mod
        .call_rpc(
            "get_effective_tree",
            json!({
                "admin_id": 1,
                "locale": "ru"
            }),
            ctx.clone(),
            state.clone(),
        )
        .await;
    assert!(res.is_ok(), "get_effective_tree failed: {:?}", res);
    let tree = res.unwrap();
    assert!(tree["supercategories"].as_array().unwrap().len() > 0);

    // 8. Test unregister_module (disable mode)
    // Re-register item first
    menu_mod.call_rpc("register_items", json!({
        "module": "mod_news",
        "items": [{ "code": "list", "category": "news", "label": "List", "link": "/news", "weight": 10 }]
    }), ctx.clone(), state.clone()).await.unwrap();

    let res = menu_mod
        .call_rpc(
            "unregister_module",
            json!({ "module": "mod_news", "mode": "disable" }),
            ctx.clone(),
            state.clone(),
        )
        .await;
    assert!(res.is_ok());

    // Verify it's hidden from effective tree
    let res = menu_mod
        .call_rpc("get_effective_tree", json!({}), ctx.clone(), state.clone())
        .await;
    let tree = res.unwrap();
    let sections = tree["supercategories"].as_array().unwrap();
    let content_sec_opt = sections.iter().find(|s| s["code"] == "content");

    if let Some(sec) = content_sec_opt {
        let categories = sec["categories"].as_array().unwrap();
        let news_cat = categories.iter().find(|c| c["code"] == "news");
        assert!(
            news_cat.is_none(),
            "News category should be pruned when its only item is hidden and module disabled"
        );
    }
}
