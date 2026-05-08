pub mod acl;
pub mod apanel;
pub mod auth;
pub mod blocks;
pub mod crud;
pub mod frontend;
pub mod models;
pub mod module;
pub mod registry;
pub mod state;
pub mod utils;

use crate::module::DanneoModule;
rust_i18n::i18n!("locales");

pub fn init_i18n() {
    rust_i18n::set_locale("ru");
}

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
    // Загрузка переменных окружения
    dotenvy::dotenv().ok();

    // Инициализация логгирования
    // tracing_subscriber::fmt::init(); // Переместим это в main.rs

    info!("Starting Danneo 2 Core...");

    // Подключение к БД
    let db_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?;
    let db = Database::connect(&db_url).await?;

    // Запуск миграций
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await?;
    info!("Database migrations completed.");

    // Инициализация глобального состояния
    let app_state = Arc::new(
        state::AppState::new(db)
            .await
            .map_err(|e| format!("Failed to initialize AppState: {}", e))?,
    );

    // Настройка роутера
    let admin_routes = Router::new()
        .route("/dashboard", get(apanel::dashboard::render_dashboard))
        .route("/settings", get(apanel::settings::show_settings))
        .route("/settings/save", post(apanel::settings::save_settings))
        .route("/seo", get(apanel::seo::show_settings))
        .route("/seo/save", post(apanel::seo::save_settings))
        .route("/seo/sitemap", get(apanel::seo::show_sitemap))
        .route("/seo/sitemap/save", post(apanel::seo::save_sitemap))
        .route("/seo/social", get(apanel::seo::show_social))
        .route("/seo/social/save", post(apanel::seo::save_social))
        .route("/seo/social/delete", post(apanel::seo::delete_social))
        .route("/design", get(apanel::design::show_design))
        .route("/design/save", post(apanel::design::save_file))
        .route("/blocks/positions", get(apanel::blocks::list_positions))
        .route(
            "/blocks/positions/save",
            post(apanel::blocks::save_position),
        )
        .route(
            "/blocks/positions/delete",
            post(apanel::blocks::delete_position),
        )
        .route("/blocks", get(apanel::blocks::list_blocks))
        .route("/blocks/edit", get(apanel::blocks::edit_block))
        .route("/blocks/save", post(apanel::blocks::save_block))
        .route("/blocks/delete", post(apanel::blocks::delete_block))
        .route("/menu", get(apanel::menu::list_groups))
        .route("/menu/group/save", post(apanel::menu::save_group))
        .route("/menu/group/delete", get(apanel::menu::delete_group))
        .route("/menu/items", get(apanel::menu::list_items))
        .route("/menu/item/save", post(apanel::menu::save_item))
        .route("/menu/item/delete", get(apanel::menu::delete_item))
        .route("/amanage", get(apanel::amanage::list_admins))
        .route("/amanage/edit", get(apanel::amanage::edit_admin))
        .route("/amanage/save", post(apanel::amanage::save_admin))
        .route("/amanage/delete", post(apanel::amanage::delete_admin))
        .route("/agroups", get(apanel::agroups::list_groups))
        .route("/agroups/edit/:id", get(apanel::agroups::edit_group))
        .route("/agroups/save", post(apanel::agroups::save_group))
        .route("/agroups/delete/:id", post(apanel::agroups::delete_group))
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
        .route(
            "/m/:module/*path",
            get(apanel::modules::dispatch_admin).post(apanel::modules::dispatch_admin),
        )
        .nest(
            "/native-demo",
            crate::module::native_demo::NativeDemoModule.register_admin_routes(),
        )
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
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            apanel::middleware::admin_acl_middleware,
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

    let app = Router::new()
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

    // Запуск сервера
    let addr_str = std::env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let addr: SocketAddr = addr_str.parse()?;

    info!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root(State(state): State<Arc<state::AppState>>) -> impl axum::response::IntoResponse {
    let mut context = tera::Context::new();
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);
    let seo = utils::seo::SeoMeta::new(&settings.site_name)
        .with_description(&settings.site_name)
        .with_breadcrumb(&settings.site_name, "/");
    seo.insert_into_context(&mut context);

    // Предварительный рендеринг блоков
    let ctx = Arc::new(crate::blocks::BlockContext {
        db: state.db.clone(),
        settings: state.settings.clone(),
    });

    let positions = state
        .block_registry
        .get_all_positions_html(ctx, &state.tera)
        .await;
    context.insert("positions", &positions);

    // Рендерим меню
    let top_menu = crate::blocks::menu::render_menu(state.db.as_ref(), "top_menu").await;
    let bot_menu = crate::blocks::menu::render_menu(state.db.as_ref(), "bot_menu").await;
    context.insert("top_menu", &top_menu);
    context.insert("bot_menu", &bot_menu);

    let template_name = format!("frontend/{}/index.html", settings.site_temp);

    match state.tera.render(&template_name, &context) {
        Ok(html) => axum::response::Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            axum::response::Html(format!("<h1>Template Error</h1><pre>{}</pre>", e))
        }
    }
}
