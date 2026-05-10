use danneo_core::state::AppState;
use sea_orm::Database;
use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn test_module_availability_api_rust() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();
    let state = Arc::new(AppState::new(db).await.unwrap());

    // 1. Check existing core module
    assert!(state.is_module_available("admin_menu").await, "admin_menu should be available by default");

    // 2. Check non-existent
    assert!(!state.is_module_available("ghost_module").await, "ghost_module should not be available");

    // 3. Check disabled module
    use sea_orm::{ActiveModelTrait, Set};
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    danneo_core::models::core_modules::ActiveModel {
        code: Set("test_mod".to_string()),
        name: Set("Test".to_string()),
        version: Set("1.0.0".to_string()),
        package_id: Set(1),
        package_path: Set("path".to_string()),
        package_hash: Set("hash".to_string()),
        runtime_type: Set("lua".to_string()),
        enabled: Set(false),
        installed: Set(true),
        manifest: Set(serde_json::json!({})),
        installed_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }.insert(state.db.as_ref()).await.unwrap();

    assert!(!state.is_module_available("test_mod").await, "Disabled module should not be available via API");
}

#[tokio::test]
async fn test_module_availability_api_lua() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();
    let state = Arc::new(AppState::new(db).await.unwrap());

    // Define a script that uses system.is_available
    let script = r#"
        function check(arg)
            return {
                admin_menu = system.is_available("admin_menu"),
                fake = system.is_available("fake")
            }
        end
    "#;
    state.script_engine.load_script_str("test_sys", script).await.unwrap();

    let res = state.script_engine.call_hook("test_sys", "check", script_rhai::Dynamic::UNIT, state.clone()).await.unwrap();
    let res_map: serde_json::Value = script_rhai::serde::from_dynamic(&res).unwrap();

    assert_eq!(res_map["admin_menu"], true);
    assert_eq!(res_map["fake"], false);
}

#[tokio::test]
async fn test_bootstrap_idempotency() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();
    
    // 1. Initial boot (AppState::new calls bootstrap internally)
    let state = Arc::new(AppState::new(db).await.unwrap());
    
    // Check if flag is set
    use sea_orm::{ConnectionTrait, Statement};
    let row = state.db.query_one(Statement::from_string(state.db.get_database_backend(), 
        "SELECT value FROM core_system_state WHERE key = 'is_bootstrapped'")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<String>("", "value").unwrap(), "true");

    // 2. Simulate manual uninstallation of a core module
    state.db.execute(Statement::from_string(state.db.get_database_backend(), 
        "DELETE FROM core_modules WHERE code = 'settings'")).await.unwrap();
    
    assert!(!state.is_module_available("settings").await);

    // 3. Second bootstrap attempt (manual)
    let installer = danneo_core::registry::PackageInstaller::new(
        state.db.clone(),
        state.packages.clone(),
        state.modules.clone(),
        state.routes.clone(),
        state.script_engine.clone(),
        state.clone(),
    );
    
    installer.bootstrap().await.unwrap();

    // 4. Verify that 'settings' did NOT reappear because flag exists
    assert!(!state.is_module_available("settings").await, "Uninstalled module should not be restored if system is already bootstrapped");
}

#[tokio::test]
async fn test_middleware_blocks_disabled_module() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.unwrap();
    let state = Arc::new(AppState::new(db).await.unwrap());

    // Create an admin router with middleware
    let admin_routes = axum::Router::new()
        .route("/test_mod/foo", axum::routing::get(|| async { "OK" }))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            danneo_core::apanel::middleware::module_enabled_middleware,
        ));

    // 1. Request to module not in DB -> should be 404
    let response = admin_routes.clone()
        .oneshot(Request::builder().uri("/test_mod/foo").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // 2. Enable module in DB
    use sea_orm::{ActiveModelTrait, Set};
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    danneo_core::models::core_modules::ActiveModel {
        code: Set("test_mod".to_string()),
        name: Set("Test".to_string()),
        version: Set("1.0.0".to_string()),
        package_id: Set(1),
        package_path: Set("path".to_string()),
        package_hash: Set("hash".to_string()),
        runtime_type: Set("lua".to_string()),
        enabled: Set(true),
        installed: Set(true),
        manifest: Set(serde_json::json!({})),
        installed_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }.insert(state.db.as_ref()).await.unwrap();

    // 3. Request again -> should be 200
    let response = admin_routes
        .oneshot(Request::builder().uri("/test_mod/foo").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
