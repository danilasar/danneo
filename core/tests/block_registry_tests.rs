use danneo_core::blocks::BlockContext;
use danneo_core::models::core_block_definitions;
use danneo_core::registry::{BlockRegistry, ScriptEngine};
use sea_orm::{ActiveModelTrait, Database, Set};
use std::sync::Arc;

async fn test_context() -> (
    Arc<sea_orm::DatabaseConnection>,
    Arc<BlockContext>,
    tera::Tera,
) {
    let db = Arc::new(Database::connect("sqlite::memory:").await.unwrap());
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(db.as_ref(), None).await.unwrap();

    let settings = Arc::new(tokio::sync::RwLock::new(
        danneo_core::state::GlobalSettings {
            site_name: "Test Site".to_string(),
            site_url: "https://example.test".to_string(),
            site_temp: "default".to_string(),
            ..Default::default()
        },
    ));

    let ctx = Arc::new(BlockContext {
        db: db.clone(),
        settings,
    });

    (db, ctx, tera::Tera::default())
}

#[tokio::test]
async fn lua_block_can_return_html() {
    let (db, ctx, tera) = test_context().await;
    let script_engine = Arc::new(ScriptEngine::new(db.clone()));
    let registry = BlockRegistry::new(db.clone(), script_engine.clone());

    script_engine
        .load_script_str(
            "mod_blocks",
            r#"
            function render_block(arg)
                return "<aside>" .. arg.block_code .. ":" .. arg.settings.title .. "</aside>"
            end
        "#,
        )
        .await
        .unwrap();

    core_block_definitions::ActiveModel {
        block_code: Set("lua_html_block".to_string()),
        module_code: Set(Some("mod_blocks".to_string())),
        package_id: Set(0),
        version: Set("1.0.0".to_string()),
        enabled: Set(true),
        manifest: Set(serde_json::json!({})),
        settings_schema: Set(None),
        template_path: Set(None),
        renderer_type: Set("lua".to_string()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let html = registry
        .render_block(
            "lua_html_block",
            ctx,
            Some(serde_json::json!({ "title": "Hello" })),
            &tera,
        )
        .await
        .unwrap();

    assert_eq!(html, "<aside>lua_html_block:Hello</aside>");
}

#[tokio::test]
async fn lua_block_can_return_template_response_like_routes() {
    let (db, ctx, mut tera) = test_context().await;
    let script_engine = Arc::new(ScriptEngine::new(db.clone()));
    let registry = BlockRegistry::new(db.clone(), script_engine.clone());

    tera.add_raw_template(
        "mod_blocks/default/block.html",
        "<section>{{ site_name }}:{{ message }}:{{ settings.limit }}</section>",
    )
    .unwrap();

    script_engine
        .load_script_str(
            "mod_blocks",
            r#"
            function render_block(arg)
                return {
                    template = "block.html",
                    context = { message = "Lua block" }
                }
            end
        "#,
        )
        .await
        .unwrap();

    core_block_definitions::ActiveModel {
        block_code: Set("lua_template_block".to_string()),
        module_code: Set(Some("mod_blocks".to_string())),
        package_id: Set(0),
        version: Set("1.0.0".to_string()),
        enabled: Set(true),
        manifest: Set(serde_json::json!({})),
        settings_schema: Set(None),
        template_path: Set(None),
        renderer_type: Set("script".to_string()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    let html = registry
        .render_block(
            "lua_template_block",
            ctx,
            Some(serde_json::json!({ "limit": 3 })),
            &tera,
        )
        .await
        .unwrap();

    assert_eq!(html, "<section>Test Site:Lua block:3</section>");
}

#[tokio::test]
async fn native_module_block_renders_through_same_registry() {
    let (db, ctx, tera) = test_context().await;
    let script_engine = Arc::new(ScriptEngine::new(db.clone()));
    let registry = BlockRegistry::new(db.clone(), script_engine);
    registry.init().await;

    let html = registry
        .render_block(
            "native_demo.summary",
            ctx,
            Some(serde_json::json!({ "title": "Native Title" })),
            &tera,
        )
        .await
        .unwrap();

    assert!(html.contains("Native Title"));
    assert!(html.contains("native Rust module"));
}
