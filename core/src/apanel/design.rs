use crate::{auth::Claims, state::AppState};
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;

#[derive(Deserialize)]
pub struct SaveFileForm {
    pub file_name: String,
    pub content: String,
}

pub async fn show_design(_claims: Claims, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let settings = state.settings.read().await;
    let theme = &settings.site_temp;

    let mut context = Context::new();
    context.insert("site_temp", theme);
    context.insert("site_name", &settings.site_name);

    // Читаем index.html
    let index_path = format!("core/templates/frontend/{}/index.html", theme);
    let index_content = std::fs::read_to_string(&index_path)
        .unwrap_or_else(|_| "<!-- Шаблон не найден -->".to_string());
    let index_json = serde_json::to_string(&index_content)
        .unwrap()
        .replace("</script>", "<\\/script>");
    context.insert("index_html_content_json", &index_json);
    context.insert("initial_content", &index_content);

    // Читаем screen.css
    let css_path = format!("core/static/frontend/{}/css/screen.css", theme);
    let css_content =
        std::fs::read_to_string(&css_path).unwrap_or_else(|_| "/* CSS не найден */".to_string());
    context.insert(
        "screen_css_content_json",
        &serde_json::to_string(&css_content).unwrap(),
    );

    match state.tera.render("apanel/design.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Tera error: {:?}", e);
            tracing::error!("Template rendering error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn save_file(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<SaveFileForm>,
) -> impl IntoResponse {
    // 1. Защита от Path Traversal и ограничение списка файлов
    if form.file_name.contains("..") || form.file_name.starts_with('/') {
        return (StatusCode::BAD_REQUEST, "Invalid file name").into_response();
    }

    if form.file_name != "index.html" && form.file_name != "css/screen.css" {
        return (StatusCode::BAD_REQUEST, "Editing this file is not allowed").into_response();
    }

    let settings = state.settings.read().await;
    let theme = &settings.site_temp;

    // 2. Определяем базовый путь
    let base_path = if form.file_name.ends_with(".html") {
        format!("core/templates/frontend/{}", theme)
    } else {
        format!("core/static/frontend/{}", theme)
    };

    let full_path = std::path::Path::new(&base_path).join(&form.file_name);

    // 3. Сохраняем
    if let Err(e) = std::fs::write(full_path, &form.content) {
        tracing::error!("Failed to save file: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Redirect::to("/admin/design").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthService;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use sea_orm::Database;
    use tower::ServiceExt;

    async fn setup_test_db() -> sea_orm::DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        use sea_orm_migration::MigratorTrait;
        migration::Migrator::up(&db, None).await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_show_design_page() {
        let db = setup_test_db().await;
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();

        let app = axum::Router::new()
            .route("/admin/design", axum::routing::get(show_design))
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/admin/design")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_save_file_path_traversal_protection() {
        let db = setup_test_db().await;
        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();

        let app = axum::Router::new()
            .route("/admin/design/save", axum::routing::post(save_file))
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service.create_token(1, 9999999999).unwrap();

        let form_data = "file_name=../../etc/passwd&content=hacking";

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/design/save")
                    .header("Cookie", format!("danneo_token={}", token))
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from(form_data))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Должно вернуть ошибку (400 или 403), а не сохранить
        assert_ne!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
