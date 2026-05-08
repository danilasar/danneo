pub mod auth;
pub mod module;
pub mod state;
pub mod models;
pub mod apanel;
pub mod blocks;

use axum::{
    routing::{get, post},
    extract::State,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use sea_orm::Database;

pub async fn run() {
    // Загрузка переменных окружения
    dotenvy::dotenv().ok();
    
    // Инициализация логгирования
    // tracing_subscriber::fmt::init(); // Переместим это в main.rs

    info!("Starting Danneo 2 Core...");

    // Подключение к БД
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Database::connect(&db_url).await.expect("Failed to connect to database");
    
    // Запуск миграций
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.expect("Failed to run migrations");
    info!("Database migrations completed.");

    // Инициализация глобального состояния
    let app_state = Arc::new(state::AppState::new(db).await.expect("Failed to initialize AppState"));

    // Настройка роутера
    let app = Router::new()
        .route("/", get(root))
        .route("/admin/login", get(auth::show_login_page))
        .route("/api/admin/login", post(auth::admin_login))
        .route("/admin/dashboard", get(apanel::dashboard::render_dashboard))
        .route("/admin/settings", get(apanel::settings::show_settings))
        .route("/admin/settings/save", post(apanel::settings::save_settings))
        .route("/admin/design", get(apanel::design::show_design))
        .route("/admin/design/save", post(apanel::design::save_file))
        .route("/admin/blocks/positions", get(apanel::blocks::list_positions))
        .route("/admin/blocks/positions/save", post(apanel::blocks::save_position))
        .route("/admin/blocks/positions/delete", get(apanel::blocks::delete_position))
        .route("/admin/blocks", get(apanel::blocks::list_blocks))
        .route("/admin/blocks/edit", get(apanel::blocks::edit_block))
        .route("/admin/blocks/save", post(apanel::blocks::save_block))
        .route("/admin/blocks/delete", get(apanel::blocks::delete_block))
        .route("/admin/menu", get(apanel::menu::list_groups))
        .route("/admin/menu/group/save", post(apanel::menu::save_group))
        .route("/admin/menu/group/delete", get(apanel::menu::delete_group))
        .route("/admin/menu/items", get(apanel::menu::list_items))
        .route("/admin/menu/item/save", post(apanel::menu::save_item))
        .route("/admin/menu/item/delete", get(apanel::menu::delete_item))
        .route("/admin/amanage", get(apanel::amanage::list_admins))
        .route("/admin/amanage/edit", get(apanel::amanage::edit_admin))
        .route("/admin/amanage/save", post(apanel::amanage::save_admin))
        .route("/admin/amanage/delete", get(apanel::amanage::delete_admin))
        .nest_service("/static", tower_http::services::ServeDir::new("core/static"))
        .with_state(app_state);

    // Запуск сервера
    let addr_str = std::env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let addr: SocketAddr = addr_str.parse().expect("Invalid SERVER_ADDR");
    
    info!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root(State(state): State<Arc<state::AppState>>) -> impl axum::response::IntoResponse {
    let mut context = tera::Context::new();
    let settings = state.settings.read().await;
    context.insert("site_name", &settings.site_name);
    
    // Предварительный рендеринг блоков
    let ctx = Arc::new(crate::blocks::BlockContext {
        db: state.db.clone(),
        settings: state.settings.clone(),
    });
    
    let positions = state.block_manager.get_all_positions_html(ctx).await;
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
