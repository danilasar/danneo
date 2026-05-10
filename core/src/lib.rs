#[macro_use]
extern crate rust_i18n;

pub use inventory;

pub mod acl;
pub mod apanel;
pub mod auth;
pub mod blocks;
pub mod crud;
pub mod frontend;
pub mod models;
pub mod module;
pub mod registry;
pub mod rpc;
pub mod state;
pub mod utils;

rust_i18n::i18n!("locales");

use crate::module::DanneoModule;
use axum::{
    Router,
    extract::State,
    routing::{get, post},
};
use sea_orm::Database;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let _ = tracing_subscriber::fmt::try_init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Database::connect(db_url).await?;

    let app_state = Arc::new(
        state::AppState::new(db)
            .await
            .map_err(|e| format!("Failed to initialize AppState: {}", e))?,
    );

    // 1. Настройка роутера админки
    let mut admin_routes = Router::<Arc<state::AppState>>::new()
        .route("/dashboard", get(apanel::dashboard::render_dashboard))
        .route("/modules", get(apanel::modules::list_modules))
        .route("/modules/upload", post(apanel::modules::upload_module))
        .route(
            "/modules/install_from_staging",
            post(apanel::modules::install_from_staging_handle),
        )
        .route("/modules/install", post(apanel::modules::install_module))
        .route(
            "/modules/uninstall",
            post(apanel::modules::uninstall_module),
        )
        .route("/modules/enable", post(apanel::modules::enable_module))
        .route("/modules/disable", post(apanel::modules::disable_module))
        .route(
            "/m/:module/*path",
            get(apanel::modules::dispatch_admin).post(apanel::modules::dispatch_admin),
        );

    // Mount Native Modules Routers statically
    admin_routes = admin_routes
        .nest("/settings", crate::module::settings::SettingsModule::new(app_state.db.clone()).register_admin_routes())
        .nest("/seo", crate::module::seo::SeoModule.register_admin_routes())
        .nest("/design", crate::module::design::DesignModule.register_admin_routes())
        .nest("/blocks", crate::module::blocks::BlocksModule.register_admin_routes())
        .nest("/security", crate::module::security::SecurityModule.register_admin_routes())
        .nest("/menu_system", crate::module::admin_menu::AdminMenuModule::new(app_state.db.clone()).register_admin_routes());

    admin_routes = admin_routes
        .route("/crud/:module/:entity/list", get(apanel::crud::list_page))
        .route("/crud/:module/:entity/edit", get(apanel::crud::edit_page))
        .route(
            "/crud/:module/:entity/edit/:id",
            get(apanel::crud::edit_page),
        )
        .route(
            "/crud/:module/:entity/save",
            post(apanel::crud::save_handle),
        )
        .route(
            "/crud/:module/:entity/delete/:id",
            get(apanel::crud::delete_handle),
        )
        .route("/crud/:module/:entity/:action", get(apanel::crud::handle))
        // Dynamic cleanup dispatcher as fallback for clean URLs (like /admin/menu from scripts)
        .fallback(apanel::modules::dispatch_admin_clean)
        // Order matters: first check if module is enabled, then check ACL
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            apanel::middleware::admin_acl_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            apanel::middleware::module_enabled_middleware,
        ));

    let packages_dir = if std::path::Path::new("modules").exists() {
        "modules"
    } else {
        "core/modules"
    };

    let static_dir = if std::path::Path::new("core/static").exists() {
        "core/static"
    } else {
        "./static"
    };

    // 2. Основной роутер приложения
    let app = Router::<Arc<state::AppState>>::new()
        .route("/", get(root))
        .route("/admin/login", get(auth::show_login_page))
        .route("/api/admin/login", post(auth::admin_login))
        .nest("/admin", admin_routes)
        .nest_service(
            "/static/m",
            tower_http::services::ServeDir::new(packages_dir),
        )
        .nest_service("/static", tower_http::services::ServeDir::new(static_dir))
        .fallback(frontend::dispatch)
        .with_state(app_state);

    // 3. Запуск сервера
    let addr_str = std::env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let addr: SocketAddr = addr_str.parse()?;

    info!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root(State(state): State<Arc<state::AppState>>) -> impl axum::response::IntoResponse {
    let mut context = tera::Context::new();
    frontend::prepare_frontend_context(state.clone(), &mut context).await;

    let settings = state.settings.read().await;
    let theme = settings.site_temp.clone();

    let welcome_html = format!(r#"
        <style>
            .welcome-container {{ padding: 40px 20px; text-align: center; }}
            .welcome-container h1 {{ font-size: 2.5em; color: #1a73e8; margin-bottom: 20px; }}
            .welcome-container p {{ font-size: 1.2em; color: #5f6368; max-width: 800px; margin: 0 auto 30px; line-height: 1.6; }}
            .welcome-btn {{ display: inline-block; padding: 15px 30px; background: #1a73e8; color: white; text-decoration: none; border-radius: 5px; font-weight: bold; }}
        </style>
        <div class="welcome-container">
            <h1>Danneo 2.0</h1>
            <p>Добро пожаловать на ваш новый сайт <strong>{}</strong>, работающий на сверхбыстром движке Rust.</p>
            <p>Это модернизированная версия легендарной Danneo CMS. Ваша система полностью настроена и готова к работе.</p>
            <a href="/admin/dashboard" class="welcome-btn">Перейти в панель управления</a>
        </div>
    "#, settings.site_name);

    context.insert("welcome_text", &welcome_html);

    let template_name = format!("frontend/{}/index.html", theme);
    match state.tera.render(&template_name, &context) {
        Ok(html) => axum::response::Html(html),
        Err(e) => {
            tracing::error!("Template error ({}): {}", template_name, e);
            // Fallback to minimal
            axum::response::Html(format!("<h1>System Error</h1><p>Theme template <b>{}</b> not found or invalid.</p><pre>{}</pre>", template_name, e))
        }
    }
}
