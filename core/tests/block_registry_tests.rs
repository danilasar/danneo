use danneo_core::blocks::BlockContext;
use danneo_core::models::core_block_definitions;
use danneo_core::registry::BlockRegistry;
use danneo_core::state::AppState;
use sea_orm::{ActiveModelTrait, Set};
use std::sync::Arc;

mod common;

async fn test_context() -> (
    Arc<sea_orm::DatabaseConnection>,
    Arc<BlockContext>,
    tera::Tera,
    Arc<AppState>,
) {
    let state = common::create_test_state().await;
    let db = state.db.clone();
    let ctx = Arc::new(BlockContext {
        db: db.clone(),
        settings: state.settings.clone(),
        state: state.clone(),
    });

    (db, ctx, tera::Tera::default(), state)
}

#[tokio::test]
async fn lua_block_can_return_html() {
    let (db, ctx, tera, state) = test_context().await;
    let script_engine = state.script_engine.clone();
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
    let (db, ctx, mut tera, state) = test_context().await;
    let script_engine = state.script_engine.clone();
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

    assert_eq!(html, "<section>Danneo:Lua block:3</section>");
}

#[tokio::test]
async fn native_module_block_renders_through_same_registry() {
    let (db, ctx, tera, state) = test_context().await;
    let script_engine = state.script_engine.clone();
    let registry = BlockRegistry::new(db.clone(), script_engine);

    let native_modules = state.modules.get_native_modules().await;
    registry.init(native_modules).await;

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
