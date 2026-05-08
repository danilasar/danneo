pub mod auth;
pub mod module;
pub mod state;
pub mod models;
pub mod apanel;

use axum::{
    routing::{get, post},
    extract::State,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use sea_orm::Database;

#[tokio::main]
async fn main() {
    // Загрузка переменных окружения
    dotenvy::dotenv().ok();
    
    // Инициализация логгирования
    tracing_subscriber::fmt::init();

    info!("Starting Danneo 2 Core...");

    // Подключение к БД
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Database::connect(&db_url).await.expect("Failed to connect to database");
    
    // Запуск миграций
    use sea_orm_migration::MigratorTrait;
    migration::Migrator::up(&db, None).await.expect("Failed to run migrations");
    info!("Database migrations completed.");

    // Проверка наличия администраторов (сид данных)
    use sea_orm::{EntityTrait, PaginatorTrait};
    use crate::models::core_admins;
    let admin_count = core_admins::Entity::find()
        .count(&db)
        .await
        .unwrap_or(0);

    if admin_count == 0 {
        use sea_orm::{ActiveModelTrait, Set};
        let password_hash = bcrypt::hash("password", 4).unwrap();
        let default_admin = core_admins::ActiveModel {
            login: Set("admin".to_string()),
            password_hash: Set(password_hash),
            ..Default::default()
        };
        default_admin.insert(&db).await.expect("Failed to create default admin");
        info!("Default admin created: admin / password");
    }

    // Инициализация глобального состояния
    let app_state = Arc::new(state::AppState::new(db).await.expect("Failed to initialize AppState"));

    // Настройка роутера
    let app = Router::new()
        .route("/", get(root))
        .route("/admin/login", get(auth::show_login_page))
        .route("/api/admin/login", post(auth::admin_login))
        .route("/admin/dashboard", get(apanel::dashboard::render_dashboard))
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
    context.insert("site_name", &state.settings.site_name);
    
    match state.tera.render("index.html", &context) {
        Ok(html) => axum::response::Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            axum::response::Html("<h1>Internal Server Error</h1>".to_string())
        }
    }
}
