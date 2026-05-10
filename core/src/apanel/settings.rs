use crate::{auth::Claims, models::core_settings, state::AppState};
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use sea_orm::{EntityTrait, Set};
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;

#[derive(Deserialize)]
pub struct SettingsForm {
    pub site_name: String,
    pub admin_email: String,
    pub site_url: String,
    pub site_temp: String,
}

pub async fn show_settings(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut context = Context::new();
    let settings = state.settings.read().await;
    context.insert("admin_email", &settings.admin_email);
    context.insert("site_url", &settings.site_url);
    context.insert("site_temp", &settings.site_temp);

    // Список доступных тем (в будущем сканировать папку)
    let themes = vec!["Soft", "Old", "Clear"];
    context.insert("themes", &themes);

    crate::apanel::render_admin_template(state.clone(), "settings/default/apanel/settings.html", context).await
}

pub async fn save_settings(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Form(form): Form<SettingsForm>,
) -> impl IntoResponse {
    let db = state.db.as_ref();

    // Обновляем в БД
    let updates = vec![
        ("site_name", &form.site_name),
        ("admin_email", &form.admin_email),
        ("site_url", &form.site_url),
        ("site_temp", &form.site_temp),
    ];

    for (key, value) in updates {
        let active_model = core_settings::ActiveModel {
            key: Set(key.to_string()),
            value: Set(serde_json::json!(value)),
        };

        if let Err(e) = core_settings::Entity::insert(active_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(core_settings::Column::Key)
                    .update_column(core_settings::Column::Value)
                    .to_owned(),
            )
            .exec(db)
            .await
        {
            tracing::error!("Failed to save setting {}: {:?}", key, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)).into_response();
        }
    }

    // Обновляем в кэше AppState
    {
        let mut settings = state.settings.write().await;
        settings.site_name = form.site_name;
        settings.admin_email = form.admin_email;
        settings.site_url = form.site_url;
        settings.site_temp = form.site_temp;
    }

    Redirect::to("/admin/settings").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use sea_orm::Database;
    use tower::ServiceExt;

    use crate::auth::AuthService;

    async fn setup_test_db() -> sea_orm::DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        use sea_orm_migration::MigratorTrait;
        migration::Migrator::up(&db, None).await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_show_settings_page() {
        let db = setup_test_db().await;

        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();

        let app = axum::Router::new()
            .route("/admin/settings", axum::routing::get(show_settings))
            .with_state(state);

        // Создаем токен для теста
        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service
            .create_token(1, 9999999999, 1000000000)
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/admin/settings")
                    .header("Cookie", format!("danneo_token={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        if response.status() != StatusCode::OK {
            let status = response.status();
            let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
            panic!("Expected 200, got {}. Body: {}", status, String::from_utf8_lossy(&body));
        }
    }

    #[tokio::test]
    async fn test_save_settings_redirect() {
        let db = setup_test_db().await;

        let state = Arc::new(AppState::new(db).await.unwrap());
        let jwt_secret = state.jwt_secret.clone();

        let app = axum::Router::new()
            .route("/admin/settings/save", axum::routing::post(save_settings))
            .with_state(state);

        let auth_service = AuthService::new(jwt_secret);
        let token = auth_service
            .create_token(1, 9999999999, 1000000000)
            .unwrap();

        let form_data =
            "site_name=New+Name&admin_email=new@test.com&site_url=http://new.com&site_temp=Old";

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/settings/save")
                    .header("Cookie", format!("danneo_token={}", token))
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from(form_data))
                    .unwrap(),
            )
            .await
            .unwrap();

        if response.status() != StatusCode::SEE_OTHER {
            let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
            println!("Response body: {}", String::from_utf8_lossy(&body));
            panic!("Expected 303, got {}", StatusCode::INTERNAL_SERVER_ERROR);
        }
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/admin/settings"
        );
    }
}
